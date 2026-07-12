use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    F2sBoot001,
    F2sPath001,
    F2sRevision001,
    F2sGate001,
    F2sStorage001,
    F2sExternal001,
}
impl ErrorCode {
    pub fn stable_code(self) -> &'static str {
        match self {
            Self::F2sBoot001 => "F2S-BOOT-001",
            Self::F2sPath001 => "F2S-PATH-001",
            Self::F2sRevision001 => "F2S-REVISION-001",
            Self::F2sGate001 => "F2S-GATE-001",
            Self::F2sStorage001 => "F2S-STORAGE-001",
            Self::F2sExternal001 => "F2S-EXTERNAL-001",
        }
    }
}
