use axum::{
    body::Bytes,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::time::SystemTime;

use crate::config;
use crate::note::{CreateRequest, generate_id};
use crate::store;

use super::{CreateResponse, MetaResponse, NoteMeta, NoteResponse};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Deserialize)]
pub struct NoteParams {
    id: String,
}

/// Note IDs are bs62-encoded random bytes generated server-side, i.e. plain
/// ASCII alphanumerics. Anything else can never exist, and rejecting it here
/// keeps arbitrary cache keys (e.g. other apps on a shared instance without
/// CACHE_PREFIX) out of reach.
fn valid_id(id: &str) -> bool {
    let max = (*config::ID_LENGTH as usize) * 2;
    !id.is_empty() && id.len() <= max && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Pure deserialization + validation for the create endpoint, split out so it
/// can be unit-tested and fuzzed without HTTP or a live cache.
pub fn parse_create(body: &[u8]) -> Result<(NoteMeta, Vec<u8>), &'static str> {
    let req: CreateRequest = rmp_serde::from_slice(body).map_err(|_| "Invalid msgpack")?;

    let mut meta = req.meta;

    if meta.views.is_none() && meta.expiration.is_none() {
        return Err("At least views or expiration must be set");
    }

    if meta.extra.len() > *config::EXTRA_SIZE_LIMIT {
        return Err("Extra data too large");
    }

    if !*config::ALLOW_ADVANCED {
        meta.views = Some(1);
        meta.expiration = None;
    }

    if let Some(v) = meta.views
        && (v > *config::MAX_VIEWS || v < 1)
    {
        return Err("Invalid views");
    }

    if let Some(e) = meta.expiration
        && (e > *config::MAX_EXPIRATION || e < 1)
    {
        return Err("Invalid expiration");
    }

    Ok((meta, req.data))
}

pub async fn create(body: Bytes) -> Response {
    let (meta, data) = match parse_create(&body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    let expiration_ts = meta.expiration.map(|e| now() + (e as u64 * 60));

    let id = generate_id();
    let views = meta.views.map(|v| v as i64);

    match store::set(&id, &data, views, expiration_ts, &meta.extra) {
        Ok(_) => {
            let resp = CreateResponse { id };
            let bytes = rmp_serde::to_vec_named(&resp).unwrap();
            (StatusCode::OK, Bytes::from(bytes)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn preview(Path(NoteParams { id }): Path<NoteParams>) -> Response {
    if !valid_id(&id) {
        return (StatusCode::NOT_FOUND).into_response();
    }
    match store::get_meta(&id) {
        Ok(Some((views, expiration, extra))) => {
            let meta = NoteMeta {
                views: views.map(|v| v.max(0) as u32),
                expiration: expiration.map(|e| e as u32),
                extra,
            };
            let resp = MetaResponse { meta };
            let bytes = rmp_serde::to_vec_named(&resp).unwrap();
            (StatusCode::OK, Bytes::from(bytes)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn view(Path(NoteParams { id }): Path<NoteParams>) -> Response {
    if !valid_id(&id) {
        return (StatusCode::NOT_FOUND).into_response();
    }

    let (views, expiration, extra) = match store::get_meta(&id) {
        Ok(Some(v)) => v,
        _ => return (StatusCode::NOT_FOUND).into_response(),
    };

    if views.is_some() {
        // Atomic check-and-decrement: concurrent requests racing for the last
        // view cannot all be served.
        let consumed = match store::consume(&id) {
            Ok(Some(v)) => v,
            _ => return (StatusCode::NOT_FOUND).into_response(),
        };

        let meta = NoteMeta {
            views: Some(consumed.remaining as u32),
            expiration: consumed.expiration.map(|e| e as u32),
            extra: consumed.extra,
        };
        let resp = NoteResponse {
            meta,
            data: consumed.data,
        };
        let bytes = rmp_serde::to_vec_named(&resp).unwrap();
        (StatusCode::OK, Bytes::from(bytes)).into_response()
    } else {
        let data = match store::get_data(&id) {
            Ok(Some(d)) => d,
            _ => return (StatusCode::NOT_FOUND).into_response(),
        };

        let meta = NoteMeta {
            views: None,
            expiration: expiration.map(|e| e as u32),
            extra,
        };
        let resp = NoteResponse { meta, data };
        let bytes = rmp_serde::to_vec_named(&resp).unwrap();
        (StatusCode::OK, Bytes::from(bytes)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{CreateRequest, NoteMeta};

    fn body(views: Option<u32>, expiration: Option<u32>, extra: Vec<u8>) -> Vec<u8> {
        rmp_serde::to_vec_named(&CreateRequest {
            meta: NoteMeta {
                views,
                expiration,
                extra,
            },
            data: b"payload".to_vec(),
        })
        .unwrap()
    }

    #[test]
    fn valid_id_accepts_generated_shape() {
        assert!(valid_id("aB09"));
        assert!(valid_id(&"a".repeat(*config::ID_LENGTH as usize)));
        assert!(valid_id(&"Z".repeat((*config::ID_LENGTH as usize) * 2)));
    }

    #[test]
    fn valid_id_rejects_foreign_keys() {
        assert!(!valid_id(""));
        assert!(!valid_id("other:app:key"));
        assert!(!valid_id("../etc/passwd"));
        assert!(!valid_id("a b"));
        assert!(!valid_id("id.with.dots"));
        assert!(!valid_id("unicode-äöü"));
        assert!(!valid_id(
            &"a".repeat((*config::ID_LENGTH as usize) * 2 + 1)
        ));
    }

    #[test]
    fn parse_create_rejects_garbage() {
        assert_eq!(parse_create(b""), Err("Invalid msgpack"));
        assert_eq!(
            parse_create(b"\xc1\xff\xfe not msgpack"),
            Err("Invalid msgpack")
        );
        // Valid msgpack, wrong shape.
        assert_eq!(
            parse_create(&rmp_serde::to_vec_named(&42u8).unwrap()),
            Err("Invalid msgpack")
        );
    }

    #[test]
    fn parse_create_requires_a_policy() {
        let b = body(None, None, vec![]);
        assert_eq!(
            parse_create(&b),
            Err("At least views or expiration must be set")
        );
    }

    #[test]
    fn parse_create_enforces_bounds() {
        assert_eq!(
            parse_create(&body(Some(0), None, vec![])),
            Err("Invalid views")
        );
        assert_eq!(
            parse_create(&body(Some(*config::MAX_VIEWS + 1), None, vec![])),
            Err("Invalid views")
        );
        assert_eq!(
            parse_create(&body(None, Some(0), vec![])),
            Err("Invalid expiration")
        );
        assert_eq!(
            parse_create(&body(None, Some(*config::MAX_EXPIRATION + 1), vec![])),
            Err("Invalid expiration")
        );
        assert_eq!(
            parse_create(&body(
                Some(1),
                None,
                vec![0u8; *config::EXTRA_SIZE_LIMIT + 1]
            )),
            Err("Extra data too large")
        );
    }

    #[test]
    fn parse_create_accepts_boundaries() {
        // Exact limits must be accepted (">", not ">=").
        let (meta, _) = parse_create(&body(
            Some(*config::MAX_VIEWS),
            Some(*config::MAX_EXPIRATION),
            vec![0u8; *config::EXTRA_SIZE_LIMIT],
        ))
        .unwrap();
        assert_eq!(meta.views, Some(*config::MAX_VIEWS));
        assert_eq!(meta.expiration, Some(*config::MAX_EXPIRATION));
    }

    #[test]
    fn parse_create_accepts_valid_requests() {
        for (v, e) in [(Some(1), None), (None, Some(1)), (Some(5), Some(30))] {
            let (meta, data) = parse_create(&body(v, e, vec![])).unwrap();
            assert_eq!(meta.views, v);
            assert_eq!(meta.expiration, e);
            assert_eq!(data, b"payload");
        }
    }
}
