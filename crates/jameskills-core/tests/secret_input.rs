use jameskills_core::{
    AppError,
    ports::{MAX_SECRET_INPUT_BYTES, SecretInput},
};
use zeroize::ZeroizeOnDrop;

fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

#[test]
fn secret_input_borrows_exact_bytes_but_never_formats_them() {
    let secret = SecretInput::new("passphrase-á".as_bytes().to_vec()).unwrap();
    assert_eq!(secret.expose_secret(), "passphrase-á".as_bytes());
    assert_eq!(format!("{secret:?}"), "[REDACTED]");
    assert_zeroize_on_drop::<SecretInput>();
}

#[test]
fn secret_input_rejects_empty_and_oversized_values_with_redacted_errors() {
    let empty = SecretInput::new(Vec::new()).unwrap_err();
    let marker = "sensitive".repeat(MAX_SECRET_INPUT_BYTES / "sensitive".len() + 1);
    let oversized = SecretInput::new(marker.as_bytes().to_vec()).unwrap_err();

    assert!(matches!(empty, AppError::Validation(_)));
    assert!(matches!(oversized, AppError::Validation(_)));
    assert!(!oversized.to_string().contains(&marker));
    assert!(!format!("{oversized:?}").contains(&marker));
}
