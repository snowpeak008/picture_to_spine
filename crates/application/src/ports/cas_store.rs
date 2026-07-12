use f2s_domain::storage::CasRef;
pub trait CasStore {
    fn put(&self, media_type: &str, bytes: &[u8]) -> Result<CasRef, String>;
    fn get(&self, reference: &CasRef) -> Result<Vec<u8>, String>;
}
