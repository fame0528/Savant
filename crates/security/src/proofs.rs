//! Kani Bounded Model Checking for Cryptographic Capabilities
//!
//! Symbolically executes the `verify_token_and_action` function to mathematically
//! prove the absence of memory violations or panics under arbitrary hostile input.

#[cfg(kani)]
mod verification {
    use crate::enclave::SecurityEnclave;
    use crate::token::{AgentToken, CapabilityPayload};
    use ed25519_dalek::{SigningKey};
    use rand_core::OsRng;

    #[kani::proof]
    #[kani::unwind(10)]
    pub fn verify_sandbox_security_boundary_no_panic() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let enclave = SecurityEnclave::new(verifying_key);

        let symbolic_hash: u64 = kani::any();
        let symbolic_expires: u64 = kani::any();
        let symbolic_resource = kani::any_string(10);
        let symbolic_action = kani::any_string(10);
        let symbolic_sig_bytes: [u8; 64] = kani::any();

        let hostile_token = AgentToken {
            payload: CapabilityPayload {
                assignee_hash: symbolic_hash,
                resource_uri: symbolic_resource,
                permitted_action: symbolic_action,
                expires_at: symbolic_expires,
            },
            signature_bytes: symbolic_sig_bytes,
        };

        let requested_agent_id: u64 = kani::any();
        let requested_resource = kani::any_string(10);
        let requested_action = kani::any_string(10);

        let result = enclave.verify_token_and_action(
            &hostile_token,
            requested_agent_id,
            &requested_resource,
            &requested_action,
        );

        if result.is_ok() {
            kani::assert(
                hostile_token.payload.assignee_hash == requested_agent_id,
                "Token ID must match requested ID on success"
            );
        }
    }
}
