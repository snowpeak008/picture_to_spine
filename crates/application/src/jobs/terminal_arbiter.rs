use f2s_domain::jobs::{Job, JobState};
pub fn arbitrate(job: &mut Job, proposed: JobState, sequence: u64) -> Result<(), String> {
    if job.accept_terminal(proposed, sequence) {
        Ok(())
    } else {
        Err("terminal state already won or proposal is nonterminal".into())
    }
}
