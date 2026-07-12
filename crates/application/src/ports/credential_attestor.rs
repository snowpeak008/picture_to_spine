use f2s_domain::governance::CredentialAttestation;
pub trait CredentialAttestor {
    fn attest(&self, purpose: &str, payload_sha256: &str) -> Result<CredentialAttestation, String>;
}
