use f2s_domain::storage::ProjectHead;
pub fn select_last_good(mut candidates: Vec<ProjectHead>) -> Option<ProjectHead> {
    candidates.sort_by_key(|v| v.head_revision);
    candidates.pop()
}
