pub mod bone_tree;
pub mod candidate;
pub mod constraints;
pub mod mesh;
pub mod pivots_sockets;
pub mod slots;
pub mod weights;

pub use candidate::*;

use serde::{Deserialize, Serialize};

pub const SPINE_CAPABILITY_ID: &str = "F2S-SPINE-CAP-4.2.43-001";
pub const SPINE_PATCH: &str = "4.2.43";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RigRevisionRefs {
    pub layer_set_revision: u64,
    pub bone_tree_revision: u64,
    pub slot_revision: u64,
    pub pivot_socket_revision: u64,
    pub mesh_revision: u64,
    pub weight_revision: u64,
    pub constraint_revision: u64,
}
