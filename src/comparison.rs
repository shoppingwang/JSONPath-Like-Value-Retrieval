use serde_json::Value;

pub fn cmp_values<F>(a: &Value, b: &Value, pred_on_ord: F) -> bool
where
    F: Fn(i32) -> bool,
{
    // Simplified to always use case-sensitive comparison
    match (a, b) {
        (Value::String(sa), Value::String(sb)) => pred_on_ord(sa.cmp(sb) as i32),
        (Value::Number(na), Value::Number(nb)) => {
            if let (Some(da), Some(db)) = (na.as_f64(), nb.as_f64()) {
                let ord = if (da - db).abs() < f64::EPSILON {
                    0
                } else if da < db {
                    -1
                } else {
                    1
                };
                pred_on_ord(ord)
            } else {
                pred_on_ord(0) && na == nb
            }
        }
        (Value::Bool(ba), Value::Bool(bb)) => {
            let ord = (*ba as i32) - (*bb as i32);
            pred_on_ord(ord)
        }
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
                pred_on_ord(a.to_string().cmp(&b.to_string()) as i32)
            }
        }
        _ => pred_on_ord(a.to_string().cmp(&b.to_string()) as i32),
    }
}
