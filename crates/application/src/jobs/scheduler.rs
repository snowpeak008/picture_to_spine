use f2s_domain::jobs::{Job, JobState};
pub fn start(job: &mut Job) -> Result<(), String> {
    if job.state != JobState::Queued {
        return Err("job is not queued".into());
    }
    job.state = JobState::Running;
    Ok(())
}
