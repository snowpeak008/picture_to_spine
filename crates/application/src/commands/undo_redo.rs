use f2s_domain::commands::{CommandHistory, CommandReceipt};
pub fn undo(history: &mut CommandHistory, current_revision: u64) -> Result<CommandReceipt, String> {
    let (_, receipt) = history.pop_undo().ok_or("nothing to undo")?;
    Ok(CommandReceipt {
        command_id: format!("undo:{}", receipt.command_id),
        before_revision: current_revision,
        after_revision: current_revision + 1,
        effect_refs: receipt.effect_refs,
    })
}
pub fn redo(history: &mut CommandHistory, current_revision: u64) -> Result<CommandReceipt, String> {
    let (_, receipt) = history.pop_redo().ok_or("nothing to redo")?;
    Ok(CommandReceipt {
        command_id: format!("redo:{}", receipt.command_id),
        before_revision: current_revision,
        after_revision: current_revision + 1,
        effect_refs: receipt.effect_refs,
    })
}
