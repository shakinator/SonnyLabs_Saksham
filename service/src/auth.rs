use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use chrono::{Utc, TimeZone};
use std::sync::OnceLock;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect},
};

use axum_extra::{
    extract::cookie::{CookieJar, Cookie, SameSite},
};

use tracing::log::*;


fn jwt_secret() -> &'static str {
    static JWT_SECRET: OnceLock<String> = OnceLock::new();
    JWT_SECRET.get_or_init(|| {
        std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable variable must be set")
    })
}

fn jwt_encoding_key() -> &'static EncodingKey {
    static JWT_ENCODING_KEY: OnceLock<EncodingKey> = OnceLock::new();
    JWT_ENCODING_KEY.get_or_init(|| {
        EncodingKey::from_secret(jwt_secret().as_ref())
    })
}
fn jwt_decoding_key() -> &'static DecodingKey {
    static JWT_DECODING_KEY: OnceLock<DecodingKey> = OnceLock::new();
    JWT_DECODING_KEY.get_or_init(|| {
        DecodingKey::from_secret(jwt_secret().as_ref())
    })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Audience {
    #[serde(rename="web")]
    Web,

    #[serde(rename="api")]
    Api,
}

impl std::fmt::Display for Audience {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Audience::Web => write!(f, "web"),
            Audience::Api => write!(f, "api"),
        }
    }
}

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: i64, // account_id
    pub exp: u64,
    pub aud: Vec<Audience>,
}

impl Claims {
    pub fn new_from_session(session: &Session) -> Claims {
        Claims {
            sub: session.account_id,
            exp: jsonwebtoken::get_current_timestamp() + 10*60,
            aud: vec![Audience::Web],
        }
    }

    pub fn to_token(&self) -> Option<String> {
        let token = encode(
            &Header::default(),
            &self,
            jwt_encoding_key(),
        )
        .inspect_err(|err| error!("{:?}", err))
        .ok();

        token
    }

    pub fn from_token(
        token: &str,
        audience: &[Audience],
    )
    -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::default();
        validation.set_audience(audience);

        let decoded = decode::<Claims>(
            token, 
            jwt_decoding_key(),
            &validation,
        )?;
        Ok(decoded.claims)
    }

    pub fn to_cookie(&self, headers: HeaderMap) -> Option<Cookie<'static>> {
        let domain = headers.get("host")
            .expect("HOST header must be set")
            .to_str().unwrap()
            .split(':').next().unwrap();
        let secure = headers.get("x-forwarded-proto")
            .map(|proto| proto == "https")
            .unwrap_or(false);

        let token = self.to_token()?;

        Some(
            Cookie::build(("claims", token))
                .domain(domain.to_string())
                .path("/")
                .same_site(SameSite::Strict)
                .secure(secure)
                .http_only(true)
                .build()
        )
    }

    pub fn get_cookie(cookies: &CookieJar) -> Option<&str> {
        Some(
            cookies
                .get("claims")?
                .value()
        )
    }
}


pub struct Session {
    pub account_id: i64,
    pub token: String,
}

impl Session {
    pub async fn new(account_id: i64, pool: sqlx::PgPool) -> Session {
        let session = sqlx::query! {
            "
            INSERT INTO
                session(account_id, token, expires_at, login_info)
            VALUES ($1, gen_random_uuid()::TEXT, $2, $3)
            RETURNING token
            ",
            account_id,
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            "<NONE>"
        }
        .fetch_one(&pool)
        .await
        .unwrap();

        Session {
            account_id,
            token: session.token,
        }
    }

    pub async fn get_from_token(token: &str, pool: sqlx::PgPool) -> Option<Session> {
        let session = sqlx::query! {
            "SELECT account_id FROM session WHERE token = $1", token
        }
        .fetch_optional(&pool)
        .await
        .unwrap()?;

        Some(
            Session {
                account_id: session.account_id,
                token: token.into(),
            }
        )
    }

    pub fn to_cookie(&self, headers: HeaderMap) -> Cookie<'static> {
        let domain = headers.get("host")
            .expect("HOST header must be set")
            .to_str().unwrap()
            .split(':').next().unwrap();
        let secure = headers.get("x-forwarded-proto")
            .map(|proto| proto == "https")
            .unwrap_or(false);

        Cookie::build(("session", self.token.clone()))
            .domain(domain.to_string())
            .path("/")
            .same_site(SameSite::Strict)
            .secure(secure)
            .http_only(true)
            .build()
    }

    pub fn get_cookie(cookies: &CookieJar) -> Option<&str> {
        Some(cookies.get("session")?.value())
    }
}


pub mod web {
    use super::*;

    pub async fn middleware(
        cookies: CookieJar,
        headers: HeaderMap,
        State(pool): State<sqlx::PgPool>,
        mut request: Request,
        next: Next,
    ) -> Result<(CookieJar, impl IntoResponse), Redirect>
    {
        macro_rules! redirect {
            () => {
                Redirect::to(
                    &format!(
                        "/login?next={}",
                        request.uri()
                            .path_and_query()
                            .unwrap()
                            .as_str()
                    )
                )
            };
        }


        let Some(claims_token) = Claims::get_cookie(&cookies) else {
            return Err(redirect!());
        };
        
        let (cookies, claims) = match Claims::from_token(claims_token, &[Audience::Web]) {
            Ok(claims) => (cookies, claims),
            Err(err) => {
                // If JWT expired, issue a new one
                use jsonwebtoken::errors::ErrorKind;
                if err.kind() == &ErrorKind::ExpiredSignature {

                    // Try get session from session cookie
                    let Some(session_token) = Session::get_cookie(&cookies) else {
                        return Err(redirect!());
                    };
                    let Some(session) = Session::get_from_token(session_token, pool).await else {
                        return Err(redirect!());
                    };

                    // Try make claims from session
                    let claims = Claims::new_from_session(&session);

                    // Add claims cookie
                    let token_cookie = claims.to_cookie(headers).unwrap();
                    let cookies = cookies.add(token_cookie);

                    (cookies, claims)
                } else {
                    return Err(redirect!());
                }
            }
        };

        request.extensions_mut().insert(claims);

        Ok((cookies, next.run(request).await))
    }
}

pub mod api {
    use super::*;
    use axum_extra::{
        TypedHeader,
        headers::{Authorization, authorization::Bearer},
    };

    pub async fn middleware(
        TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
        State(pool): State<sqlx::PgPool>,
        mut request: Request,
        next: Next,
    ) -> Result<impl IntoResponse, StatusCode>
    {
        // API tokens prefixed with `api::` in db.
        let token = format!("api::{}", bearer.token());

        let session = sqlx::query! {
            "SELECT account_id
            FROM session
            WHERE token = $1",
            token
        }
        .fetch_optional(&pool)
        .await
        .unwrap();

        match session {
            Some(row) => {
                request.extensions_mut().insert(
                    Claims {
                        sub: row.account_id,
                        exp: 0,
                        aud: vec![Audience::Api],
                    }
                );
            },
            None => {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }

        Ok(next.run(request).await)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_jwt_encode_decode() {
        std::env::set_var("JWT_SECRET", "dev jwt secret"); // ensure JWT_SECRET is set

        let claim = Claims {
            sub: 123,
            exp: jsonwebtoken::get_current_timestamp() + 10*60,
            aud: vec![Audience::Web]
        };

        let token = claim.to_token().unwrap();
        println!("token: {}", token);
        let dec_claim = Claims::from_token(&token, &[Audience::Web]).unwrap();

        assert_eq!(dec_claim.sub, claim.sub);
        assert_eq!(dec_claim.exp, claim.exp);
        assert_eq!(dec_claim.aud, claim.aud);
    }
}
