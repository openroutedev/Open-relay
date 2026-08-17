use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
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

#[derive(Debug, Clone)]
pub struct SealedLayer {
    pub encapsulated_key: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub fn seal_onion_layer(recipient_pubkey: &PublicKey, payload: &[u8]) -> Result<SealedLayer, String> {
    let mut rng = rand::thread_rng();
    let mut eph_bytes = [0u8; 32];
    rng.fill_bytes(&mut eph_bytes);

    let ephemeral_secret = StaticSecret::from(eph_bytes);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);
    let shared_secret = ephemeral_secret.diffie_hellman(recipient_pubkey);

    let key_bytes = blake3::hash(shared_secret.as_bytes());
    let cipher = ChaCha20Poly1305::new_from_slice(key_bytes.as_bytes())
        .map_err(|e| format!("Cipher init error: {:?}", e))?;

    let mut nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, payload)
        .map_err(|e| format!("Encryption error: {:?}", e))?;

    Ok(SealedLayer {
        encapsulated_key: ephemeral_public.as_bytes().to_vec(),
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    })
}

pub fn open_onion_layer(recipient_secret: &StaticSecret, layer: &SealedLayer) -> Result<Vec<u8>, String> {
    if layer.encapsulated_key.len() != 32 {
        return Err("Invalid encapsulated key length".into());
    }
    let mut eph_pub_bytes = [0u8; 32];
    eph_pub_bytes.copy_from_slice(&layer.encapsulated_key);
    let ephemeral_public = PublicKey::from(eph_pub_bytes);

    let shared_secret = recipient_secret.diffie_hellman(&ephemeral_public);
    let key_bytes = blake3::hash(shared_secret.as_bytes());
    let cipher = ChaCha20Poly1305::new_from_slice(key_bytes.as_bytes())
        .map_err(|e| format!("Cipher init error: {:?}", e))?;

    if layer.nonce.len() != 12 {
        return Err("Invalid nonce length".into());
    }
    let nonce = Nonce::from_slice(&layer.nonce);

    cipher
        .decrypt(nonce, layer.ciphertext.as_ref())
        .map_err(|e| format!("Decryption error: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onion_encryption_roundtrip() {
        let recipient = identity::NodeIdentity::generate();
        let msg = b"SECRET_ROUTING_INSTRUCTION_HUB_SLC";

        let sealed = seal_onion_layer(&recipient.kem_public, msg).unwrap();
        let opened = open_onion_layer(&recipient.kem_secret, &sealed).unwrap();

        assert_eq!(msg.to_vec(), opened);
    }
}
