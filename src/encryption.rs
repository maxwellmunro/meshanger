use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use sha2::{Digest, Sha256};
use tokio::sync::RwLockReadGuard;

fn make_nonce(sender_id: u64, counter: u64) -> [u8; 24] {
    let mut nonce = [0u8; 24];

    nonce[..8].copy_from_slice(&sender_id.to_be_bytes());

    nonce[8..16].copy_from_slice(&counter.to_be_bytes());

    nonce
}

fn make_aad(sender_id: u64, counter: u64) -> Vec<u8> {
    let mut aad = Vec::new();

    aad.extend_from_slice(&sender_id.to_be_bytes());

    aad.extend_from_slice(&counter.to_be_bytes());

    aad
}

pub fn encrypt_message(
    cipher: XChaCha20Poly1305,
    sender_id: u64,
    counter: u64,
    plaintext: &[u8],
) -> Vec<u8> {
    let nonce_bytes = make_nonce(sender_id, counter);

    let nonce = XNonce::from_slice(&nonce_bytes);

    let aad = make_aad(sender_id, counter);

    cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .expect("encryption failed")
}

pub fn decrypt_message(
    cipher: XChaCha20Poly1305,
    sender_id: u64,
    counter: u64,
    ciphertext: &[u8],
) -> Result<Vec<u8>, String> {
    let nonce_bytes = make_nonce(sender_id, counter);

    let nonce = XNonce::from_slice(&nonce_bytes);

    let aad = make_aad(sender_id, counter);

    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        ).map_err(|e| e.to_string())
}

pub fn generate_key(user_input: &str) -> [u8; 32] {
    let mut random_bytes = [0u8; 32];
    getrandom::getrandom(&mut random_bytes).expect("failed to get OS randomness");

    let mut hasher = Sha256::new();

    hasher.update(random_bytes);
    hasher.update(user_input.as_bytes());

    let result = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&result);

    key
}
