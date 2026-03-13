use serde_json::Value;

/// Compute JSON patch between two states.
pub fn compute_diff(_old: &Value, _new: &Value) -> Value {
    // Trivial fallback: return the entire new state as the diff
    _new.clone()
}
