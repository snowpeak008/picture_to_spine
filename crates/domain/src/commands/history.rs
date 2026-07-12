use super::{Command, CommandReceipt};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandHistory {
    pub done: Vec<(Command, CommandReceipt)>,
    pub undone: Vec<(Command, CommandReceipt)>,
}
impl CommandHistory {
    pub fn push(&mut self, command: Command, receipt: CommandReceipt) {
        self.done.push((command, receipt));
        self.undone.clear()
    }
    pub fn pop_undo(&mut self) -> Option<(Command, CommandReceipt)> {
        let value = self.done.pop()?;
        self.undone.push(value.clone());
        Some(value)
    }
    pub fn pop_redo(&mut self) -> Option<(Command, CommandReceipt)> {
        let value = self.undone.pop()?;
        self.done.push(value.clone());
        Some(value)
    }
}
