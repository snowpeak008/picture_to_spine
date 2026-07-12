use f2s_domain::governance::{Priority, Waiver};
#[test]
fn safety_and_license_waivers_fail_closed() {
    for priority in [Priority::Safety, Priority::License] {
        let waiver = Waiver {
            waiver_id: "w".into(),
            rule_id: "hard".into(),
            rule_priority: priority,
            target_revision: 1,
            expires_at_utc: "future".into(),
            limitation_acknowledged: true,
        };
        assert!(waiver.validate().is_err());
    }
}
