use f2s_domain::governance::{Decision, Gate, GateState};
pub fn apply_decision(gate: &mut Gate, decision: &Decision) -> Result<(), String> {
    if gate.gate_id != decision.gate_id {
        return Err("decision gate mismatch".into());
    }
    if gate.target_revision != decision.target_revision {
        return Err("stale decision revision".into());
    }
    if decision.state == GateState::Pending {
        return Err("pending is not a terminal decision".into());
    }
    gate.state = decision.state;
    Ok(())
}
