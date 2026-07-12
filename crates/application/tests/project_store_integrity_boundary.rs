use f2s_application::{
    ports::{CasStore, ProjectStore},
    project::open_project,
};
use f2s_domain::storage::{CasRef, ProjectHead};
use std::sync::atomic::{AtomicUsize, Ordering};

struct RejectingProjectStore;

impl ProjectStore for RejectingProjectStore {
    fn load_head(&self, _project_id: &str) -> Result<Option<ProjectHead>, String> {
        Err("integrity verification failed".into())
    }

    fn commit_head(&self, _head: &ProjectHead, _manifest: &[u8]) -> Result<ProjectHead, String> {
        unreachable!("test store is read-only")
    }
}

#[derive(Default)]
struct CountingCas {
    reads: AtomicUsize,
}

impl CasStore for CountingCas {
    fn put(&self, _media_type: &str, _bytes: &[u8]) -> Result<CasRef, String> {
        unreachable!("test CAS is read-only")
    }

    fn get(&self, _reference: &CasRef) -> Result<Vec<u8>, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Err("CAS must not be reached".into())
    }
}

#[test]
fn open_project_verifies_head_before_reading_cas() {
    let cas = CountingCas::default();
    assert_eq!(
        open_project(&RejectingProjectStore, &cas, "project-1").unwrap_err(),
        "integrity verification failed"
    );
    assert_eq!(cas.reads.load(Ordering::SeqCst), 0);
}
