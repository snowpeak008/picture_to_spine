use f2s_domain::layers::{Layer, LayerRole, LayerSet};
pub fn add_layer(set: &mut LayerSet, layer: Layer) -> Result<(), String> {
    if set.approval_state == "APPROVED" {
        return Err("approved layer set must be revised".into());
    }
    set.layers.push(layer);
    set.revision += 1;
    set.validate()
}

pub fn remove_optional_layer(set: &mut LayerSet, layer_id: &str) -> Result<(), String> {
    if set.approval_state == "APPROVED" {
        return Err("approved layer set must be revised".into());
    }
    let index = set
        .layers
        .iter()
        .position(|layer| layer.layer_id == layer_id)
        .ok_or("layer not found")?;
    if set.layers[index].role != LayerRole::Accessory {
        return Err("V1 required semantic layers cannot be deleted".into());
    }
    set.layers.remove(index);
    set.revision = set
        .revision
        .checked_add(1)
        .ok_or("layer revision overflow")?;
    set.validate()
}

pub fn reorder_layers(set: &mut LayerSet, ordered_ids: &[String]) -> Result<(), String> {
    if set.approval_state == "APPROVED" {
        return Err("approved layer set must be revised".into());
    }
    if ordered_ids.len() != set.layers.len() {
        return Err("reorder must contain every layer exactly once".into());
    }
    let mut reordered = Vec::with_capacity(set.layers.len());
    for id in ordered_ids {
        let layer = set
            .layers
            .iter()
            .find(|layer| &layer.layer_id == id)
            .cloned()
            .ok_or("reorder contains an unknown layer")?;
        if reordered
            .iter()
            .any(|existing: &Layer| existing.layer_id == layer.layer_id)
        {
            return Err("reorder contains a duplicate layer".into());
        }
        reordered.push(layer);
    }
    set.layers = reordered;
    set.revision = set
        .revision
        .checked_add(1)
        .ok_or("layer revision overflow")?;
    set.validate()
}

pub fn replace_layer_attachment(
    set: &mut LayerSet,
    layer_id: &str,
    attachment_sha256: &str,
    mask_sha256: &str,
) -> Result<(), String> {
    if set.approval_state == "APPROVED" {
        return Err("approved layer set must be revised before replacing pixels".into());
    }
    let valid_hash = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    if !valid_hash(attachment_sha256) || !valid_hash(mask_sha256) {
        return Err("replacement attachment and mask require lowercase SHA-256".into());
    }
    let layer = set
        .layers
        .iter_mut()
        .find(|layer| layer.layer_id == layer_id)
        .ok_or("replacement layer not found")?;
    if layer.attachment_sha256 == attachment_sha256 && layer.mask_sha256 == mask_sha256 {
        return Err("replacement pixels are unchanged".into());
    }
    layer.attachment_sha256 = attachment_sha256.into();
    layer.mask_sha256 = mask_sha256.into();
    layer.approved = false;
    set.revision = set
        .revision
        .checked_add(1)
        .ok_or("layer revision overflow")?;
    set.validate()
}
