use serde_json::Value;

/// Compares two `serde_json::Value` instances using a provided predicate on their ordering.
/// The comparison is case-sensitive for strings and attempts to handle numbers, booleans, and mixed types.
///
/// # Arguments
/// * `a` - First value to compare.
/// * `b` - Second value to compare.
/// * `pred_on_ord` - A predicate function that takes an `i32` representing the ordering:
///     -1 if `a` < `b`, 0 if `a` == `b`, 1 if `a` > `b`.
///
/// # Returns
/// * `bool` - Result of the predicate applied to the comparison.
pub fn cmp_values<F>(a: &Value, b: &Value, pred_on_ord: F) -> bool
where
    F: Fn(i32) -> bool,
{
    // Match on the types of both values
    match (a, b) {
        // Both are strings: compare lexicographically (case-sensitive)
        (Value::String(sa), Value::String(sb)) => pred_on_ord(sa.cmp(sb) as i32),

        // Both are numbers: compare as f64 if possible, otherwise fallback to equality
        (Value::Number(na), Value::Number(nb)) => {
            if let (Some(da), Some(db)) = (na.as_f64(), nb.as_f64()) {
                // Use epsilon to check for floating-point equality
                let ord = if (da - db).abs() < f64::EPSILON {
                    0
                } else if da < db {
                    -1
                } else {
                    1
                };
                pred_on_ord(ord)
            } else {
                // Fallback: compare as JSON numbers (rare case)
                pred_on_ord(0) && na == nb
            }
        }

        // Both are booleans: compare by casting to i32 (false=0, true=1)
        (Value::Bool(ba), Value::Bool(bb)) => {
            let ord = (*ba as i32) - (*bb as i32);
            pred_on_ord(ord)
        }

        // One is a number, one is a string: try to parse string as f64 and compare
        (Value::Number(na), Value::String(sb)) | (Value::String(sb), Value::Number(na)) => {
            if let (Some(da), Ok(db)) = (na.as_f64(), sb.parse::<f64>()) {
                let ord = if (da - db).abs() < f64::EPSILON {
                    0
                } else if da < db {
                    -1
                } else {
                    1
                };
                pred_on_ord(ord)
            } else {
                // Fallback: compare their string representations
                pred_on_ord(a.to_string().cmp(&b.to_string()) as i32)
            }
        }

        // All other type combinations: compare their string representations
        _ => pred_on_ord(a.to_string().cmp(&b.to_string()) as i32),
    }
}
