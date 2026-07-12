use f2s_application::export::publish_snapshot::PublishSnapshot;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasInputEntry<'a> {
    pub logical_path: &'a str,
    pub sha256: &'a str,
    pub width: u32,
    pub height: u32,
    pub color_space: &'static str,
    pub alpha: &'static str,
    pub slot_id: &'a str,
    pub attachment_id: &'a str,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasInputManifest<'a> {
    pub schema_version: &'static str,
    pub editor_version: &'static str,
    pub note: &'static str,
    pub entries: Vec<AtlasInputEntry<'a>>,
}
pub fn atlas_input_bytes(snapshot: &PublishSnapshot) -> Result<Vec<u8>, String> {
    let mut refs = snapshot.attachments.iter().collect::<Vec<_>>();
    refs.sort_by(|a, b| a.logical_png_path.cmp(&b.logical_png_path));
    let manifest = AtlasInputManifest {
        schema_version: "1.0.0",
        editor_version: "4.2.43",
        note: "Open atlas packing input only. This is not a Spine .atlas file.",
        entries: refs
            .into_iter()
            .map(|v| AtlasInputEntry {
                logical_path: &v.logical_png_path,
                sha256: &v.source_sha256,
                width: v.width,
                height: v.height,
                color_space: "sRGB",
                alpha: "straight",
                slot_id: &v.slot_id,
                attachment_id: &v.attachment_id,
            })
            .collect(),
    };
    f2s_domain::canonical::canonical_bytes(&manifest).map_err(|e| e.to_string())
}
