use super::{
    RigRevisionRefs,
    bone_tree::{BoneId, BoneNode, BoneTree, RestTransform},
    constraints::{ConstraintCapability, RigConstraint, validate_constraints},
    mesh::{Mesh, Triangle, Vertex},
    pivots_sockets::{LocalPoint, Pivot, Socket, SocketKind, validate_pivots_and_sockets},
    slots::{Slot, SlotSet},
    weights::{BoneWeight, WeightSet},
};
use crate::{
    canonical::canonical_sha256,
    layers::{LayerRole, LayerSet},
    master::{PrimaryWeaponSpec, WeaponHand},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RigApprovalState {
    Pending,
    Approved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigCanvas {
    pub width_px: u32,
    pub height_px: u32,
}

impl RigCanvas {
    pub fn validate(self) -> Result<(), String> {
        if self.width_px == 0
            || self.height_px == 0
            || self.width_px > 32_768
            || self.height_px > 32_768
        {
            return Err("Rig canvas must be between 1 and 32768 pixels per axis".into());
        }
        Ok(())
    }

    pub fn contains_local_point(self, point: LocalPoint) -> bool {
        let max_x = i64::from(self.width_px) * 1_000;
        let max_y = i64::from(self.height_px) * 1_000;
        (-max_x..=max_x).contains(&point.x_milli_px) && (-max_y..=max_y).contains(&point.y_milli_px)
    }
}

/// The complete, versioned aggregate reviewed at the Rig approval gate.
///
/// Aggregate revisions are explicit instead of inferred from collection sizes. This lets the
/// application reject stale editor commands and makes every approval payload replayable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigCandidate {
    pub rig_id: String,
    pub revision: u64,
    pub layer_set_id: String,
    pub layer_set_revision: u64,
    pub layer_set_approval_sha256: String,
    pub canvas: RigCanvas,
    pub primary_weapon: PrimaryWeaponSpec,
    pub bone_tree: BoneTree,
    pub slot_set: SlotSet,
    pub pivot_socket_revision: u64,
    pub pivots: Vec<Pivot>,
    pub sockets: Vec<Socket>,
    pub mesh_revision: u64,
    pub meshes: Vec<Mesh>,
    pub weight_revision: u64,
    pub weights: Vec<WeightSet>,
    pub constraint_revision: u64,
    pub constraints: Vec<RigConstraint>,
    pub constraint_capability: ConstraintCapability,
    pub approval_state: RigApprovalState,
}

impl RigCandidate {
    pub fn revision_refs(&self) -> RigRevisionRefs {
        RigRevisionRefs {
            layer_set_revision: self.layer_set_revision,
            bone_tree_revision: self.bone_tree.revision,
            slot_revision: self.slot_set.revision,
            pivot_socket_revision: self.pivot_socket_revision,
            mesh_revision: self.mesh_revision,
            weight_revision: self.weight_revision,
            constraint_revision: self.constraint_revision,
        }
    }

    /// Validates both the aggregate internals and its immutable dependency on an approved layer
    /// set. The expected layer approval hash is recomputed with the same normalization as the
    /// layer approval gate, so a caller cannot bind this Rig to an unrelated approval string.
    pub fn validate(&self, layer_set: &LayerSet) -> Result<(), String> {
        if self.rig_id.trim().is_empty() || self.revision == 0 {
            return Err("Rig id and a positive aggregate revision are required".into());
        }
        self.canvas.validate()?;
        self.primary_weapon.validate()?;
        layer_set.validate()?;
        layer_set.validate_required_v1_roles()?;
        if layer_set.approval_state != "APPROVED"
            || layer_set.layers.iter().any(|layer| !layer.approved)
        {
            return Err("Rig requires a fully approved LayerSet".into());
        }
        if self.layer_set_id != layer_set.layer_set_id
            || self.layer_set_revision != layer_set.revision
        {
            return Err("Rig is bound to a different LayerSet revision".into());
        }
        let layer_hash = layer_set_approval_payload_sha256(layer_set)?;
        if !is_lower_sha256(&self.layer_set_approval_sha256)
            || self.layer_set_approval_sha256 != layer_hash
        {
            return Err("Rig LayerSet approval hash mismatch".into());
        }
        for revision in [
            self.bone_tree.revision,
            self.slot_set.revision,
            self.pivot_socket_revision,
            self.mesh_revision,
            self.weight_revision,
            self.constraint_revision,
        ] {
            if revision == 0 {
                return Err("every Rig component requires a positive revision".into());
            }
        }

        self.bone_tree.validate()?;
        let layer_ids: Vec<String> = layer_set
            .layers
            .iter()
            .map(|layer| layer.layer_id.clone())
            .collect();
        let required_layers: BTreeSet<_> = layer_ids.iter().map(String::as_str).collect();
        self.slot_set.validate(&layer_ids, &self.bone_tree)?;
        let slot_layers: BTreeSet<_> = self
            .slot_set
            .slots
            .iter()
            .map(|slot| slot.layer_id.as_str())
            .collect();
        if self.slot_set.slots.len() != layer_set.layers.len() || slot_layers != required_layers {
            return Err("slots must cover every LayerSet layer exactly once".into());
        }

        validate_pivots_and_sockets(
            &self.pivots,
            &self.sockets,
            &self.bone_tree,
            Some(&self.primary_weapon.socket_semantic),
        )?;
        let pivot_layers: BTreeSet<_> = self
            .pivots
            .iter()
            .map(|pivot| pivot.layer_id.as_str())
            .collect();
        if self.pivots.len() != layer_set.layers.len() || pivot_layers != required_layers {
            return Err("pivots must cover every LayerSet layer exactly once".into());
        }
        if self
            .pivots
            .iter()
            .any(|pivot| !self.canvas.contains_local_point(pivot.point))
            || self
                .sockets
                .iter()
                .any(|socket| !self.canvas.contains_local_point(socket.point))
        {
            return Err("pivot or socket lies outside the supported Rig canvas".into());
        }
        let weapon_socket = self
            .sockets
            .iter()
            .find(|socket| socket.kind == SocketKind::PrimaryWeapon)
            .ok_or("primary weapon socket missing")?;
        if weapon_socket.semantic != self.primary_weapon.socket_semantic {
            return Err("primary weapon socket semantic differs from StyleSpec".into());
        }

        let mut meshes_by_id = BTreeMap::new();
        let mut mesh_layers = BTreeSet::new();
        for mesh in &self.meshes {
            mesh.validate()?;
            if !required_layers.contains(mesh.layer_id.as_str()) {
                return Err(format!("mesh references unknown layer: {}", mesh.layer_id));
            }
            if meshes_by_id.insert(mesh.mesh_id.as_str(), mesh).is_some()
                || !mesh_layers.insert(mesh.layer_id.as_str())
            {
                return Err("duplicate mesh id or more than one mesh for a layer".into());
            }
        }
        if self.meshes.len() != layer_set.layers.len() || mesh_layers != required_layers {
            return Err("meshes must cover every LayerSet layer exactly once".into());
        }
        let mut weighted_meshes = BTreeSet::new();
        for weights in &self.weights {
            if !weighted_meshes.insert(weights.mesh_id.as_str()) {
                return Err("duplicate WeightSet for mesh".into());
            }
            let mesh = meshes_by_id
                .get(weights.mesh_id.as_str())
                .ok_or_else(|| format!("weights reference unknown mesh: {}", weights.mesh_id))?;
            weights.validate(mesh, &self.bone_tree)?;
        }
        if weighted_meshes.len() != meshes_by_id.len()
            || meshes_by_id
                .keys()
                .any(|mesh_id| !weighted_meshes.contains(mesh_id))
        {
            return Err("every mesh requires exactly one WeightSet".into());
        }
        validate_constraints(
            &self.constraints,
            &self.bone_tree,
            &self.constraint_capability,
        )?;
        Ok(())
    }

    /// Returns a canonicalizable clone with set-like collections ordered and approval state
    /// normalized. This is the sole aggregate used by the Rig approval payload.
    pub fn normalized_for_approval(&self) -> Self {
        let mut value = self.clone();
        value.approval_state = RigApprovalState::Pending;
        value
            .bone_tree
            .bones
            .sort_by(|a, b| a.bone_id.cmp(&b.bone_id));
        value
            .slot_set
            .slots
            .sort_by(|a, b| a.slot_id.cmp(&b.slot_id));
        value.pivots.sort_by(|a, b| a.layer_id.cmp(&b.layer_id));
        value.sockets.sort_by(|a, b| a.socket_id.cmp(&b.socket_id));
        value.meshes.sort_by(|a, b| a.mesh_id.cmp(&b.mesh_id));
        for mesh in &mut value.meshes {
            mesh.vertices.sort_by_key(|vertex| vertex.vertex_id);
            mesh.triangles.sort_by_key(|triangle| {
                let mut ids = [triangle.0, triangle.1, triangle.2];
                ids.sort();
                ids
            });
        }
        value.weights.sort_by(|a, b| a.mesh_id.cmp(&b.mesh_id));
        for weights in &mut value.weights {
            for influences in weights.by_vertex.values_mut() {
                influences.sort_by(|a, b| a.bone_id.cmp(&b.bone_id));
            }
        }
        value.constraints.sort_by(|a, b| {
            a.order
                .cmp(&b.order)
                .then_with(|| a.constraint_id.cmp(&b.constraint_id))
        });
        value
    }

    pub fn mark_edited(&mut self) -> Result<(), String> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or("Rig revision overflow")?;
        self.approval_state = RigApprovalState::Pending;
        Ok(())
    }
}

pub fn layer_set_approval_payload_sha256(layer_set: &LayerSet) -> Result<String, String> {
    let mut normalized = layer_set.clone();
    normalized.approval_state = "PENDING".into();
    for layer in &mut normalized.layers {
        layer.approved = false;
    }
    canonical_sha256(&normalized).map_err(|error| error.to_string())
}

pub fn rig_approval_payload_sha256(candidate: &RigCandidate) -> Result<String, String> {
    canonical_sha256(&candidate.normalized_for_approval()).map_err(|error| error.to_string())
}

pub fn build_default_side_view_humanoid_rig(
    rig_id: impl Into<String>,
    layer_set: &LayerSet,
    layer_set_approval_sha256: impl Into<String>,
    primary_weapon: PrimaryWeaponSpec,
    canvas: RigCanvas,
    constraint_capability: ConstraintCapability,
) -> Result<RigCandidate, String> {
    let rig_id = rig_id.into();
    canvas.validate()?;
    primary_weapon.validate()?;
    let width_milli = i64::from(canvas.width_px) * 1_000;
    let height_milli = i64::from(canvas.height_px) * 1_000;
    let rest = |x: i64, y: i64| RestTransform {
        x_milli_px: x,
        y_milli_px: y,
        ..RestTransform::default()
    };
    let bones = vec![
        bone(
            "root",
            "Root",
            None,
            rest(width_milli / 2, height_milli * 9 / 10),
        ),
        bone(
            "torso",
            "Torso",
            Some("root"),
            rest(0, -height_milli * 7 / 20),
        ),
        bone("head", "Head", Some("torso"), rest(0, -height_milli / 5)),
        bone(
            "upper-arm-back",
            "Upper Arm Back",
            Some("torso"),
            rest(0, -height_milli / 8),
        ),
        bone(
            "forearm-back",
            "Forearm Back",
            Some("upper-arm-back"),
            rest(width_milli / 10, 0),
        ),
        bone(
            "hand-back",
            "Hand Back",
            Some("forearm-back"),
            rest(width_milli / 10, 0),
        ),
        bone(
            "upper-arm-front",
            "Upper Arm Front",
            Some("torso"),
            rest(0, -height_milli / 8),
        ),
        bone(
            "forearm-front",
            "Forearm Front",
            Some("upper-arm-front"),
            rest(width_milli / 10, 0),
        ),
        bone(
            "hand-front",
            "Hand Front",
            Some("forearm-front"),
            rest(width_milli / 10, 0),
        ),
        bone(
            "thigh-back",
            "Thigh Back",
            Some("root"),
            rest(0, -height_milli / 4),
        ),
        bone(
            "shin-back",
            "Shin Back",
            Some("thigh-back"),
            rest(0, height_milli / 5),
        ),
        bone(
            "foot-back",
            "Foot Back",
            Some("shin-back"),
            rest(0, height_milli / 5),
        ),
        bone(
            "thigh-front",
            "Thigh Front",
            Some("root"),
            rest(0, -height_milli / 4),
        ),
        bone(
            "shin-front",
            "Shin Front",
            Some("thigh-front"),
            rest(0, height_milli / 5),
        ),
        bone(
            "foot-front",
            "Foot Front",
            Some("shin-front"),
            rest(0, height_milli / 5),
        ),
    ];
    let bone_tree = BoneTree { revision: 1, bones };
    let weapon_bone = weapon_bone_id(&primary_weapon).to_owned();
    let mut slots = Vec::with_capacity(layer_set.layers.len());
    let mut pivots = Vec::with_capacity(layer_set.layers.len());
    let mut meshes = Vec::with_capacity(layer_set.layers.len());
    let mut weights = Vec::with_capacity(layer_set.layers.len());
    for (index, layer) in layer_set.layers.iter().enumerate() {
        let bone_id = if layer.role == LayerRole::Weapon {
            weapon_bone.clone()
        } else {
            role_bone_id(layer.role).to_owned()
        };
        let draw_key = i32::try_from(index).map_err(|_| "too many layers for stable draw order")?;
        slots.push(Slot {
            slot_id: format!("slot:{}", layer.layer_id),
            layer_id: layer.layer_id.clone(),
            bone_id: bone_id.clone(),
            draw_key,
        });
        pivots.push(Pivot {
            layer_id: layer.layer_id.clone(),
            point: LocalPoint {
                x_milli_px: width_milli / 2,
                y_milli_px: height_milli / 2,
            },
        });
        let mesh_id = format!("mesh:{}", layer.layer_id);
        let bone_origin = global_bone_translation(&bone_tree, &bone_id)?;
        meshes.push(default_quad_mesh(
            mesh_id.clone(),
            layer.layer_id.clone(),
            width_milli,
            height_milli,
            bone_origin,
        ));
        weights.push(default_rigid_weights(mesh_id, bone_id));
    }
    let candidate = RigCandidate {
        rig_id,
        revision: 1,
        layer_set_id: layer_set.layer_set_id.clone(),
        layer_set_revision: layer_set.revision,
        layer_set_approval_sha256: layer_set_approval_sha256.into(),
        canvas,
        primary_weapon: primary_weapon.clone(),
        bone_tree,
        slot_set: SlotSet { revision: 1, slots },
        pivot_socket_revision: 1,
        pivots,
        sockets: vec![Socket {
            socket_id: "primary-weapon".into(),
            bone_id: weapon_bone,
            kind: SocketKind::PrimaryWeapon,
            point: LocalPoint {
                x_milli_px: 0,
                y_milli_px: 0,
            },
            semantic: primary_weapon.socket_semantic.clone(),
        }],
        mesh_revision: 1,
        meshes,
        weight_revision: 1,
        weights,
        constraint_revision: 1,
        constraints: Vec::new(),
        constraint_capability,
        approval_state: RigApprovalState::Pending,
    };
    candidate.validate(layer_set)?;
    Ok(candidate)
}

fn bone(id: &str, name: &str, parent: Option<&str>, rest: RestTransform) -> BoneNode {
    BoneNode {
        bone_id: id.into(),
        name: name.into(),
        parent_id: parent.map(str::to_owned),
        rest,
    }
}

fn weapon_bone_id(spec: &PrimaryWeaponSpec) -> &'static str {
    match spec.weapon_hand {
        WeaponHand::FarHand => "hand-back",
        WeaponHand::NearHand | WeaponHand::BothHands => "hand-front",
    }
}

fn role_bone_id(role: LayerRole) -> &'static str {
    match role {
        LayerRole::HairBack | LayerRole::Head | LayerRole::HairFront => "head",
        LayerRole::Body | LayerRole::Accessory => "torso",
        LayerRole::UpperArmBack => "upper-arm-back",
        LayerRole::ForearmBack => "forearm-back",
        LayerRole::HandBack => "hand-back",
        LayerRole::UpperArmFront => "upper-arm-front",
        LayerRole::ForearmFront => "forearm-front",
        LayerRole::HandFront => "hand-front",
        LayerRole::ThighBack => "thigh-back",
        LayerRole::ShinBack => "shin-back",
        LayerRole::FootBack => "foot-back",
        LayerRole::ThighFront => "thigh-front",
        LayerRole::ShinFront => "shin-front",
        LayerRole::FootFront => "foot-front",
        LayerRole::Weapon => "hand-front",
    }
}

fn default_quad_mesh(
    mesh_id: String,
    layer_id: String,
    width_milli: i64,
    height_milli: i64,
    bone_origin: (i64, i64),
) -> Mesh {
    let (origin_x, origin_y) = bone_origin;
    Mesh {
        mesh_id,
        layer_id,
        topology_revision: 1,
        vertices: vec![
            Vertex {
                vertex_id: 0,
                x_milli_px: -origin_x,
                y_milli_px: -origin_y,
                u_ppm: 0,
                v_ppm: 0,
            },
            Vertex {
                vertex_id: 1,
                x_milli_px: width_milli - origin_x,
                y_milli_px: -origin_y,
                u_ppm: 1_000_000,
                v_ppm: 0,
            },
            Vertex {
                vertex_id: 2,
                x_milli_px: width_milli - origin_x,
                y_milli_px: height_milli - origin_y,
                u_ppm: 1_000_000,
                v_ppm: 1_000_000,
            },
            Vertex {
                vertex_id: 3,
                x_milli_px: -origin_x,
                y_milli_px: height_milli - origin_y,
                u_ppm: 0,
                v_ppm: 1_000_000,
            },
        ],
        triangles: vec![Triangle(0, 1, 2), Triangle(0, 2, 3)],
    }
}

fn global_bone_translation(tree: &BoneTree, bone_id: &str) -> Result<(i64, i64), String> {
    let mut x = 0i64;
    let mut y = 0i64;
    let mut cursor = Some(bone_id);
    let mut seen = BTreeSet::new();
    while let Some(id) = cursor {
        if !seen.insert(id) {
            return Err("bone cycle while computing attachment-local coordinates".into());
        }
        let bone = tree
            .bones
            .iter()
            .find(|bone| bone.bone_id == id)
            .ok_or("mesh bone missing")?;
        x = x
            .checked_add(bone.rest.x_milli_px)
            .ok_or("bone translation overflow")?;
        y = y
            .checked_add(bone.rest.y_milli_px)
            .ok_or("bone translation overflow")?;
        cursor = bone.parent_id.as_deref();
    }
    Ok((x, y))
}

fn default_rigid_weights(mesh_id: String, bone_id: BoneId) -> WeightSet {
    WeightSet {
        mesh_id,
        topology_revision: 1,
        by_vertex: (0..4)
            .map(|vertex_id| {
                (
                    vertex_id,
                    vec![BoneWeight {
                        bone_id: bone_id.clone(),
                        weight_ppm: 1_000_000,
                    }],
                )
            })
            .collect(),
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
