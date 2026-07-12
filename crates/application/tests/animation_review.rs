use f2s_application::animation::review::{approve_hit_frame, approve_key_poses};
use f2s_domain::canonical::canonical_sha256;
use f2s_domain::{
    TimeBase,
    animation::{
        clip::{AnimationClip, Curve, Keyframe, Track, TrackChannel},
        markers::{GameplayMarker, GameplayMarkerKind},
    },
};
mod common;
fn attack() -> AnimationClip {
    AnimationClip {
        clip_id: "clip-attack-01".into(),
        action_key: "attack_01".into(),
        revision: 1,
        duration_ticks: 30_000,
        time_base: TimeBase::default(),
        tracks: vec![Track {
            track_id: "hand-rotation".into(),
            target_id: "weapon-hand".into(),
            channel: TrackChannel::BoneRotate,
            keyframes: vec![
                Keyframe {
                    keyframe_id: "anticipation".into(),
                    tick: 0,
                    values_milli: vec![-30_000],
                    curve: Curve::Bezier,
                    bezier_milli: Some([250, 0, 750, 1000]),
                },
                Keyframe {
                    keyframe_id: "contact".into(),
                    tick: 18_000,
                    values_milli: vec![75_000],
                    curve: Curve::Linear,
                    bezier_milli: None,
                },
            ],
        }],
    }
}
#[test]
fn pose_and_hit_approvals_are_independent_human_gates() {
    let clip = attack();
    let required = vec!["anticipation".into(), "contact".into()];
    let pose_actor = common::human(
        "approve-key-poses",
        &canonical_sha256(&clip).unwrap(),
        "animator-01",
    );
    let pose = approve_key_poses(&clip, &required, &required, pose_actor).unwrap();
    assert!(pose.valid);
    let markers = vec![GameplayMarker {
        marker_id: "hit-01".into(),
        action_key: "attack_01".into(),
        kind: GameplayMarkerKind::HitFrame,
        start_tick: 18_000,
        end_tick: 18_000,
        socket_id: Some("weapon".into()),
    }];
    let hit_actor = common::human(
        "approve-hit-frame",
        &canonical_sha256(&markers).unwrap(),
        "combat-designer-01",
    );
    let hit = approve_hit_frame(&clip, &markers, hit_actor).unwrap();
    assert!(hit.valid);
    assert_ne!(pose.clip_hash, hit.marker_hash);
}
#[test]
fn attack_without_single_hit_or_with_ai_actor_fails() {
    let clip = attack();
    let actor = common::human("approve-hit-frame", &"0".repeat(64), "designer");
    assert!(approve_hit_frame(&clip, &[], actor).is_err());
    let markers = vec![GameplayMarker {
        marker_id: "hit-01".into(),
        action_key: "attack_01".into(),
        kind: GameplayMarkerKind::HitFrame,
        start_tick: 18_000,
        end_tick: 18_000,
        socket_id: Some("weapon".into()),
    }];
    assert!(
        approve_hit_frame(
            &clip,
            &markers,
            common::human("approve-hit-frame", &"0".repeat(64), "designer")
        )
        .is_err(),
        "an attestation bound to another marker payload must fail"
    );
}
