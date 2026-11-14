use bs62;
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NoteMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub views: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<u32>,
    #[serde(default)]
    pub extra: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CreateRequest {
    pub meta: NoteMeta,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateResponse {
    pub id: String,
}

#[derive(Serialize, Deserialize)]
pub struct MetaResponse {
    pub meta: NoteMeta,
}

#[derive(Serialize, Deserialize)]
pub struct NoteResponse {
    pub meta: NoteMeta,
    pub data: Vec<u8>,
}

pub fn generate_id() -> String {
    let mut result = "".to_owned();
    let mut id: [u8; 1] = [0; 1];
    let sr = ring::rand::SystemRandom::new();

    for _ in 0..*config::ID_LENGTH {
        sr.fill(&mut id).expect("system RNG failed");
        result.push_str(&bs62::encode_data(&id));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_match_valid_id_shape() {
        for _ in 0..64 {
            let id = generate_id();
            assert!(id.len() >= *config::ID_LENGTH as usize);
            assert!(id.len() <= (*config::ID_LENGTH as usize) * 2);
            assert!(id.bytes().all(|b| b.is_ascii_alphanumeric()));
        }
    }

    #[test]
    fn generated_ids_do_not_collide() {
        let ids: std::collections::HashSet<String> = (0..256).map(|_| generate_id()).collect();
        assert_eq!(ids.len(), 256);
    }

    #[test]
    fn note_meta_msgpack_roundtrip() {
        let meta = NoteMeta {
            views: Some(3),
            expiration: Some(60),
            extra: vec![1, 2, 3],
        };
        let bytes = rmp_serde::to_vec_named(&meta).unwrap();
        let back: NoteMeta = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back.views, Some(3));
        assert_eq!(back.expiration, Some(60));
        assert_eq!(back.extra, vec![1, 2, 3]);
    }

    #[test]
    fn note_meta_missing_extra_defaults_empty() {
        let bytes = rmp_serde::to_vec_named(&serde_json::json!({ "views": 1 })).unwrap();
        let meta: NoteMeta = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(meta.views, Some(1));
        assert!(meta.extra.is_empty());
    }
}
