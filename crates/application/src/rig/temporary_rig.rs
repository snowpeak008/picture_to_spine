use f2s_domain::layers::LayerSet;
use f2s_domain::{
    master::WeaponHand,
    rig::{RigCandidate, RigRevisionRefs},
};
use serde::{Deserialize, Serialize};

pub const RIG_DIAGNOSTIC_ENGINE_ID: &str = "f2s-rig-diagnostics/1.0.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RigIssueSeverity {
    P0,
    P1,
    P2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RigIssue {
    pub code: String,
    pub target: String,
    pub severity: RigIssueSeverity,
    pub fix_target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporaryRigSnapshot {
    pub source_revisions: RigRevisionRefs,
    pub issues: Vec<RigIssue>,
    pub ephemeral: bool,
    pub completed: bool,
    pub engine_id: String,
    pub rig_payload_sha256: Option<String>,
}

impl TemporaryRigSnapshot {
    pub fn new(source_revisions: RigRevisionRefs, issues: Vec<RigIssue>) -> Self {
        Self {
            source_revisions,
            issues,
            ephemeral: true,
            completed: false,
            engine_id: "CALLER_SUPPLIED_UNVERIFIED".into(),
            rig_payload_sha256: None,
        }
    }
    pub fn has_blocking_issues(&self) -> bool {
        self.issues
            .iter()
            .any(|v| matches!(v.severity, RigIssueSeverity::P0 | RigIssueSeverity::P1))
    }

    pub fn is_current_for(&self, candidate: &RigCandidate) -> bool {
        self.completed
            && self.engine_id == RIG_DIAGNOSTIC_ENGINE_ID
            && self.source_revisions == candidate.revision_refs()
            && self.rig_payload_sha256.as_deref()
                == crate::rig::rig_approval_payload(candidate).ok().as_deref()
    }
}

/// Runs the deterministic structural/export diagnostics used by the production
/// Rig gate. An empty issue list is meaningful only on a completed snapshot
/// carrying the exact normalized Rig payload hash.
pub fn diagnose_rig_candidate(
    candidate: &RigCandidate,
    layer_set: &LayerSet,
) -> TemporaryRigSnapshot {
    let mut issues = Vec::new();
    if let Err(error) = candidate.validate(layer_set) {
        issues.push(RigIssue {
            code: "RIG_AGGREGATE_INVALID".into(),
            target: error,
            severity: RigIssueSeverity::P0,
            fix_target: "rig-editor".into(),
        });
    }

    for weights in &candidate.weights {
        for (vertex_id, influences) in &weights.by_vertex {
            if influences.len() != 1 {
                issues.push(RigIssue {
                    code: "MULTI_BONE_WEIGHTS_UNSUPPORTED".into(),
                    target: format!("{}:vertex:{vertex_id}", weights.mesh_id),
                    severity: RigIssueSeverity::P1,
                    fix_target: "weight-editor".into(),
                });
            }
        }
    }

    if let Some(socket) = candidate
        .sockets
        .iter()
        .find(|socket| socket.kind == f2s_domain::rig::pivots_sockets::SocketKind::PrimaryWeapon)
    {
        let expected_hand = match candidate.primary_weapon.weapon_hand {
            WeaponHand::NearHand => socket.bone_id == "hand-front",
            WeaponHand::FarHand => socket.bone_id == "hand-back",
            WeaponHand::BothHands => matches!(socket.bone_id.as_str(), "hand-front" | "hand-back"),
        };
        if !expected_hand {
            issues.push(RigIssue {
                code: "PRIMARY_WEAPON_SOCKET_NOT_ON_DECLARED_HAND".into(),
                target: socket.socket_id.clone(),
                severity: RigIssueSeverity::P1,
                fix_target: "socket-editor".into(),
            });
        }
    }

    let shared_default_pivots = candidate.pivots.first().is_some_and(|first| {
        candidate.pivots.len() > 1
            && candidate
                .pivots
                .iter()
                .all(|pivot| pivot.point == first.point)
    });
    if shared_default_pivots {
        issues.push(RigIssue {
            code: "SHARED_DEFAULT_PIVOTS_REQUIRE_HUMAN_REVIEW".into(),
            target: "all-layer-pivots".into(),
            severity: RigIssueSeverity::P2,
            fix_target: "pivot-editor".into(),
        });
    }

    TemporaryRigSnapshot {
        source_revisions: candidate.revision_refs(),
        issues,
        ephemeral: true,
        completed: true,
        engine_id: RIG_DIAGNOSTIC_ENGINE_ID.into(),
        rig_payload_sha256: crate::rig::rig_approval_payload(candidate).ok(),
    }
}
