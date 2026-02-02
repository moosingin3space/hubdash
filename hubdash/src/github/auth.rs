//! GitHub App authentication using JWT.
//!
//! GitHub Apps authenticate using RS256-signed JWTs. The JWT contains:
//! - `iat`: Issued at (60 seconds in the past to account for clock drift)
//! - `exp`: Expiration (max 10 minutes from now)
//! - `iss`: Issuer (the app's client ID or app ID)

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;
use thiserror::Error;

/// Credentials for authenticating as a GitHub App.
#[derive(Clone)]
pub struct AppCredentials {
    app_id: String,
    encoding_key: EncodingKey,
}

/// Errors that can occur when working with app credentials.
#[derive(Debug, Error)]
pub enum AppCredentialsError {
    #[error("invalid private key: {0}")]
    InvalidPrivateKey(jsonwebtoken::errors::Error),
    #[error("failed to encode JWT: {0}")]
    JwtEncoding(#[from] jsonwebtoken::errors::Error),
}

/// JWT claims for GitHub App authentication.
#[derive(Serialize)]
struct Claims {
    /// Issued at (Unix timestamp).
    iat: i64,
    /// Expiration (Unix timestamp).
    exp: i64,
    /// Issuer (app ID).
    iss: String,
}

impl AppCredentials {
    /// Creates new app credentials from an app ID and PEM-encoded private key.
    ///
    /// The private key should be the RSA private key downloaded from GitHub
    /// when creating the app, in PEM format.
    pub fn new(
        app_id: impl Into<String>,
        private_key_pem: &[u8],
    ) -> Result<Self, AppCredentialsError> {
        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem)
            .map_err(AppCredentialsError::InvalidPrivateKey)?;

        Ok(Self {
            app_id: app_id.into(),
            encoding_key,
        })
    }

    /// Generates a JWT for authenticating as the GitHub App.
    ///
    /// The JWT is valid for 10 minutes and uses RS256 signing.
    /// Per GitHub's documentation, `iat` is set 60 seconds in the past
    /// to account for clock drift.
    pub fn generate_jwt(&self) -> Result<String, AppCredentialsError> {
        let now = jiff::Timestamp::now();
        let iat = now.as_second() - 60;
        let exp = now.as_second() + 600;

        let claims = Claims {
            iat,
            exp,
            iss: self.app_id.clone(),
        };

        let header = Header::new(Algorithm::RS256);
        encode(&header, &claims, &self.encoding_key).map_err(AppCredentialsError::JwtEncoding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_private_key() -> Vec<u8> {
        use rsa::RsaPrivateKey;
        use rsa::pkcs8::EncodePrivateKey;

        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap()
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn test_credentials_creation() {
        let pem = generate_test_private_key();
        let creds = AppCredentials::new("123456", &pem);
        assert!(creds.is_ok());
    }

    #[test]
    fn test_jwt_generation() {
        let pem = generate_test_private_key();
        let creds = AppCredentials::new("123456", &pem).unwrap();
        let jwt = creds.generate_jwt();
        assert!(jwt.is_ok());

        let token = jwt.unwrap();
        assert!(token.starts_with("eyJ"));
        assert_eq!(token.matches('.').count(), 2);
    }

    #[test]
    fn test_invalid_private_key() {
        let result = AppCredentials::new("123456", b"not a valid key");
        assert!(result.is_err());
    }
}
