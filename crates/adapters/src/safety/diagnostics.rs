use serde_json::Value;

const SECRET_KEYS: [&str; 8] = [
    "token",
    "authorization",
    "password",
    "secret",
    "credential",
    "license",
    "hostname",
    "username",
];
pub fn redact(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let lower = key.to_ascii_lowercase();
                    if SECRET_KEYS.iter().any(|needle| lower.contains(needle)) {
                        (key.clone(), Value::String("[REDACTED]".into()))
                    } else {
                        (key.clone(), redact(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        Value::String(text) => {
            let looks_absolute = text.len() > 3
                && text.as_bytes().get(1) == Some(&b':')
                && matches!(text.as_bytes().get(2), Some(b'\\') | Some(b'/'));
            if looks_absolute || text.starts_with("\\\\") {
                Value::String("[LOCAL_PATH_REDACTED]".into())
            } else if text.len() > 4096 {
                Value::String("[OVERSIZE_TEXT_REMOVED]".into())
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}
pub fn diagnostic_bundle_is_safe(value: &Value) -> bool {
    let serialized = serde_json::to_string(value)
        .unwrap_or_default()
        .to_ascii_lowercase();
    ![
        "bearer ",
        "token=",
        "c:\\users\\",
        "license-key",
        "private_prompt",
    ]
    .iter()
    .any(|needle| serialized.contains(needle))
}
