use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialAttestation {
    pub attestation_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub credential_ref: String,
    pub purpose: String,
    pub issued_at_utc: String,
    pub expires_at_utc: String,
    pub payload_sha256: String,
    pub verification_proof_sha256: String,
}
