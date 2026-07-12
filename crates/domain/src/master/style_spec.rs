use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GripMode {
    OneHand,
    TwoHand,
    Flexible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WeaponHand {
    NearHand,
    FarHand,
    BothHands,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WeaponSizeClass {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrimaryWeaponSpec {
    pub weapon_type: String,
    pub grip_mode: GripMode,
    pub weapon_hand: WeaponHand,
    pub socket_semantic: String,
    pub size_class: WeaponSizeClass,
    pub silhouette_constraints: String,
}

impl PrimaryWeaponSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.weapon_type.trim().is_empty()
            || self.socket_semantic.trim().is_empty()
            || self.silhouette_constraints.trim().is_empty()
        {
            return Err(
                "primary weapon type, socket, and silhouette constraints are required".into(),
            );
        }
        Ok(())
    }
    pub fn prompt_description(&self) -> String {
        format!(
            "{}，握持={:?}，手位={:?}，尺寸={:?}，socket={}，轮廓约束={}",
            self.weapon_type,
            self.grip_mode,
            self.weapon_hand,
            self.size_class,
            self.socket_semantic,
            self.silhouette_constraints
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSpec {
    pub revision: u64,
    pub viewpoint: String,
    pub rendering_style: String,
    pub outline: String,
    pub palette_notes: String,
    pub identity_notes: String,
    pub primary_weapon: Option<PrimaryWeaponSpec>,
}
impl StyleSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.viewpoint != "side-view" {
            return Err("V1 requires side-view".into());
        }
        self.primary_weapon
            .as_ref()
            .ok_or("V1 requires one confirmed primary weapon")?
            .validate()?;
        if self.identity_notes.trim().is_empty() {
            return Err("identity notes required".into());
        }
        Ok(())
    }
}
