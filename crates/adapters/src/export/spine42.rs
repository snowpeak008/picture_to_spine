use f2s_application::export::publish_snapshot::PublishSnapshot;
use f2s_domain::animation::clip::{Curve, Keyframe, Track, TrackChannel};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn rounded_nanos(tick: i64, numerator: i64, denominator: i64) -> Result<i128, String> {
    if tick < 0 || numerator <= 0 || denominator <= 0 {
        return Err("invalid tick/timebase".into());
    }
    let scaled = (tick as i128)
        .checked_mul(numerator as i128)
        .and_then(|v| v.checked_mul(1_000_000_000))
        .ok_or("tick conversion overflow")?;
    let divisor = denominator as i128;
    let quotient = scaled / divisor;
    let remainder = scaled % divisor;
    let twice = remainder.checked_mul(2).ok_or("tick rounding overflow")?;
    Ok(
        if twice > divisor || (twice == divisor && quotient % 2 != 0) {
            quotient + 1
        } else {
            quotient
        },
    )
}
fn seconds(tick: i64, n: i64, d: i64) -> Result<f64, String> {
    Ok(rounded_nanos(tick, n, d)? as f64 / 1_000_000_000.0)
}
fn curve_value(key: &Keyframe) -> Result<Option<Value>, String> {
    match key.curve {
        Curve::Linear => Ok(None),
        Curve::Stepped => Ok(Some(json!("stepped"))),
        Curve::Bezier => {
            let c = key.bezier_milli.ok_or("Bezier key missing controls")?;
            Ok(Some(json!(c.map(|v| v as f64 / 1000.0))))
        }
    }
}
fn base_frame(key: &Keyframe, n: i64, d: i64) -> Result<Map<String, Value>, String> {
    let mut frame = Map::new();
    frame.insert("time".into(), json!(seconds(key.tick, n, d)?));
    if let Some(curve) = curve_value(key)? {
        frame.insert("curve".into(), curve);
    }
    Ok(frame)
}
fn ensure_time_precision(track: &Track, n: i64, d: i64) -> Result<(), String> {
    let mut previous = None;
    for key in &track.keyframes {
        let current = rounded_nanos(key.tick, n, d)?;
        if previous == Some(current) {
            return Err(format!("binary time collision in track {}", track.track_id));
        }
        previous = Some(current)
    }
    Ok(())
}
fn insert_timeline(
    group: &mut Map<String, Value>,
    target: &str,
    kind: &str,
    frames: Vec<Value>,
) -> Result<(), String> {
    let target_value = group
        .entry(target)
        .or_insert_with(|| Value::Object(Map::new()));
    let target_map = target_value
        .as_object_mut()
        .ok_or("timeline target is not object")?;
    if target_map
        .insert(kind.into(), Value::Array(frames))
        .is_some()
    {
        return Err(format!("duplicate {kind} timeline for {target}"));
    }
    Ok(())
}
fn path_without_extension(path: &str) -> String {
    path.trim_start_matches("images/")
        .strip_suffix(".png")
        .unwrap_or(path)
        .into()
}

fn mesh_attachment(
    snapshot: &PublishSnapshot,
    attachment_id: &str,
    path: &str,
    width: u32,
    height: u32,
) -> Result<Option<Value>, String> {
    let Some(mesh) = snapshot.meshes.iter().find(|v| v.layer_id == attachment_id) else {
        return Ok(None);
    };
    mesh.validate()?;
    let vertex_index: BTreeMap<_, _> = mesh
        .vertices
        .iter()
        .enumerate()
        .map(|(index, v)| (v.vertex_id, index as u32))
        .collect();
    let mut triangles = Vec::new();
    for triangle in &mesh.triangles {
        for id in [triangle.0, triangle.1, triangle.2] {
            triangles.push(*vertex_index.get(&id).ok_or("mesh triangle id missing")?)
        }
    }
    let uvs = mesh
        .vertices
        .iter()
        .flat_map(|v| [v.u_ppm as f64 / 1_000_000.0, v.v_ppm as f64 / 1_000_000.0])
        .collect::<Vec<_>>();
    let mut vertices = Vec::new();
    if let Some(weights) = snapshot.weights.iter().find(|v| v.mesh_id == mesh.mesh_id) {
        weights.validate(mesh, &snapshot.bones)?;
        let bone_index: BTreeMap<_, _> = snapshot
            .bones
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| (b.bone_id.as_str(), i))
            .collect();
        for vertex in &mesh.vertices {
            let influences = weights
                .by_vertex
                .get(&vertex.vertex_id)
                .ok_or("weighted mesh vertex missing weights")?;
            vertices.push(json!(influences.len()));
            for influence in influences {
                vertices.push(json!(
                    *bone_index
                        .get(influence.bone_id.as_str())
                        .ok_or("weight bone missing")?
                ));
                vertices.push(json!(vertex.x_milli_px as f64 / 1000.0));
                vertices.push(json!(vertex.y_milli_px as f64 / 1000.0));
                vertices.push(json!(influence.weight_ppm as f64 / 1_000_000.0));
            }
        }
    } else {
        for vertex in &mesh.vertices {
            vertices.push(json!(vertex.x_milli_px as f64 / 1000.0));
            vertices.push(json!(vertex.y_milli_px as f64 / 1000.0));
        }
    }
    Ok(Some(
        json!({"type":"mesh","path":path,"uvs":uvs,"triangles":triangles,"vertices":vertices,"hull":mesh.vertices.len(),"width":width,"height":height}),
    ))
}

fn animation_json(
    snapshot: &PublishSnapshot,
    clip: &f2s_domain::animation::clip::AnimationClip,
    event_definitions: &mut Map<String, Value>,
) -> Result<Value, String> {
    let n = snapshot.time_base.numerator;
    let d = snapshot.time_base.denominator;
    let bone_ids: BTreeSet<_> = snapshot
        .bones
        .bones
        .iter()
        .map(|v| v.bone_id.as_str())
        .collect();
    let slot_ids: BTreeSet<_> = snapshot
        .slots
        .slots
        .iter()
        .map(|v| v.slot_id.as_str())
        .collect();
    let mut bones = Map::new();
    let mut slots = Map::new();
    let mut draworder = Vec::new();
    let mut deform = Map::new();
    let mut events = Vec::new();
    for track in &clip.tracks {
        ensure_time_precision(track, n, d)?;
        match track.channel {
            TrackChannel::BoneRotate | TrackChannel::BoneTranslate | TrackChannel::BoneScale => {
                if !bone_ids.contains(track.target_id.as_str()) {
                    return Err(format!(
                        "animation target bone missing: {}",
                        track.target_id
                    ));
                }
                let (kind, arity) = match track.channel {
                    TrackChannel::BoneRotate => ("rotate", 1),
                    TrackChannel::BoneTranslate => ("translate", 2),
                    _ => ("scale", 2),
                };
                let mut frames = Vec::new();
                for key in &track.keyframes {
                    if key.values_milli.len() != arity {
                        return Err(format!("{kind} keyframe arity mismatch"));
                    }
                    let mut frame = base_frame(key, n, d)?;
                    match track.channel {
                        TrackChannel::BoneRotate => {
                            frame
                                .insert("angle".into(), json!(key.values_milli[0] as f64 / 1000.0));
                        }
                        TrackChannel::BoneTranslate => {
                            frame.insert("x".into(), json!(key.values_milli[0] as f64 / 1000.0));
                            frame.insert("y".into(), json!(key.values_milli[1] as f64 / 1000.0));
                        }
                        TrackChannel::BoneScale => {
                            frame.insert(
                                "x".into(),
                                json!(key.values_milli[0] as f64 / 1_000_000.0),
                            );
                            frame.insert(
                                "y".into(),
                                json!(key.values_milli[1] as f64 / 1_000_000.0),
                            );
                        }
                        _ => {}
                    }
                    frames.push(Value::Object(frame));
                }
                insert_timeline(&mut bones, &track.target_id, kind, frames)?;
            }
            TrackChannel::SlotColor => {
                if !slot_ids.contains(track.target_id.as_str()) {
                    return Err("slot color target missing".into());
                }
                let mut frames = Vec::new();
                for key in &track.keyframes {
                    if key.values_milli.len() != 4
                        || key.values_milli.iter().any(|v| !(0..=1000).contains(v))
                    {
                        return Err("slot color requires four values in 0..1000".into());
                    }
                    let mut frame = base_frame(key, n, d)?;
                    let color = key
                        .values_milli
                        .iter()
                        .map(|v| format!("{:02X}", ((*v * 255 + 500) / 1000)))
                        .collect::<String>();
                    frame.insert("color".into(), json!(color));
                    frames.push(Value::Object(frame));
                }
                insert_timeline(&mut slots, &track.target_id, "color", frames)?;
            }
            TrackChannel::DrawOrder => {
                if !slot_ids.contains(track.target_id.as_str()) {
                    return Err("draw order slot missing".into());
                }
                for key in &track.keyframes {
                    if key.values_milli.len() != 1 {
                        return Err("draw order requires one integer offset".into());
                    }
                    draworder.push(json!({"time":seconds(key.tick,n,d)?,"offsets":[{"slot":track.target_id,"offset":key.values_milli[0]}]}));
                }
            }
            TrackChannel::Deform => {
                let (slot, mesh) = track
                    .target_id
                    .split_once('/')
                    .ok_or("deform target must be slot/mesh")?;
                if !slot_ids.contains(slot) || !snapshot.meshes.iter().any(|v| v.mesh_id == mesh) {
                    return Err("deform target missing".into());
                }
                let mut frames = Vec::new();
                for key in &track.keyframes {
                    if key.values_milli.len() % 2 != 0 {
                        return Err("deform values must be xy pairs".into());
                    }
                    let mut frame = base_frame(key, n, d)?;
                    frame.insert(
                        "vertices".into(),
                        json!(
                            key.values_milli
                                .iter()
                                .map(|v| *v as f64 / 1000.0)
                                .collect::<Vec<_>>()
                        ),
                    );
                    frames.push(Value::Object(frame));
                }
                let skin = deform
                    .entry("default")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .ok_or("deform skin")?;
                let slot_map = skin
                    .entry(slot)
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .ok_or("deform slot")?;
                if slot_map.insert(mesh.into(), Value::Array(frames)).is_some() {
                    return Err("duplicate deform timeline".into());
                }
            }
            TrackChannel::Event => {
                event_definitions
                    .entry(track.target_id.clone())
                    .or_insert_with(|| json!({}));
                for key in &track.keyframes {
                    events.push(json!({"time":seconds(key.tick,n,d)?,"name":track.target_id}));
                }
            }
        }
    }
    for marker in snapshot
        .markers
        .iter()
        .filter(|v| v.action_key == clip.action_key)
    {
        let name = format!("f2s:{}", marker.marker_id);
        event_definitions
            .entry(name.clone())
            .or_insert_with(|| json!({"string":marker.socket_id}));
        events.push(
            json!({"time":seconds(marker.start_tick,n,d)?,"name":name,"string":marker.socket_id}),
        );
    }
    let mut animation = Map::new();
    if !bones.is_empty() {
        animation.insert("bones".into(), Value::Object(bones));
    }
    if !slots.is_empty() {
        animation.insert("slots".into(), Value::Object(slots));
    }
    if !deform.is_empty() {
        animation.insert("deform".into(), Value::Object(deform));
    }
    if !draworder.is_empty() {
        draworder.sort_by(|a, b| {
            a["time"]
                .as_f64()
                .partial_cmp(&b["time"].as_f64())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        animation.insert("draworder".into(), Value::Array(draworder));
    }
    if !events.is_empty() {
        events.sort_by(|a, b| {
            a["time"]
                .as_f64()
                .partial_cmp(&b["time"].as_f64())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        animation.insert("events".into(), Value::Array(events));
    }
    if animation.is_empty() {
        return Err(format!(
            "clip {} has no serializable timeline",
            clip.action_key
        ));
    }
    Ok(Value::Object(animation))
}

fn validate_reopened(value: &Value) -> Result<(), String> {
    let root = value.as_object().ok_or("Spine JSON root must be object")?;
    if root
        .get("skeleton")
        .and_then(|v| v.get("spine"))
        .and_then(Value::as_str)
        != Some("4.2.43")
    {
        return Err("Spine JSON patch mismatch".into());
    }
    if value.to_string().contains("f2sDuration") {
        return Err("nonstandard duration field forbidden".into());
    }
    let animations = root
        .get("animations")
        .and_then(Value::as_object)
        .ok_or("animations missing")?;
    if animations.len() != 10
        || !f2s_domain::ACTION_KEYS.iter().all(|key| {
            animations
                .get(*key)
                .and_then(Value::as_object)
                .is_some_and(|v| !v.is_empty())
        })
    {
        return Err("Spine JSON must contain ten nonempty animations".into());
    }
    let definitions = root.get("events").and_then(Value::as_object);
    for animation in animations.values() {
        if let Some(events) = animation.get("events").and_then(Value::as_array) {
            for event in events {
                let name = event
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("event name missing")?;
                if !definitions.is_some_and(|v| v.contains_key(name)) {
                    return Err("animation event lacks setup definition".into());
                }
            }
        }
    }
    Ok(())
}

pub fn spine_json_bytes(snapshot: &PublishSnapshot) -> Result<Vec<u8>, String> {
    if !snapshot.pinned_capability() {
        return Err("serializer supports only Spine 4.2.43".into());
    }
    let bones = snapshot
        .bones
        .bones
        .iter()
        .map(|b| {
            let mut item = Map::new();
            item.insert("name".into(), json!(b.bone_id));
            if let Some(parent) = &b.parent_id {
                item.insert("parent".into(), json!(parent));
            }
            item.insert("x".into(), json!(b.rest.x_milli_px as f64 / 1000.0));
            item.insert("y".into(), json!(b.rest.y_milli_px as f64 / 1000.0));
            item.insert(
                "rotation".into(),
                json!(b.rest.rotation_milli_deg as f64 / 1000.0),
            );
            item.insert(
                "scaleX".into(),
                json!(b.rest.scale_x_ppm as f64 / 1_000_000.0),
            );
            item.insert(
                "scaleY".into(),
                json!(b.rest.scale_y_ppm as f64 / 1_000_000.0),
            );
            Value::Object(item)
        })
        .collect::<Vec<_>>();
    let slots = snapshot
        .slots
        .stable_draw_order()
        .into_iter()
        .map(|s| json!({"name":s.slot_id,"bone":s.bone_id,"attachment":s.layer_id}))
        .collect::<Vec<_>>();
    let mut attachments = Map::new();
    for item in &snapshot.attachments {
        let path = path_without_extension(&item.logical_png_path);
        let value = if let Some(mesh) = mesh_attachment(
            snapshot,
            &item.attachment_id,
            &path,
            item.width,
            item.height,
        )? {
            mesh
        } else {
            let pivot = snapshot
                .pivots
                .iter()
                .find(|v| v.layer_id == item.attachment_id)
                .map(|v| v.point);
            json!({"path":path,"x":pivot.map(|v|v.x_milli_px as f64/1000.0).unwrap_or(0.0),"y":pivot.map(|v|v.y_milli_px as f64/1000.0).unwrap_or(0.0),"width":item.width,"height":item.height})
        };
        attachments
            .entry(item.slot_id.clone())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or("skin slot object")?
            .insert(item.attachment_id.clone(), value);
    }
    let transforms=snapshot.constraints.iter().map(|v|json!({"name":v.constraint_id,"order":v.order,"bones":[v.constrained_bone_id],"target":v.target_bone_id,"rotateMix":v.mix_ppm as f64/1_000_000.0,"translateMix":v.mix_ppm as f64/1_000_000.0,"scaleMix":v.mix_ppm as f64/1_000_000.0,"shearMix":0.0})).collect::<Vec<_>>();
    let mut event_definitions = Map::new();
    let mut animations = Map::new();
    for clip in &snapshot.clips {
        animations.insert(
            clip.action_key.clone(),
            animation_json(snapshot, clip, &mut event_definitions)?,
        );
    }
    let mut root = Map::new();
    root.insert(
        "skeleton".into(),
        json!({"hash":snapshot.export_id,"spine":"4.2.43","images":"./images/"}),
    );
    root.insert("bones".into(), Value::Array(bones));
    root.insert("slots".into(), Value::Array(slots));
    if !transforms.is_empty() {
        root.insert("transform".into(), Value::Array(transforms));
    }
    root.insert(
        "skins".into(),
        json!([{"name":"default","attachments":attachments}]),
    );
    if !event_definitions.is_empty() {
        root.insert("events".into(), Value::Object(event_definitions));
    }
    root.insert("animations".into(), Value::Object(animations));
    let value = Value::Object(root);
    validate_reopened(&value)?;
    let bytes = f2s_domain::canonical::canonical_bytes(&value).map_err(|e| e.to_string())?;
    let reopened: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    validate_reopened(&reopened)?;
    Ok(bytes)
}
