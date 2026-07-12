use super::journal::{JournalRecord, read_all};
use std::path::Path;
pub fn replay_committed(path: &Path) -> Result<Vec<JournalRecord>, String> {
    let records = read_all(path)?;
    let mut expected = 0;
    let mut out = Vec::new();
    for record in records {
        if record.sequence != expected {
            return Err("journal sequence gap".into());
        }
        expected += 1;
        if record.committed {
            out.push(record)
        }
    }
    Ok(out)
}
