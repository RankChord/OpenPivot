use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: i64,
    pub exp: i64,
    pub iat: i64,
}

pub fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

pub fn create_access_token(
    user_id: i64,
    jwt_secret: &str,
    ttl_minutes: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = OffsetDateTime::now_utc();
    let expires_at = now + time::Duration::minutes(ttl_minutes as i64);

    let claims = AccessTokenClaims {
        sub: user_id,
        iat: now.unix_timestamp(),
        exp: expires_at.unix_timestamp(),
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

pub fn verify_access_token(
    token: &str,
    jwt_secret: &str,
) -> Result<AccessTokenClaims, jsonwebtoken::errors::Error> {
    let token_data = jsonwebtoken::decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}