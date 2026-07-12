use f2s_adapters::export::cli_policy::{SpineCliPolicy, proprietary_output_allowed};
use f2s_adapters::export::{
    atlas_manifest::atlas_input_bytes,
    package::{AttachmentBytes, commit_open_export},
    psd::{PsdLayer, minimal_psd_bytes, psd_layer_from_png},
    rig_ir::rig_ir_bytes,
    spine42::spine_json_bytes,
};
use f2s_application::export::{
    preflight::preflight,
    publish_snapshot::{ActionApprovalBinding, AttachmentSnapshot, PublishSnapshot},
};
use f2s_domain::{
    ACTION_KEYS, TimeBase,
    animation::{
        clip::{AnimationClip, Curve, Keyframe, Track, TrackChannel},
        markers::{GameplayMarker, GameplayMarkerKind},
    },
    motion::prompt::{PromptEntry, PromptPack},
    rig::{
        SPINE_CAPABILITY_ID, SPINE_PATCH,
        bone_tree::{BoneNode, BoneTree, RestTransform},
        constraints::{ConstraintCapability, ConstraintKind, RigConstraint},
        mesh::{Mesh, Triangle, Vertex},
        pivots_sockets::{LocalPoint, Socket, SocketKind},
        slots::{Slot, SlotSet},
        weights::{BoneWeight, WeightSet},
    },
};
use image::{ImageFormat, ImageReader};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
fn snapshot() -> PublishSnapshot {
    let clips: Vec<AnimationClip> = ACTION_KEYS
        .iter()
        .map(|key| AnimationClip {
            clip_id: format!("clip-{key}"),
            action_key: (*key).into(),
            revision: 1,
            duration_ticks: 30_000,
            time_base: TimeBase::default(),
            tracks: vec![Track {
                track_id: format!("{key}-root-rotate"),
                target_id: "root".into(),
                channel: TrackChannel::BoneRotate,
                keyframes: vec![
                    Keyframe {
                        keyframe_id: format!("{key}-start"),
                        tick: 0,
                        values_milli: vec![0],
                        curve: Curve::Linear,
                        bezier_milli: None,
                    },
                    Keyframe {
                        keyframe_id: format!("{key}-end"),
                        tick: 30_000,
                        values_milli: vec![0],
                        curve: Curve::Linear,
                        bezier_milli: None,
                    },
                ],
            }],
        })
        .collect();
    let action_approvals = clips
        .iter()
        .map(|clip| ActionApprovalBinding {
            action_key: clip.action_key.clone(),
            clip_sha256: f2s_domain::canonical::canonical_sha256(clip).unwrap(),
            pose_approval_sha256: "b".repeat(64),
            hit_approval_sha256: clip
                .action_key
                .starts_with("attack_")
                .then(|| "c".repeat(64)),
        })
        .collect();
    let markers = ["attack_01", "attack_02", "attack_03"]
        .iter()
        .map(|key| GameplayMarker {
            marker_id: format!("hit-{key}"),
            action_key: (*key).into(),
            kind: GameplayMarkerKind::HitFrame,
            start_tick: 18_000,
            end_tick: 18_000,
            socket_id: Some("weapon".into()),
        })
        .collect();
    PublishSnapshot {
        export_id: "export-001".into(),
        rig_id: "rig-001".into(),
        project_revision: 7,
        approved_layer_set_hash: "a".repeat(64),
        approved_rig_hash: "b".repeat(64),
        action_approvals,
        capability_id: SPINE_CAPABILITY_ID.into(),
        spine_patch: SPINE_PATCH.into(),
        primary_weapon: "single-sword".into(),
        time_base: TimeBase::default(),
        bones: BoneTree {
            revision: 1,
            bones: vec![BoneNode {
                bone_id: "root".into(),
                name: "Root".into(),
                parent_id: None,
                rest: RestTransform::default(),
            }],
        },
        slots: SlotSet {
            revision: 1,
            slots: vec![Slot {
                slot_id: "body-slot".into(),
                layer_id: "body".into(),
                bone_id: "root".into(),
                draw_key: 0,
            }],
        },
        pivots: vec![],
        sockets: vec![Socket {
            socket_id: "weapon".into(),
            bone_id: "root".into(),
            kind: SocketKind::PrimaryWeapon,
            point: LocalPoint {
                x_milli_px: 0,
                y_milli_px: 0,
            },
            semantic: "single-sword".into(),
        }],
        meshes: vec![],
        weights: vec![],
        constraints: vec![],
        constraint_capability: ConstraintCapability::from_verified_manifest(
            include_bytes!("../../../fixtures/m00/spine42-probe/capability-manifest.json"),
            &[
                (
                    "rig-ir.json",
                    include_bytes!("../../../fixtures/m00/spine42-probe/rig-ir.json"),
                ),
                (
                    "skeleton.json",
                    include_bytes!("../../../fixtures/m00/spine42-probe/skeleton.json"),
                ),
            ],
        )
        .unwrap(),
        attachments: vec![AttachmentSnapshot {
            attachment_id: "body".into(),
            slot_id: "body-slot".into(),
            logical_png_path: "images/body.png".into(),
            source_sha256: "d".repeat(64),
            width: 2,
            height: 2,
        }],
        clips,
        markers,
    }
}

fn prompt_pack() -> PromptPack {
    PromptPack {
        pack_id: "prompt-pack-export-test".into(),
        revision: 1,
        style_revision: 3,
        style_sha256: "e".repeat(64),
        motion_revision_hash: "f".repeat(64),
        provider_profile: "offline-human-handoff".into(),
        entries: ACTION_KEYS
            .iter()
            .map(|action| PromptEntry {
                asset_spec_id: format!("asset-{action}"),
                action_key: (*action).into(),
                pose_key: "contact".into(),
                positive: format!("side-view anime humanoid {action} key pose"),
                negative: "perspective view, extra weapon, text".into(),
            })
            .collect(),
        network_calls_made: 0,
    }
}

fn contains_exact_string(value: &serde_json::Value, needle: &str) -> bool {
    match value {
        serde_json::Value::String(value) => value == needle,
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| contains_exact_string(value, needle)),
        serde_json::Value::Object(values) => values
            .values()
            .any(|value| contains_exact_string(value, needle)),
        _ => false,
    }
}
#[test]
fn open_exports_are_deterministic_and_exact_patch() {
    let value = snapshot();
    let report = preflight(&value);
    assert!(report.passed, "{:?}", report.errors);
    assert_eq!(rig_ir_bytes(&value).unwrap(), rig_ir_bytes(&value).unwrap());
    let atlas = String::from_utf8(atlas_input_bytes(&value).unwrap()).unwrap();
    assert!(atlas.contains("not a Spine .atlas file"));
    let spine: String = String::from_utf8(spine_json_bytes(&value).unwrap()).unwrap();
    assert!(spine.contains("\"spine\":\"4.2.43\""));
    assert!(!spine.contains("4.3"));
}
#[test]
fn minimal_psd_has_layers_and_rejects_bad_pixels() {
    let layer = PsdLayer {
        name: "身体".into(),
        width: 2,
        height: 2,
        rgba: [40, 80, 160, 255].repeat(4),
        visible: true,
        opacity: 255,
        origin_x: 0,
        origin_y: 0,
    };
    let bytes = minimal_psd_bytes(2, 2, &[layer.clone()]).unwrap();
    assert_eq!(&bytes[..4], b"8BPS");
    assert_eq!(u16::from_be_bytes([bytes[22], bytes[23]]), 8);
    let mut bad = layer;
    bad.rgba.pop();
    assert!(minimal_psd_bytes(2, 2, &[bad]).is_err());
}

#[test]
fn proprietary_extensions_require_exact_external_operation_provenance() {
    assert!(!proprietary_output_allowed(
        Path::new("character.atlas"),
        None,
        None
    ));
    assert!(!proprietary_output_allowed(
        Path::new("character.skel"),
        Some("op-1"),
        Some("4.2.44")
    ));
    assert!(proprietary_output_allowed(
        Path::new("character.spine"),
        Some("op-1"),
        Some("4.2.43")
    ));
    assert!(proprietary_output_allowed(
        Path::new("character.spine.json"),
        None,
        None
    ));
    let policy = SpineCliPolicy {
        executable: PathBuf::from("Spine.com"),
        user_confirmed_professional_license: true,
        network_granted_for_operation: false,
        expected_patch: "4.2.43".into(),
    };
    assert!(
        policy.validate().is_err(),
        "relative executable must fail closed"
    );
}

#[test]
fn export_paths_fail_closed_on_windows_escape_syntax() {
    for unsafe_path in [
        "../escape.png",
        "C:/escape.png",
        "images/../escape.png",
        "images/body.png:secret",
        "images/CON.png",
        "\\\\server\\share\\body.png",
    ] {
        let mut value = snapshot();
        value.attachments[0].logical_png_path = unsafe_path.into();
        let report = preflight(&value);
        assert!(!report.passed, "path must fail: {unsafe_path}");
        assert_eq!(report.publish_status, "BLOCKED");
    }
    let mut value = snapshot();
    value.export_id = "..\\outside".into();
    assert!(!preflight(&value).passed);
}

#[test]
fn spine42_serializes_real_track_mesh_weight_constraint_and_event_subset() {
    let mut value = snapshot();
    value.bones.bones.push(BoneNode {
        bone_id: "target".into(),
        name: "Target".into(),
        parent_id: Some("root".into()),
        rest: RestTransform::default(),
    });
    value.meshes.push(Mesh {
        mesh_id: "body".into(),
        layer_id: "body".into(),
        topology_revision: 1,
        vertices: vec![
            Vertex {
                vertex_id: 10,
                x_milli_px: 0,
                y_milli_px: 0,
                u_ppm: 0,
                v_ppm: 0,
            },
            Vertex {
                vertex_id: 20,
                x_milli_px: 1000,
                y_milli_px: 0,
                u_ppm: 1_000_000,
                v_ppm: 0,
            },
            Vertex {
                vertex_id: 30,
                x_milli_px: 0,
                y_milli_px: 1000,
                u_ppm: 0,
                v_ppm: 1_000_000,
            },
        ],
        triangles: vec![Triangle(10, 20, 30)],
    });
    value.weights.push(WeightSet {
        mesh_id: "body".into(),
        topology_revision: 1,
        by_vertex: BTreeMap::from([10, 20, 30].map(|id| {
            (
                id,
                vec![BoneWeight {
                    bone_id: "root".into(),
                    weight_ppm: 1_000_000,
                }],
            )
        })),
    });
    value.constraints.push(RigConstraint {
        constraint_id: "root-follow-target".into(),
        kind: ConstraintKind::Transform,
        constrained_bone_id: "root".into(),
        target_bone_id: "target".into(),
        mix_ppm: 500_000,
        order: 0,
    });
    value.clips[0].tracks.extend([
        Track {
            track_id: "root-translate".into(),
            target_id: "root".into(),
            channel: TrackChannel::BoneTranslate,
            keyframes: vec![Keyframe {
                keyframe_id: "move".into(),
                tick: 1000,
                values_milli: vec![1500, -500],
                curve: Curve::Stepped,
                bezier_milli: None,
            }],
        },
        Track {
            track_id: "body-color".into(),
            target_id: "body-slot".into(),
            channel: TrackChannel::SlotColor,
            keyframes: vec![Keyframe {
                keyframe_id: "tint".into(),
                tick: 2000,
                values_milli: vec![1000, 500, 0, 1000],
                curve: Curve::Linear,
                bezier_milli: None,
            }],
        },
        Track {
            track_id: "body-deform".into(),
            target_id: "body-slot/body".into(),
            channel: TrackChannel::Deform,
            keyframes: vec![Keyframe {
                keyframe_id: "deform".into(),
                tick: 3000,
                values_milli: vec![0, 0, 100, 0, 0, 100],
                curve: Curve::Linear,
                bezier_milli: None,
            }],
        },
        Track {
            track_id: "footstep-event".into(),
            target_id: "footstep".into(),
            channel: TrackChannel::Event,
            keyframes: vec![Keyframe {
                keyframe_id: "footstep".into(),
                tick: 4000,
                values_milli: vec![],
                curve: Curve::Linear,
                bezier_milli: None,
            }],
        },
    ]);
    value.action_approvals[0].clip_sha256 =
        f2s_domain::canonical::canonical_sha256(&value.clips[0]).unwrap();
    let bytes = spine_json_bytes(&value).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["transform"][0]["bones"][0], "root");
    assert_eq!(
        json["skins"][0]["attachments"]["body-slot"]["body"]["type"],
        "mesh"
    );
    assert!(
        json["skins"][0]["attachments"]["body-slot"]["body"]["vertices"]
            .as_array()
            .unwrap()
            .len()
            > 6
    );
    assert_eq!(
        json["animations"]["idle"]["bones"]["root"]["translate"][0]["x"],
        1.5
    );
    assert_eq!(
        json["animations"]["idle"]["slots"]["body-slot"]["color"][0]["color"],
        "FF8000FF"
    );
    assert!(json["animations"]["idle"]["deform"]["default"]["body-slot"]["body"].is_array());
    assert!(json["events"]["footstep"].is_object());
    assert!(!String::from_utf8(bytes).unwrap().contains("f2sDuration"));
}

#[test]
fn spine42_rejects_time_rounding_collisions() {
    let mut value = snapshot();
    value.time_base = TimeBase {
        numerator: 1,
        denominator: 2_000_000_000,
    };
    value.clips[0].tracks[0].keyframes[1].tick = 1;
    assert!(spine_json_bytes(&value).is_err());
}

#[test]
fn committed_package_is_immutable_complete_and_self_checked() {
    let png = include_bytes!("../../../fixtures/m00/spine42-probe/attachments/body.png").to_vec();
    let dimensions = ImageReader::with_format(Cursor::new(&png), ImageFormat::Png)
        .into_dimensions()
        .unwrap();
    let png_hash = format!("{:x}", Sha256::digest(&png));
    let mut value = snapshot();
    value.attachments[0].source_sha256 = png_hash;
    value.attachments[0].width = dimensions.0;
    value.attachments[0].height = dimensions.1;
    let psd_layers = vec![psd_layer_from_png("body", &png, true).unwrap()];
    let attachments = vec![AttachmentBytes {
        attachment_id: "body".into(),
        bytes: png,
    }];
    let prompt_pack = prompt_pack();
    let root =
        std::env::temp_dir().join(format!("f2s-export-package-test-{}", uuid::Uuid::new_v4()));
    let commit = commit_open_export(
        &value,
        &attachments,
        &psd_layers,
        dimensions,
        &prompt_pack,
        &root,
    )
    .unwrap();

    assert_eq!(commit.status, "EXPORTED_UNVERIFIED");
    assert_eq!(commit.external_editor_status, "EXPORTED_UNVERIFIED");
    assert!(commit.checksums.contains_key("compatibility-manifest.json"));
    assert!(commit.checksums.contains_key("prompt-pack.json"));
    assert!(commit.checksums.contains_key("prompt-pack.md"));
    assert!(!commit.checksums.contains_key("checksums.sha256"));

    let checksum_text = fs::read_to_string(commit.directory.join("checksums.sha256")).unwrap();
    for (path, expected_hash) in &commit.checksums {
        assert!(checksum_text.contains(&format!("{expected_hash}  {path}\n")));
        let actual = fs::read(commit.directory.join(path)).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(actual)), *expected_hash);
    }
    assert!(!checksum_text.contains("checksums.sha256"));

    let compatibility: serde_json::Value = serde_json::from_slice(
        &fs::read(commit.directory.join("compatibility-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(compatibility["targetPatch"], "4.2.43");
    assert_eq!(compatibility["packageStatus"], "EXPORTED_UNVERIFIED");
    assert_eq!(
        compatibility["spineEditorRoundTripStatus"],
        "EXPORTED_UNVERIFIED"
    );
    assert!(!compatibility["releaseReady"].as_bool().unwrap());
    assert!(!contains_exact_string(&compatibility, "VERIFIED"));
    assert_eq!(
        compatibility["intentionallyNotProduced"],
        serde_json::json!(["Spine .atlas", "Spine .spine", "Spine .skel"])
    );
    for artifact in compatibility["artifacts"].as_array().unwrap() {
        assert_eq!(artifact["status"], "CONTRACT_VERIFIED");
    }
    let inventory = compatibility["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|artifact| artifact["path"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let expected_inventory = BTreeSet::from([
        "rig-ir.json",
        "atlas-input-manifest.json",
        "character.spine.json",
        "character.psd",
        "prompt-pack.json",
        "prompt-pack.md",
        "images/body.png",
        "compatibility-manifest.json",
        "checksums.sha256",
    ]);
    assert_eq!(inventory, expected_inventory);

    let prompt_json = fs::read(commit.directory.join("prompt-pack.json")).unwrap();
    assert_eq!(
        prompt_json,
        f2s_domain::canonical::canonical_bytes(&prompt_pack).unwrap()
    );
    let prompt_markdown = fs::read_to_string(commit.directory.join("prompt-pack.md")).unwrap();
    assert!(prompt_markdown.contains("# AI Action Keyframe Prompt Pack"));
    for action in ACTION_KEYS {
        assert!(prompt_markdown.contains(&format!("- Action: `{action}`")));
    }
    let rig_ir: serde_json::Value =
        serde_json::from_slice(&fs::read(commit.directory.join("rig-ir.json")).unwrap()).unwrap();
    let first_bone = &rig_ir["bones"][0];
    assert!(first_bone.get("scaleXPpm").is_some());
    assert!(first_bone.get("scaleYPpm").is_some());
    assert!(first_bone.get("scaleXppm").is_none());
    assert!(first_bone.get("scaleYppm").is_none());

    let rig_ir_schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/src/rig-ir.schema.json")).unwrap();
    let bone_required = rig_ir_schema["$defs"]["Bone"]["required"]
        .as_array()
        .unwrap();
    for field in ["scaleXPpm", "scaleYPpm"] {
        assert!(bone_required.iter().any(|value| value == field));
        assert!(
            rig_ir_schema["$defs"]["Bone"]["properties"]
                .get(field)
                .is_some()
        );
    }
    assert!(
        fs::read_to_string(commit.directory.join("character.spine.json"))
            .unwrap()
            .contains("\"spine\":\"4.2.43\"")
    );
    assert!(commit.directory.join("character.psd").is_file());
    assert!(commit.directory.join("images/body.png").is_file());
    assert!(commit.directory.join("atlas-input-manifest.json").is_file());
    for forbidden in ["character.atlas", "character.spine", "character.skel"] {
        assert!(!commit.directory.join(forbidden).exists());
    }

    let before = fs::read(commit.directory.join("rig-ir.json")).unwrap();
    let overwrite = commit_open_export(
        &value,
        &attachments,
        &psd_layers,
        dimensions,
        &prompt_pack,
        &root,
    );
    assert!(overwrite.is_err());
    assert_eq!(
        fs::read(commit.directory.join("rig-ir.json")).unwrap(),
        before
    );
    assert!(!root.join(".export-001.f2s-staging").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_rejects_non_offline_prompt_pack_before_creating_output() {
    let mut prompt_pack = prompt_pack();
    prompt_pack.network_calls_made = 1;
    let root = std::env::temp_dir().join(format!(
        "f2s-export-prompt-reject-test-{}",
        uuid::Uuid::new_v4()
    ));
    let result = commit_open_export(&snapshot(), &[], &[], (1, 1), &prompt_pack, &root);
    assert!(result.is_err());
    assert!(!root.exists());
}
