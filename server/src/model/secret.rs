use std::array;

use axum::extract::{FromRef, FromRequestParts};
use bcrypt_pbkdf::bcrypt_pbkdf;
use http::{HeaderValue, StatusCode, header::AUTHORIZATION, request::Parts};
use rusqlite::ToSql;
use sha2::{Digest as _, Sha256};

use crate::web_handlers::{ApiError, MooseWebData};

const PASS_MIN_LEN: usize = 16;
pub const PASS_MAX_LEN: usize = 64;

#[derive(Clone)]
pub struct Secret<const N: usize>(pub [u8; N]);

impl<const N: usize> Default for Secret<N> {
    fn default() -> Self {
        Self(array::from_fn(|_| rand::random()))
    }
}

pub type CookieSecret = Secret<64>;

const COOKIE_SALT: &[u8] = br####";o'"#|`=8kZhT:DWK\x4#<:&C.#Rzdd@"####;
const PBKDF_ROUNDS: u32 = 8u32;

impl TryFrom<&str> for CookieSecret {
    type Error = bcrypt_pbkdf::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut new = Secret([0; 64]);
        bcrypt_pbkdf(s, COOKIE_SALT, PBKDF_ROUNDS, &mut new.0)?;
        Ok(new)
    }
}

impl From<CookieSecret> for tower_cookies::Key {
    fn from(value: CookieSecret) -> Self {
        tower_cookies::Key::from(&value.0)
    }
}

pub type InviteSecret = Secret<32>;

#[derive(Debug, thiserror::Error)]
pub enum InviteSecretError {
    #[error("Invite secret must be at least {PASS_MIN_LEN} bytes.")]
    Short,
    #[error("Invite secret should be unambiguous in an HTTP header (ASCII 33..127).")]
    AsciiPrint,
}

impl InviteSecret {
    pub fn new<B: AsRef<[u8]>>(b: B) -> Self {
        Self(Sha256::digest(b).into())
    }

    pub fn new_invite_hash(i: &str) -> Result<Self, InviteSecretError> {
        if i.len() < PASS_MIN_LEN {
            return Err(InviteSecretError::Short);
        }
        i.bytes()
            .all(|c| c.is_ascii_graphic())
            .ok_or(InviteSecretError::AsciiPrint)
            .map(|_| Self::new(i))
    }

    pub fn cmp_undigested<B: AsRef<[u8]>>(&self, b: B) -> bool {
        let dgst: [u8; 32] = Sha256::digest(b).into();
        self.0 == dgst
    }
}

impl TryFrom<&str> for InviteSecret {
    type Error = &'static str;

    fn try_from(pass: &str) -> Result<Self, Self::Error> {
        if matches!(pass.len(), PASS_MIN_LEN..=PASS_MAX_LEN) {
            Ok(Self(Sha256::digest(pass).into()))
        } else {
            Err("Password must be between 16 and 64 characters.")
        }
    }
}

impl ToSql for InviteSecret {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_vec()))
    }
}

/// Marker struct for requiring validating invite code
pub struct Invited;

const SCHEME_PREFIX_LEN: usize = b"Bearer ".len();

fn get_token(hv: &HeaderValue) -> Option<&[u8]> {
    // we don't really care what the prefix is, honestly.
    // it just has to be somewhere and same length as BEARER
    hv.as_bytes()
        .split_at_checked(SCHEME_PREFIX_LEN)
        .map(|(_, rest)| rest)
}

impl<S> FromRequestParts<S> for Invited
where
    MooseWebData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let webdata = MooseWebData::from_ref(state);
        if parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|tok| get_token(tok))
            .map(|v| webdata.invite_hash.cmp_undigested(v))
            .unwrap_or_default()
        {
            Ok(Invited)
        } else {
            Err(ApiError::new_with_status(
                StatusCode::UNAUTHORIZED,
                "Not a valid invite code",
            ))
        }
    }
}
