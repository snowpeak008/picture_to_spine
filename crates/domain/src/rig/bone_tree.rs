use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type BoneId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestTransform {
    pub x_milli_px: i64,
    pub y_milli_px: i64,
    pub rotation_milli_deg: i32,
    pub scale_x_ppm: i32,
    pub scale_y_ppm: i32,
}

impl Default for RestTransform {
    fn default() -> Self {
        Self {
            x_milli_px: 0,
            y_milli_px: 0,
            rotation_milli_deg: 0,
            scale_x_ppm: 1_000_000,
            scale_y_ppm: 1_000_000,
        }
    }
}

impl RestTransform {
    pub fn validate(self) -> Result<(), String> {
        if self.scale_x_ppm == 0 || self.scale_y_ppm == 0 {
            return Err("singular bone scale".into());
        }
        let supported_translation = -100_000_000..=100_000_000;
        if !supported_translation.contains(&self.x_milli_px)
            || !supported_translation.contains(&self.y_milli_px)
        {
            return Err("bone translation outside supported canvas".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RestTransform;

    #[test]
    fn extreme_i64_translations_fail_closed_without_overflow() {
        for value in [i64::MIN, i64::MAX] {
            let x = RestTransform {
                x_milli_px: value,
                ..RestTransform::default()
            };
            assert_eq!(
                x.validate().unwrap_err(),
                "bone translation outside supported canvas"
            );

            let y = RestTransform {
                y_milli_px: value,
                ..RestTransform::default()
            };
            assert_eq!(
                y.validate().unwrap_err(),
                "bone translation outside supported canvas"
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoneNode {
    pub bone_id: BoneId,
    pub name: String,
    pub parent_id: Option<BoneId>,
    pub rest: RestTransform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoneTree {
    pub revision: u64,
    pub bones: Vec<BoneNode>,
}

impl BoneTree {
    pub fn validate(&self) -> Result<(), String> {
        if self.bones.is_empty() {
            return Err("bone tree requires a root".into());
        }
        let ids: BTreeSet<_> = self.bones.iter().map(|b| b.bone_id.as_str()).collect();
        if ids.len() != self.bones.len() {
            return Err("duplicate bone id".into());
        }
        if self.bones.iter().filter(|b| b.parent_id.is_none()).count() != 1 {
            return Err("bone tree requires exactly one root".into());
        }
        for bone in &self.bones {
            if bone.bone_id.trim().is_empty() || bone.name.trim().is_empty() {
                return Err("bone id and name are required".into());
            }
            bone.rest.validate()?;
            if let Some(parent) = &bone.parent_id {
                if parent == &bone.bone_id {
                    return Err("bone cannot parent itself".into());
                }
                if !ids.contains(parent.as_str()) {
                    return Err(format!("unknown parent bone: {parent}"));
                }
            }
        }
        let parents: BTreeMap<_, _> = self
            .bones
            .iter()
            .map(|b| (b.bone_id.as_str(), b.parent_id.as_deref()))
            .collect();
        for bone in &self.bones {
            let mut seen = BTreeSet::new();
            let mut cursor = Some(bone.bone_id.as_str());
            while let Some(id) = cursor {
                if !seen.insert(id) {
                    return Err(format!("bone cycle at {id}"));
                }
                cursor = parents.get(id).copied().flatten();
            }
        }
        Ok(())
    }

    pub fn reparent(&mut self, bone_id: &str, parent_id: &str) -> Result<Option<BoneId>, String> {
        if bone_id == parent_id {
            return Err("bone cannot parent itself".into());
        }
        let before = self.clone();
        let previous = self
            .bones
            .iter()
            .find(|b| b.bone_id == bone_id)
            .ok_or("unknown bone")?
            .parent_id
            .clone();
        if !self.bones.iter().any(|b| b.bone_id == parent_id) {
            return Err("unknown parent bone".into());
        }
        self.bones
            .iter_mut()
            .find(|b| b.bone_id == bone_id)
            .expect("checked above")
            .parent_id = Some(parent_id.into());
        if let Err(error) = self.validate() {
            *self = before;
            return Err(error);
        }
        self.revision = match self.revision.checked_add(1) {
            Some(revision) => revision,
            None => {
                *self = before;
                return Err("bone tree revision overflow".into());
            }
        };
        Ok(previous)
    }

    pub fn set_rest_transform(
        &mut self,
        bone_id: &str,
        rest: RestTransform,
    ) -> Result<RestTransform, String> {
        rest.validate()?;
        let bone = self
            .bones
            .iter_mut()
            .find(|bone| bone.bone_id == bone_id)
            .ok_or("unknown bone")?;
        if bone.rest == rest {
            return Err("bone transform is unchanged".into());
        }
        let previous = bone.rest;
        bone.rest = rest;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or("bone tree revision overflow")?;
        Ok(previous)
    }

    pub fn referenced_descendants(&self, bone_id: &str) -> Vec<BoneId> {
        let mut result = Vec::new();
        let mut frontier = vec![bone_id];
        while let Some(parent) = frontier.pop() {
            for bone in self
                .bones
                .iter()
                .filter(|b| b.parent_id.as_deref() == Some(parent))
            {
                result.push(bone.bone_id.clone());
                frontier.push(&bone.bone_id);
            }
        }
        result.sort();
        result
    }
}
