use f2s_application::{
    commands::{IdempotencyRegistry, execute_command},
    jobs::arbitrate,
};
use f2s_domain::{
    commands::Command,
    jobs::{Job, JobState},
};

#[test]
fn command_is_idempotent_and_revision_checked() {
    let command = Command {
        command_id: "c1".into(),
        project_id: "p".into(),
        expected_revision: 2,
        kind: "rename".into(),
        payload: serde_json::json!({}),
    };
    let mut registry = IdempotencyRegistry::default();
    let first = execute_command(&command, 2, &mut registry).unwrap();
    let again = execute_command(&command, 999, &mut registry).unwrap();
    assert_eq!(first, again);
    let other = Command {
        command_id: "c2".into(),
        expected_revision: 1,
        ..command
    };
    assert!(execute_command(&other, 2, &mut registry).is_err());
}
#[test]
fn one_terminal_state_wins() {
    let mut job = Job {
        job_id: "j".into(),
        kind: "layer".into(),
        state: JobState::Running,
        project_revision: 1,
        created_at_utc: "now".into(),
        terminal_sequence: None,
    };
    assert!(arbitrate(&mut job, JobState::Succeeded, 10).is_ok());
    assert!(arbitrate(&mut job, JobState::Failed, 11).is_err());
    assert_eq!(job.state, JobState::Succeeded);
}
