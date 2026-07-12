use f2s_domain::governance::Approval;
pub fn revalidate(approval: &Approval, id: &str, revision: u64, sha: &str) -> Result<(), String> {
    if approval.is_valid_for(id, revision, sha) {
        Ok(())
    } else {
        Err("approval is stale or invalidated".into())
    }
}
