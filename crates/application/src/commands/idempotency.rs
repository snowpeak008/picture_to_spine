use f2s_domain::commands::CommandReceipt;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct IdempotencyRegistry {
    receipts: BTreeMap<String, CommandReceipt>,
}
impl IdempotencyRegistry {
    pub fn get(&self, id: &str) -> Option<&CommandReceipt> {
        self.receipts.get(id)
    }
    pub fn record(&mut self, receipt: CommandReceipt) -> Result<(), String> {
        if self.receipts.contains_key(&receipt.command_id) {
            return Err("duplicate command id".into());
        }
        self.receipts.insert(receipt.command_id.clone(), receipt);
        Ok(())
    }
}
