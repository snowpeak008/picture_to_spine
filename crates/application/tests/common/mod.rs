use f2s_application::approvals::{HumanCredentialVerifier, VerifiedHumanActor};
use f2s_domain::governance::CredentialAttestation;

struct TestVerifier;
impl HumanCredentialVerifier for TestVerifier {
    fn verify_and_consume(&self, attestation: &CredentialAttestation) -> Result<(), String> {
        if attestation.credential_ref.starts_with("test-credential://")
            && attestation.issued_at_utc < attestation.expires_at_utc
        {
            Ok(())
        } else {
            Err("test credential rejected".into())
        }
    }
}

pub fn human(purpose: &str, payload_sha256: &str, actor: &str) -> VerifiedHumanActor {
    VerifiedHumanActor::verify(
        CredentialAttestation {
            attestation_id: format!("attestation-{actor}"),
            actor_id: actor.into(),
            actor_kind: "HUMAN".into(),
            credential_ref: format!("test-credential://{actor}"),
            purpose: purpose.into(),
            issued_at_utc: "2026-01-01T00:00:00Z".into(),
            expires_at_utc: "2027-01-01T00:00:00Z".into(),
            payload_sha256: payload_sha256.into(),
            verification_proof_sha256: "f".repeat(64),
        },
        purpose,
        payload_sha256,
        &TestVerifier,
    )
    .expect("valid test human attestation")
}
