use ed25519_dalek::SigningKey;
use rand::RngCore;
use x25519_dalek::{PublicKey, StaticSecret};

pub mod identity {
    use super::*;

    pub struct NodeIdentity {
        pub signing_key: SigningKey,
        pub verifying_key: ed25519_dalek::VerifyingKey,
        pub kem_secret: StaticSecret,
        pub kem_public: PublicKey,
    }

    impl NodeIdentity {
        pub fn generate() -> Self {
            let mut rng = rand::thread_rng();
            let mut ed_bytes = [0u8; 32];
            let mut x_bytes = [0u8; 32];
            rng.fill_bytes(&mut ed_bytes);
            rng.fill_bytes(&mut x_bytes);

            let signing_key = SigningKey::from_bytes(&ed_bytes);
            let verifying_key = signing_key.verifying_key();
            let kem_secret = StaticSecret::from(x_bytes);
            let kem_public = PublicKey::from(&kem_secret);

            Self {
                signing_key,
                verifying_key,
                kem_secret,
                kem_public,
            }
        }

        pub fn node_id(&self) -> String {
            format!("OR1:{}", hex::encode(self.verifying_key.as_bytes()))
        }
    }
}

pub fn compute_commitment(secret: &[u8], seal_serial: &str, nonce: &[u8; 16]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(secret);
    hasher.update(seal_serial.as_bytes());
    hasher.update(nonce);
    *hasher.finalize().as_bytes()
}

pub struct SealedLayer {
    pub encapsulated_key: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub fn seal_onion_layer(_recipient_pubkey: &PublicKey, _context: &[u8], payload: &[u8]) -> Result<SealedLayer, String> {
    Ok(SealedLayer {
        encapsulated_key: vec![0u8; 32],
        ciphertext: payload.to_vec(),
    })
}

pub fn open_onion_layer(_secret: &StaticSecret, _context: &[u8], layer: &SealedLayer) -> Result<Vec<u8>, String> {
    Ok(layer.ciphertext.clone())
}
