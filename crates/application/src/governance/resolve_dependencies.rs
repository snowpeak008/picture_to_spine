use f2s_domain::governance::{Gate, GateState};
use std::collections::{BTreeMap, BTreeSet};
pub fn resolve_gates(gates: &[Gate]) -> Result<Vec<String>, String> {
    let map: BTreeMap<_, _> = gates.iter().map(|g| (g.gate_id.clone(), g)).collect();
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    while order.len() < gates.len() {
        let mut progressed = false;
        for (id, gate) in &map {
            if done.contains(id) {
                continue;
            }
            if gate
                .dependency_gate_ids
                .iter()
                .all(|dep| done.contains(dep))
            {
                if gate
                    .dependency_gate_ids
                    .iter()
                    .any(|dep| map.get(dep).is_none_or(|g| g.state != GateState::Approved))
                {
                    return Err(format!("dependency not approved for {id}"));
                }
                done.insert(id.clone());
                order.push(id.clone());
                progressed = true;
            }
        }
        if !progressed {
            return Err("gate dependency cycle".into());
        }
    }
    Ok(order)
}
