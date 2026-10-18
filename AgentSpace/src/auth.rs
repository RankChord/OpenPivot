//! HTTP 身份边界。管理 API 使用管理凭证；MCP 只接受动态 Agent 凭证。
//!
//! 凭证旋转/停用后，每个新请求都查询数据库，因此旧身份立即失效。
//! 前端的 actor 选择仅存在于已授权的管理 API，MCP 参数不能冒充其他 Agent。
use crate::{config::Config, store::Store};
use axum::{
    Json,
    body::Body,
    extract::State,
    http::{
        Request, StatusCode,
        header::{AUTHORIZATION, HOST, ORIGIN},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Principal(pub String);

#[derive(Clone)]
pub struct AuthState {
    pub store: Store,
    pub config: Arc<Config>,
}

fn denied(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error":message}))).into_response()
}

/// 先验证访问来源，再验证身份。静态页面不需要凭证，数据接口需要。
fn transport_guard(
    request: &Request<Body>,
    config: &Config,
) -> Result<(), (StatusCode, &'static str)> {
    let headers = request.headers();
    let Some(host) = headers.get(HOST).and_then(|v| v.to_str().ok()) else {
        return Err((StatusCode::BAD_REQUEST, "invalid Host header"));
    };
    if !host_allowed(host, config.allowed_hosts.as_deref()) {
        return Err((StatusCode::FORBIDDEN, "Host is not allowed"));
    }
    if let Some(origin) = headers.get(ORIGIN)
        && !origin
            .to_str()
            .ok()
            .is_some_and(|o| origin_allowed(o, config.allowed_origins.as_deref()))
    {
        return Err((StatusCode::FORBIDDEN, "Origin is not allowed"));
    }
    Ok(())
}

fn bearer(request: &Request<Body>) -> Option<&str> {
    let value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

pub async fn protect_admin(
    State(state): State<AuthState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if let Err((status, message)) = transport_guard(&request, &state.config) {
        return denied(status, message);
    }
    if bearer(&request) != Some(state.config.admin_token.as_str()) {
        return denied(
            StatusCode::UNAUTHORIZED,
            "administrator credential required",
        );
    }
    next.run(request).await
}

pub async fn protect_mcp(
    State(state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    if let Err((status, message)) = transport_guard(&request, &state.config) {
        return denied(status, message);
    }
    let Some(token) = bearer(&request).map(str::to_owned) else {
        return denied(StatusCode::UNAUTHORIZED, "Agent bearer token required");
    };
    if token == state.config.admin_token {
        return denied(StatusCode::UNAUTHORIZED, "use an Agent credential for MCP");
    }
    let store = state.store.clone();
    let identity = tokio::task::spawn_blocking(move || store.authenticate(&token)).await;
    match identity {
        Ok(Ok(Some(agent_id))) => {
            request.extensions_mut().insert(Principal(agent_id));
            next.run(request).await
        }
        Ok(Ok(None)) => denied(
            StatusCode::UNAUTHORIZED,
            "invalid or disabled Agent credential",
        ),
        _ => denied(
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not validate Agent credential",
        ),
    }
}

fn is_loopback_host(authority: &str) -> bool {
    if authority.contains('@') {
        return false;
    }
    let Ok(parsed) = authority.parse::<http::uri::Authority>() else {
        return false;
    };
    let Some(suffix) = authority.strip_prefix(parsed.host()) else {
        return false;
    };
    if !suffix.is_empty()
        && suffix
            .strip_prefix(':')
            .is_none_or(|port| port.parse::<u16>().is_err())
    {
        return false;
    }
    let host = parsed.host().trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

pub fn host_allowed(host: &str, configured: Option<&[String]>) -> bool {
    configured.map_or_else(
        || is_loopback_host(host),
        |entries| entries.iter().any(|entry| entry.eq_ignore_ascii_case(host)),
    )
}
pub fn origin_allowed(origin: &str, configured: Option<&[String]>) -> bool {
    if let Some(entries) = configured {
        return entries
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(origin));
    }
    let Some((scheme, authority)) = origin.split_once("://") else {
        return false;
    };
    matches!(scheme, "http" | "https")
        && !authority.chars().any(|c| matches!(c, '/' | '?' | '#'))
        && is_loopback_host(authority)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_allowlists_reject_lookalike_hosts_and_origins() {
        assert!(host_allowed("127.0.0.1:8080", None));
        for invalid in [
            "127.0.0.1.attacker.test",
            "[::1]attacker.test",
            "user@localhost",
        ] {
            assert!(!host_allowed(invalid, None));
        }
        assert!(origin_allowed("http://localhost:8080", None));
        assert!(!origin_allowed("http://localhost/path", None));
        assert!(!origin_allowed("https://localhost.attacker.test", None));
        let allowed = vec!["https://ui.example.test".into()];
        assert!(origin_allowed("https://ui.example.test", Some(&allowed)));
        assert!(!origin_allowed(
            "https://ui.example.test:444",
            Some(&allowed)
        ));
    }
}
