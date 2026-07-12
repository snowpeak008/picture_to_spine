use super::bone_tree::BoneTree;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub slot_id: String,
    pub layer_id: String,
    pub bone_id: String,
    pub draw_key: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotSet {
    pub revision: u64,
    pub slots: Vec<Slot>,
}

impl SlotSet {
    pub fn validate(&self, required_layers: &[String], bones: &BoneTree) -> Result<(), String> {
        let bone_ids: BTreeSet<_> = bones.bones.iter().map(|v| v.bone_id.as_str()).collect();
        let mut slots = BTreeSet::new();
        let mut layers = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for slot in &self.slots {
            if !slots.insert(slot.slot_id.as_str()) {
                return Err("duplicate slot id".into());
            }
            if !layers.insert(slot.layer_id.as_str()) {
                return Err("layer mapped to multiple slots".into());
            }
            if !keys.insert(slot.draw_key) {
                return Err("duplicate draw key".into());
            }
            if !bone_ids.contains(slot.bone_id.as_str()) {
                return Err(format!("slot references unknown bone: {}", slot.bone_id));
            }
        }
        for layer in required_layers {
            if !layers.contains(layer.as_str()) {
                return Err(format!("unmapped layer: {layer}"));
            }
        }
        Ok(())
    }

    pub fn stable_draw_order(&self) -> Vec<&Slot> {
        let mut ordered: Vec<_> = self.slots.iter().collect();
        ordered.sort_by_key(|v| (v.draw_key, &v.slot_id));
        ordered
    }

    pub fn set_binding_and_draw_key(
        &mut self,
        slot_id: &str,
        bone_id: &str,
        draw_key: i32,
        bones: &BoneTree,
    ) -> Result<Slot, String> {
        let before = self.clone();
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.slot_id == slot_id)
            .ok_or("unknown slot")?;
        if slot.bone_id == bone_id && slot.draw_key == draw_key {
            return Err("slot binding and draw key are unchanged".into());
        }
        let previous = slot.clone();
        slot.bone_id = bone_id.into();
        slot.draw_key = draw_key;
        let required_layers = self
            .slots
            .iter()
            .map(|slot| slot.layer_id.clone())
            .collect::<Vec<_>>();
        if let Err(error) = self.validate(&required_layers, bones) {
            *self = before;
            return Err(error);
        }
        self.revision = match self.revision.checked_add(1) {
            Some(revision) => revision,
            None => {
                *self = before;
                return Err("slot revision overflow".into());
            }
        };
        Ok(previous)
    }
}
