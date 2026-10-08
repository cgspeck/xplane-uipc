use std::collections::{BTreeMap, HashMap};

use crate::Expr;

/// The most times one client write can run a command, so a bad expression
/// can't fire it thousands of times.
pub const MAX_COMMAND_RUNS: u32 = 10;

/// What one client write does, worked out before anything is written.
#[derive(Debug, PartialEq)]
pub struct WritePlan<'a> {
    /// `(datarefs name, value)` in key order. Written first.
    pub datarefs: Vec<(&'a str, f64)>,
    /// `(command path, times to run)` in path order. Run after the datarefs.
    pub commands: Vec<(&'a str, u32)>,
}

/// Evaluate every write expression and command expression of a mapping
/// against one snapshot of variables (`value` plus each dataref's current
/// value).
///
/// A command's result is rounded and clamped to `0..=MAX_COMMAND_RUNS`. If any
/// result isn't finite, returns an error naming that entry, and nothing
/// should be written or run.
pub fn eval_writes<'a>(
    write_exprs: &'a BTreeMap<String, Expr>,
    write_commands: &'a BTreeMap<String, Expr>,
    vars: &HashMap<String, f64>,
) -> Result<WritePlan<'a>, String> {
    let eval = |table: &str, key: &str, expr: &Expr| {
        let v = expr.eval(vars);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(format!("{} '{}' gave {}", table, key, v))
        }
    };
    let datarefs = write_exprs
        .iter()
        .map(|(name, expr)| Ok((name.as_str(), eval("write_exprs", name, expr)?)))
        .collect::<Result<_, String>>()?;
    let commands = write_commands
        .iter()
        .map(|(path, expr)| {
            let runs = eval("write_commands", path, expr)?
                .round()
                .clamp(0.0, MAX_COMMAND_RUNS as f64);
            Ok((path.as_str(), runs as u32))
        })
        .collect::<Result<_, String>>()?;
    Ok(WritePlan { datarefs, commands })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(entries: &[(&str, &str)]) -> BTreeMap<String, Expr> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), Expr::parse(v).unwrap()))
            .collect()
    }

    fn vars(entries: &[(&str, f64)]) -> HashMap<String, f64> {
        entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn no_commands() -> BTreeMap<String, Expr> {
        BTreeMap::new()
    }

    #[test]
    fn fan_out_in_key_order() {
        let w = table(&[("Nav", "$value 1 & 0 !="), ("Bcn", "$value 2 & 0 !=")]);
        let c = no_commands();
        let plan = eval_writes(&w, &c, &vars(&[("value", 2.0)])).unwrap();
        assert_eq!(plan.datarefs, vec![("Bcn", 1.0), ("Nav", 0.0)]);
        assert!(plan.commands.is_empty());
    }

    #[test]
    fn uses_current_dataref_value() {
        // Zulu hour: 12:34:56.5, client writes 1 → 01:34:56.5
        let w = table(&[("Z", "$value 3600 * $Z 3600 % +")]);
        let c = no_commands();
        let plan = eval_writes(&w, &c, &vars(&[("value", 1.0), ("Z", 45296.5)])).unwrap();
        assert_eq!(plan.datarefs, vec![("Z", 5696.5)]);
    }

    #[test]
    fn bcd_frequency_write() {
        // COM1: a client writes 0x2250 for 122.50 MHz; the dataref is 12250.
        let w = table(&[("C", "$value frombcd 10000 +")]);
        let c = no_commands();
        let plan = eval_writes(&w, &c, &vars(&[("value", 0x2250 as f64)])).unwrap();
        assert_eq!(plan.datarefs, vec![("C", 12250.0)]);
    }

    #[test]
    fn invalid_bcd_write_blocks_everything() {
        let w = table(&[("C", "$value frombcd 10000 +")]);
        let c = no_commands();
        let err = eval_writes(&w, &c, &vars(&[("value", 0x12AB as f64)])).unwrap_err();
        assert!(err.contains("'C'"), "{}", err);
    }

    #[test]
    fn results_see_one_snapshot() {
        let w = table(&[("A", "$B"), ("B", "$A")]);
        let c = no_commands();
        let plan = eval_writes(&w, &c, &vars(&[("value", 0.0), ("A", 1.0), ("B", 2.0)])).unwrap();
        assert_eq!(plan.datarefs, vec![("A", 2.0), ("B", 1.0)]);
    }

    #[test]
    fn non_finite_result_blocks_everything() {
        let w = table(&[("A", "$value"), ("B", "10 $value ^")]);
        let c = no_commands();
        let err = eval_writes(&w, &c, &vars(&[("value", 400.0)])).unwrap_err();
        assert!(err.contains("'B'"), "{}", err);
    }

    #[test]
    fn nan_dataref_blocks_everything() {
        let w = table(&[("A", "$value"), ("B", "$B 1 +")]);
        let c = no_commands();
        let err = eval_writes(&w, &c, &vars(&[("value", 1.0), ("B", f64::NAN)])).unwrap_err();
        assert!(err.contains("'B'"), "{}", err);
    }

    #[test]
    fn toggle_runs_only_when_state_differs() {
        // Landing lights are bit 4; toggle the add-on switch when it differs.
        let w = table(&[("Land", "$value 4 & 0 !=")]);
        let c = table(&[("addon/land_toggle", "$value 4 & 0 != $ZLand 0 != !=")]);
        let off = eval_writes(&w, &c, &vars(&[("value", 5.0), ("ZLand", 0.0)])).unwrap();
        assert_eq!(off.datarefs, vec![("Land", 1.0)]);
        assert_eq!(off.commands, vec![("addon/land_toggle", 1)]);
        let on = eval_writes(&w, &c, &vars(&[("value", 5.0), ("ZLand", 1.0)])).unwrap();
        assert_eq!(on.commands, vec![("addon/land_toggle", 0)]);
    }

    #[test]
    fn run_count_is_rounded_and_clamped() {
        let w = no_commands();
        let c = table(&[("a/round", "2.6"), ("b/negative", "-1"), ("c/many", "50")]);
        let plan = eval_writes(&w, &c, &vars(&[("value", 0.0)])).unwrap();
        assert_eq!(
            plan.commands,
            vec![
                ("a/round", 3),
                ("b/negative", 0),
                ("c/many", MAX_COMMAND_RUNS)
            ]
        );
    }

    #[test]
    fn non_finite_command_blocks_dataref_writes() {
        let w = table(&[("A", "$value")]);
        let c = table(&[("addon/cmd", "10 $value ^")]);
        let err = eval_writes(&w, &c, &vars(&[("value", 400.0)])).unwrap_err();
        assert!(err.contains("write_commands 'addon/cmd'"), "{}", err);
    }
}
