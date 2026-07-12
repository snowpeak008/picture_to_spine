use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub fn canonicalize(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize).collect()),
        Value::Object(items) => {
            let mut keys: Vec<_> = items.into_iter().collect();
            keys.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(Map::from_iter(
                keys.into_iter()
                    .map(|(key, value)| (key, canonicalize(value))),
            ))
        }
        other => other,
    }
}

pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&canonicalize(serde_json::to_value(value)?))
}
pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let digest = Sha256::digest(canonical_bytes(value)?);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn sha256_bytes(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
