use serde_json::Value;
use crate::engine::JpOptions;

#[derive(Clone, Copy)]
pub enum CmpMode {
    CaseSensitive,
    CaseFoldLower,
    CaseFoldUpper,
}

impl Default for CmpMode {
    fn default() -> Self {
        CmpMode::CaseSensitive
    }
}

fn cmp_strings(sa: &str, sb: &str, mode: CmpMode, pred_on_ord: impl Fn(i32) -> bool) -> bool {
    let (la, lb) = match mode {
        CmpMode::CaseSensitive => (sa.to_owned(), sb.to_owned()),
        CmpMode::CaseFoldLower => (sa.to_lowercase(), sb.to_lowercase()),
        CmpMode::CaseFoldUpper => (sa.to_uppercase(), sb.to_uppercase()),
    };
    pred_on_ord(la.cmp(&lb) as i32)
}

pub fn cmp_values<F>(a: &Value, b: &Value, opts: &JpOptions, pred_on_ord: F) -> bool
where
    F: Fn(i32) -> bool,
{
    match (a, b) {
        (Value::String(sa), Value::String(sb)) => {
            cmp_strings(sa, sb, opts.cmp, pred_on_ord)
        }
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
                cmp_strings(&a.to_string(), &b.to_string(), opts.cmp, pred_on_ord)
            }
        }
        _ => {
            cmp_strings(&a.to_string(), &b.to_string(), opts.cmp, pred_on_ord)
        }
    }
}
