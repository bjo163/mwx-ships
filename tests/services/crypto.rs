use moonships::services::crypto::CryptoService;
use serial_test::serial;

fn configure_test_key() {
    std::env::set_var(
        "ENCRYPTION_KEY",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
}

#[test]
#[serial]
fn test_encryption_and_decryption_roundtrip() {
    configure_test_key();
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
#[serial]
fn test_encryption_uses_random_nonce() {
    configure_test_key();
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
#[serial]
fn test_decryption_fails_on_tampered_ciphertext() {
    configure_test_key();
    let encrypted = CryptoService::encrypt("hello world").unwrap();
    let (nonce_hex, ciphertext_hex) = encrypted
        .split_once(':')
        .expect("encrypted payload must contain nonce and ciphertext");
    let mut ciphertext = hex::decode(ciphertext_hex).expect("ciphertext must be valid hex");

    // Flip an authenticated ciphertext bit. Mutating a textual hex digit can be
    // accidentally equivalent when only letter casing changes (e.g. 'a' -> 'A').
    ciphertext[0] ^= 0x01;
    let tampered = format!("{nonce_hex}:{}", hex::encode(ciphertext));

    let result = CryptoService::decrypt(&tampered);
    assert!(
        result.is_err(),
        "decryption of tampered ciphertext must fail authenticated tag check"
    );
}

#[test]
#[serial]
fn test_missing_encryption_key_fails_closed() {
    let previous = std::env::var_os("ENCRYPTION_KEY");
    std::env::remove_var("ENCRYPTION_KEY");

    let result = CryptoService::encrypt("must not use a fallback key");
    assert!(
        result.is_err(),
        "encryption must fail when ENCRYPTION_KEY is missing"
    );

    if let Some(value) = previous {
        std::env::set_var("ENCRYPTION_KEY", value);
    }
}
