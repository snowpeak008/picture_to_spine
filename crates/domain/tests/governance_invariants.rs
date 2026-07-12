use f2s_domain::{
    canonical::canonical_sha256,
    governance::{Gate, GateState, Priority, Rule},
    storage::ClockCheckpoint,
    validate_action_key,
};

#[test]
fn hard_rules_cannot_be_waived() {
    let rule = Rule {
        rule_id: "license".into(),
        priority: Priority::License,
        hard: true,
        message: "blocked".into(),
    };
    assert!(rule.cannot_be_waived());
    assert!(!Priority::Quality.can_override(Priority::Safety));
}
#[test]
fn approval_is_revision_bound() {
    let gate = Gate {
        gate_id: "master".into(),
        target_id: "asset".into(),
        target_revision: 7,
        state: GateState::Approved,
        dependency_gate_ids: vec![],
    };
    assert!(gate.is_currently_approved(7));
    assert!(!gate.is_currently_approved(8));
}
#[test]
fn canonical_hash_ignores_object_key_order() {
    let a = serde_json::json!({"b":2,"a":1});
    let b = serde_json::json!({"a":1,"b":2});
    assert_eq!(canonical_sha256(&a).unwrap(), canonical_sha256(&b).unwrap());
}
#[test]
fn aliases_and_clock_downgrade_are_rejected() {
    assert!(validate_action_key("attack_1").is_err());
    let old = ClockCheckpoint {
        epoch: 3,
        sequence: 4,
        observed_at_utc: "a".into(),
        previous_sha256: None,
    };
    let bad = ClockCheckpoint {
        epoch: 2,
        sequence: 5,
        observed_at_utc: "b".into(),
        previous_sha256: Some("x".into()),
    };
    assert!(!bad.can_follow(&old));
}
