use std::collections::{BTreeMap, HashMap};

use crate::Expr;

/// Evaluate every write expression of a mapping against one snapshot of
/// variables (`value` plus each dataref's current value).
///
/// Returns `(target, value)` pairs in key order. If any result isn't finite,
/// returns an error naming that target and nothing should be written.
pub fn eval_write_exprs<'a>(
    write_exprs: &'a BTreeMap<String, Expr>,
    vars: &HashMap<String, f64>,
) -> Result<Vec<(&'a str, f64)>, String> {
    write_exprs
        .iter()
        .map(|(target, expr)| {
            let v = expr.eval(vars);
            if v.is_finite() {
                Ok((target.as_str(), v))
            } else {
                Err(format!("write_exprs '{}' gave {}", target, v))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_exprs(entries: &[(&str, &str)]) -> BTreeMap<String, Expr> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), Expr::parse(v).unwrap()))
            .collect()
    }

    fn vars(entries: &[(&str, f64)]) -> HashMap<String, f64> {
        entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn fan_out_in_key_order() {
        let w = write_exprs(&[("Nav", "$value 1 & 0 !="), ("Bcn", "$value 2 & 0 !=")]);
        let results = eval_write_exprs(&w, &vars(&[("value", 2.0)])).unwrap();
        assert_eq!(results, vec![("Bcn", 1.0), ("Nav", 0.0)]);
    }

    #[test]
    fn uses_current_dataref_value() {
        // Zulu hour: 12:34:56.5, client writes 1 → 01:34:56.5
        let w = write_exprs(&[("Z", "$value 3600 * $Z 3600 % +")]);
        let results = eval_write_exprs(&w, &vars(&[("value", 1.0), ("Z", 45296.5)])).unwrap();
        assert_eq!(results, vec![("Z", 5696.5)]);
    }

    #[test]
    fn results_see_one_snapshot() {
        let w = write_exprs(&[("A", "$B"), ("B", "$A")]);
        let results =
            eval_write_exprs(&w, &vars(&[("value", 0.0), ("A", 1.0), ("B", 2.0)])).unwrap();
        assert_eq!(results, vec![("A", 2.0), ("B", 1.0)]);
    }

    #[test]
    fn non_finite_result_blocks_everything() {
        let w = write_exprs(&[("A", "$value"), ("B", "10 $value ^")]);
        let err = eval_write_exprs(&w, &vars(&[("value", 400.0)])).unwrap_err();
        assert!(err.contains("'B'"), "{}", err);
    }

    #[test]
    fn nan_dataref_blocks_everything() {
        let w = write_exprs(&[("A", "$value"), ("B", "$B 1 +")]);
        let err = eval_write_exprs(&w, &vars(&[("value", 1.0), ("B", f64::NAN)])).unwrap_err();
        assert!(err.contains("'B'"), "{}", err);
    }
}
