use f2s_domain::governance::CredentialAttestation;

pub trait HumanCredentialVerifier {
    fn verify_and_consume(&self, attestation: &CredentialAttestation) -> Result<(), String>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedHumanActor {
    actor_id: String,
    attestation_id: String,
    proof_sha256: String,
    purpose: String,
    payload_sha256: String,
}

impl VerifiedHumanActor {
    pub fn verify(
        attestation: CredentialAttestation,
        expected_purpose: &str,
        expected_payload_sha256: &str,
        verifier: &dyn HumanCredentialVerifier,
    ) -> Result<Self, String> {
        if attestation.actor_kind != "HUMAN"
            || attestation.actor_id.trim().is_empty()
            || attestation.attestation_id.trim().is_empty()
            || attestation.purpose != expected_purpose
            || attestation.payload_sha256 != expected_payload_sha256
            || attestation.verification_proof_sha256.len() != 64
            || !attestation
                .verification_proof_sha256
                .bytes()
                .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
        {
            return Err(
                "human credential attestation is invalid or bound to another purpose".into(),
            );
        }
        verifier.verify_and_consume(&attestation)?;
        Ok(Self {
            actor_id: attestation.actor_id,
            attestation_id: attestation.attestation_id,
            proof_sha256: attestation.verification_proof_sha256,
            purpose: attestation.purpose,
            payload_sha256: attestation.payload_sha256,
        })
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }
    pub fn attestation_id(&self) -> &str {
        &self.attestation_id
    }
    pub fn proof_sha256(&self) -> &str {
        &self.proof_sha256
    }
    pub fn require_binding(&self, purpose: &str, payload_sha256: &str) -> Result<(), String> {
        if self.purpose == purpose && self.payload_sha256 == payload_sha256 {
            Ok(())
        } else {
            Err("human attestation is not bound to this gate payload".into())
        }
    }
}
