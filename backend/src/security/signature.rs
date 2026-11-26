use crate::error::{AppError, AppResult};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Sign data with HMAC-SHA256
pub fn sign(data: &str, secret: &secrecy::Secret<String>) -> AppResult<String> {
    let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
        .map_err(|e| AppError::EncryptionError(format!("HMAC key error: {}", e)))?;

    mac.update(data.as_bytes());
    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;

    /// Verify HMAC-SHA256 signature
    fn verify(data: &str, signature: &str, secret: &secrecy::Secret<String>) -> AppResult<bool> {
        let Ok(sig_bytes) = hex::decode(signature) else {
            return Ok(false);
        };
        let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
            .map_err(|e| AppError::EncryptionError(format!("HMAC key error: {}", e)))?;
        mac.update(data.as_bytes());
        // Constant-time comparison
        Ok(mac.verify_slice(&sig_bytes).is_ok())
    }

    #[test]
    fn test_sign_verify() {
        let secret = Secret::new("test_secret_key_12345".to_string());
        let data = r#"{"message":"hello"}"#;

        let signature = sign(data, &secret).unwrap();
        assert!(verify(data, &signature, &secret).unwrap());

        // Wrong signature should fail
        assert!(!verify(data, "wrong_signature", &secret).unwrap());

        // Modified data should fail
        let modified_data = r#"{"message":"bye"}"#;
        assert!(!verify(modified_data, &signature, &secret).unwrap());
    }
}
