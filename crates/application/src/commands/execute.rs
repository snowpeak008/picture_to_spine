use super::IdempotencyRegistry;
use f2s_domain::commands::{Command, CommandReceipt};
pub fn execute_command(
    command: &Command,
    current_revision: u64,
    registry: &mut IdempotencyRegistry,
) -> Result<CommandReceipt, String> {
    if let Some(receipt) = registry.get(&command.command_id) {
        return Ok(receipt.clone());
    }
    if command.expected_revision != current_revision {
        return Err("revision conflict".into());
    }
    let receipt = CommandReceipt {
        command_id: command.command_id.clone(),
        before_revision: current_revision,
        after_revision: current_revision + 1,
        effect_refs: vec![],
    };
    registry.record(receipt.clone())?;
    Ok(receipt)
}
