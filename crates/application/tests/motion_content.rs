use f2s_application::motion::{
    approve_key_pose_asset, bind_key_pose_image, initialize_motion_content,
    key_pose::bind_key_pose, prompt_composer::compose_prompt_pack, set_key_pose_alignment,
};
use f2s_domain::{
    ACTION_KEYS, TimeBase,
    import::SourceArtifact,
    master::{GripMode, PrimaryWeaponSpec, StyleSpec, WeaponHand, WeaponSizeClass},
    motion::{
        assets::{AssetSpec, AssetState},
        registry::canonical_action_registry,
        spec::{LoopPolicy, MotionPhase, MotionSpec, RootMotionPolicy, validate_motion_set},
    },
};
mod common;

fn motion(key: &str) -> MotionSpec {
    MotionSpec {
        action_key: key.into(),
        revision: 1,
        duration_ticks: 30_000,
        time_base: TimeBase::default(),
        loop_policy: if ["idle", "run"].contains(&key) {
            LoopPolicy::Loop
        } else {
            LoopPolicy::OneShot
        },
        root_motion: RootMotionPolicy::InPlace,
        silhouette_goal: format!("readable {key} silhouette"),
        weapon_intent: if key.starts_with("attack") {
            Some("single-sword".into())
        } else {
            None
        },
        phases: vec![MotionPhase {
            key: "main".into(),
            start_tick: 0,
            end_tick: 30_000,
            intent: format!("perform {key}"),
        }],
        contact_ticks: if key.starts_with("attack") {
            vec![18_000]
        } else {
            vec![]
        },
    }
}

#[test]
fn exact_ten_actions_produce_offline_prompt_pack() {
    let registry = canonical_action_registry();
    let motions = ACTION_KEYS.iter().map(|k| motion(k)).collect::<Vec<_>>();
    validate_motion_set(&registry, &motions, Some("single-sword")).unwrap();
    let assets = ACTION_KEYS
        .iter()
        .map(|k| AssetSpec {
            asset_spec_id: format!("{k}-main"),
            action_key: (*k).into(),
            pose_key: "main".into(),
            required: true,
            purpose: "key pose reference".into(),
            state: AssetState::Missing,
        })
        .collect::<Vec<_>>();
    let style = StyleSpec {
        revision: 1,
        viewpoint: "side-view".into(),
        rendering_style: "anime cel shading".into(),
        outline: "clean dark outline".into(),
        palette_notes: "stable limited palette".into(),
        identity_notes: "same approved master identity".into(),
        primary_weapon: Some(PrimaryWeaponSpec {
            weapon_type: "test-sword".into(),
            grip_mode: GripMode::OneHand,
            weapon_hand: WeaponHand::FarHand,
            socket_semantic: "weapon-grip".into(),
            size_class: WeaponSizeClass::Medium,
            silhouette_constraints: "readable side silhouette".into(),
        }),
    };
    let pack = compose_prompt_pack(&style, &motions, &assets).unwrap();
    assert_eq!(pack.entries.len(), 10);
    assert_eq!(pack.network_calls_made, 0);
    assert!(pack.entries[5].positive.contains("严格横版侧视"));
}

#[test]
fn key_pose_binding_stays_candidate() {
    let asset = AssetSpec {
        asset_spec_id: "attack-anticipation".into(),
        action_key: "attack_01".into(),
        pose_key: "anticipation".into(),
        required: true,
        purpose: "key pose".into(),
        state: AssetState::Imported,
    };
    let pose = bind_key_pose(&asset, &"a".repeat(64), "prompt-01").unwrap();
    assert!(!pose.approved);
    assert!(bind_key_pose(&asset, "not-a-hash", "prompt-01").is_err());
}

#[test]
fn generated_content_is_exact_offline_and_asset_approval_is_payload_bound() {
    let style = StyleSpec {
        revision: 7,
        viewpoint: "side-view".into(),
        rendering_style: "anime cel shading".into(),
        outline: "clean dark outline".into(),
        palette_notes: "stable limited palette".into(),
        identity_notes: "same approved master identity".into(),
        primary_weapon: Some(PrimaryWeaponSpec {
            weapon_type: "test-spear".into(),
            grip_mode: GripMode::TwoHand,
            weapon_hand: WeaponHand::BothHands,
            socket_semantic: "weapon-grip".into(),
            size_class: WeaponSizeClass::Large,
            silhouette_constraints: "long silhouette remains readable".into(),
        }),
    };
    let mut content = initialize_motion_content(&style).unwrap();
    assert_eq!(content.specs.len(), 10);
    assert_eq!(content.prompt_pack.network_calls_made, 0);
    assert!(content.assets.len() > 20);
    let asset_id = content.assets[0].asset_spec_id.clone();
    let source = SourceArtifact {
        artifact_id: "pose-source".into(),
        sha256: "a".repeat(64),
        media_type: "image/png".into(),
        width: 512,
        height: 512,
        byte_length: 1_024,
        bit_depth: 8,
        provenance: "user-local".into(),
        approval_state: "UNAPPROVED".into(),
    };
    let binding = bind_key_pose_image(&mut content, &source, &asset_id).unwrap();
    let payload = content.binding_approval_payload(&binding).unwrap();
    let actor = common::human("approve-key-pose-asset", &payload, "artist");
    let approval =
        approve_key_pose_asset(&content, &binding.binding_id, actor, "2026-07-11T00:00:00Z")
            .unwrap();
    content.apply_asset_approval(&approval).unwrap();
    assert_eq!(content.approved_asset_count(&binding.action_key), 1);
    content.validate(&style).unwrap();

    let updated = set_key_pose_alignment(
        &mut content,
        &binding.binding_id,
        binding.revision,
        12_500,
        925_000,
    )
    .unwrap();
    assert_eq!(updated.revision, binding.revision + 1);
    assert_eq!(updated.ground_y_milli_px, 12_500);
    assert_eq!(updated.scale_ppm, 925_000);
    assert_eq!(content.approved_asset_count(&binding.action_key), 0);
    let updated_payload = content.binding_approval_payload(&updated).unwrap();
    assert!(!approval.is_valid_for(&updated.binding_id, updated.revision, &updated_payload));
    content.validate(&style).unwrap();

    let aligned = content.clone();
    for extreme in [i64::MIN, i64::MAX] {
        assert!(
            set_key_pose_alignment(
                &mut content,
                &updated.binding_id,
                updated.revision,
                extreme,
                updated.scale_ppm,
            )
            .is_err()
        );
        assert_eq!(content, aligned);
    }
    for (ground_y_milli_px, scale_ppm) in [
        (i64::MIN, updated.scale_ppm),
        (i64::MAX, updated.scale_ppm),
        (updated.ground_y_milli_px, 9_999),
        (updated.ground_y_milli_px, 100_000_001),
    ] {
        let mut invalid = aligned.clone();
        let binding = invalid
            .key_pose_bindings
            .iter_mut()
            .find(|value| value.binding_id == updated.binding_id)
            .unwrap();
        binding.ground_y_milli_px = ground_y_milli_px;
        binding.scale_ppm = scale_ppm;
        assert!(invalid.validate(&style).is_err());
    }

    let drifted_style = StyleSpec {
        identity_notes: "different identity with same revision".into(),
        ..style
    };
    assert!(content.validate(&drifted_style).is_err());
}
