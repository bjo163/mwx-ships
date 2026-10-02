use moonships::services::crypto::CryptoService;

#[test]
fn test_encryption_and_decryption_roundtrip() {
    let secret_data = "SUPER_SECRET_SSH_PRIVATE_KEY_12345!@#$%^&*()";
    let encrypted = CryptoService::encrypt(secret_data).expect("encryption should succeed");

    assert_ne!(
        secret_data, encrypted,
        "encrypted text must not match plaintext"
    );

    let decrypted = CryptoService::decrypt(&encrypted).expect("decryption should succeed");
    assert_eq!(
        secret_data, decrypted,
        "decrypted text must match original plaintext"
    );
}

#[test]
fn test_encryption_uses_random_nonce() {
    let secret = "identic_secret";
    let enc1 = CryptoService::encrypt(secret).expect("enc1 ok");
    let enc2 = CryptoService::encrypt(secret).expect("enc2 ok");

    assert_ne!(
        enc1, enc2,
        "two encryptions of the same plaintext must have different nonces"
    );

    assert_eq!(CryptoService::decrypt(&enc1).unwrap(), secret);
    assert_eq!(CryptoService::decrypt(&enc2).unwrap(), secret);
}

#[test]
fn test_decryption_fails_on_tampered_ciphertext() {
    let encrypted = CryptoService::encrypt("hello world").unwrap();
    let mut tampered = encrypted.clone();

    // Corrupt one character
    if let Some(last_char) = tampered.pop() {
        let replacement = if last_char == 'A' { 'B' } else { 'A' };
        tampered.push(replacement);
    }

    let result = CryptoService::decrypt(&tampered);
    assert!(
        result.is_err(),
        "decryption of tampered ciphertext must fail authenticated tag check"
    );
}
