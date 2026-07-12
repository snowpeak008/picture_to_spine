use serde_json::Value;
pub fn migrate_copy_on_write(mut value: Value, from: u32, to: u32) -> Result<Value, String> {
    if from > to {
        return Err("downgrade migration forbidden".into());
    }
    for version in from + 1..=to {
        value["schemaVersion"] = Value::String(format!("{version}.0.0"));
    }
    Ok(value)
}
