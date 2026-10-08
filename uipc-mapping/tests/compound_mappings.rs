//! Runs the cases in `xplane_uipc/mappings.tests.toml` against the shipped
//! `mappings.toml`, and checks that every compound mapping (an expression
//! mapping with two or more datarefs, or with writes) and each of its parts
//! has a case. See the header of `mappings.tests.toml` for the case format.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use uipc_mapping::{DatarefMapping, FsuipcType, MappingSource, eval_writes, load_mappings};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseFile {
    #[serde(default, rename = "case")]
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    offset: u16,
    name: String,
    /// `datarefs` name → value. Names left out are 0.
    #[serde(default)]
    state: BTreeMap<String, f64>,
    /// Read case: the value the offset serves.
    expect: Option<f64>,
    /// Read case: `update_if_expr` keeps the previous value.
    #[serde(default)]
    expect_retain: bool,
    /// Write case: the value the client writes.
    write: Option<f64>,
    /// Write case: the value written to every `write_exprs` target.
    expect_datarefs: Option<BTreeMap<String, f64>>,
    /// Write case: run count of each command that runs.
    #[serde(default)]
    expect_commands: BTreeMap<String, u32>,
    /// Write case: a non-finite result blocks the write.
    #[serde(default)]
    expect_blocked: bool,
}

impl Case {
    fn label(&self) -> String {
        format!("case '{}' ({:#06X})", self.name, self.offset)
    }

    fn is_write(&self) -> bool {
        self.write.is_some()
    }
}

/// The parts of an expression mapping, borrowed from its source.
struct Expr<'a> {
    datarefs: &'a HashMap<String, (String, Option<i32>)>,
    expr: &'a uipc_mapping::Expr,
    update_if_expr: Option<&'a uipc_mapping::Expr>,
    write_exprs: &'a BTreeMap<String, uipc_mapping::Expr>,
    write_commands: &'a BTreeMap<String, uipc_mapping::Expr>,
}

fn expr_parts(m: &DatarefMapping) -> Option<Expr<'_>> {
    match &m.source {
        MappingSource::Expr {
            datarefs,
            expr,
            update_if_expr,
            write_exprs,
            write_commands,
        } => Some(Expr {
            datarefs,
            expr,
            update_if_expr: update_if_expr.as_ref(),
            write_exprs,
            write_commands,
        }),
        _ => None,
    }
}

/// An expression mapping with two or more datarefs, or with writes.
fn is_compound(m: &DatarefMapping) -> bool {
    expr_parts(m).is_some_and(|e| {
        e.datarefs.len() >= 2 || !e.write_exprs.is_empty() || !e.write_commands.is_empty()
    })
}

/// The value the plugin would serve for `value`: rounded and wrapped for
/// integer types (as `offset-value-semantics` specifies), unchanged for
/// floats. `None` when it isn't served.
fn served(value: f64, ty: FsuipcType) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    let r = value.round();
    let wrapped = r as i64;
    Some(match ty {
        FsuipcType::U8 => wrapped as u8 as f64,
        FsuipcType::I8 => wrapped as i8 as f64,
        FsuipcType::U16 => wrapped as u16 as f64,
        FsuipcType::I16 => wrapped as i16 as f64,
        FsuipcType::U32 => wrapped as u32 as f64,
        FsuipcType::I32 => wrapped as i32 as f64,
        FsuipcType::U64 if r < 0.0 => wrapped as u64 as f64,
        FsuipcType::U64 => r as u64 as f64,
        FsuipcType::I64 => wrapped as f64,
        FsuipcType::F32 => value as f32 as f64,
        FsuipcType::F64 => value,
        FsuipcType::String => return None,
    })
}

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance * expected.abs().max(1.0)
}

fn is_float(ty: FsuipcType) -> bool {
    matches!(ty, FsuipcType::F32 | FsuipcType::F64)
}

fn vars(e: &Expr, case: &Case) -> HashMap<String, f64> {
    e.datarefs
        .keys()
        .map(|name| (name.clone(), case.state.get(name).copied().unwrap_or(0.0)))
        .collect()
}

/// Problems with the case itself rather than the mapping.
fn validate(case: &Case, m: &DatarefMapping, e: &Expr) -> Vec<String> {
    let mut errors = Vec::new();
    let label = case.label();
    for name in case.state.keys() {
        if !e.datarefs.contains_key(name) {
            errors.push(format!(
                "{}: state name '{}' isn't in datarefs",
                label, name
            ));
        }
    }
    if case.is_write() {
        if case.expect.is_some() || case.expect_retain {
            errors.push(format!(
                "{}: a write case can't have expect or expect_retain",
                label
            ));
        }
        if !m.writable {
            errors.push(format!(
                "{}: write case on a mapping that isn't writable",
                label
            ));
        }
        match (&case.expect_datarefs, case.expect_blocked) {
            (Some(_), true) | (None, false) => errors.push(format!(
                "{}: a write case needs exactly one of expect_datarefs or expect_blocked",
                label
            )),
            _ => {}
        }
        if case.expect_blocked && !case.expect_commands.is_empty() {
            errors.push(format!(
                "{}: expect_blocked can't have expect_commands",
                label
            ));
        }
        for name in case.expect_datarefs.iter().flat_map(|d| d.keys()) {
            if !e.write_exprs.contains_key(name) {
                errors.push(format!(
                    "{}: expect_datarefs name '{}' isn't a write_exprs target",
                    label, name
                ));
            }
        }
        for path in case.expect_commands.keys() {
            if !e.write_commands.contains_key(path) {
                errors.push(format!(
                    "{}: expect_commands '{}' isn't in write_commands",
                    label, path
                ));
            }
        }
    } else {
        if case.expect_datarefs.is_some() || !case.expect_commands.is_empty() || case.expect_blocked
        {
            errors.push(format!(
                "{}: a read case can't have write expectations (is 'write' missing?)",
                label
            ));
        }
        if case.expect.is_some() == case.expect_retain {
            errors.push(format!(
                "{}: a read case needs exactly one of expect or expect_retain",
                label
            ));
        }
        if case.expect_retain && e.update_if_expr.is_none() {
            errors.push(format!(
                "{}: expect_retain on a mapping without update_if_expr",
                label
            ));
        }
    }
    errors
}

fn run_read(case: &Case, m: &DatarefMapping, e: &Expr) -> Option<String> {
    let label = case.label();
    let vars = vars(e, case);
    let retained = e.update_if_expr.is_some_and(|c| c.eval(&vars) <= 0.0);
    if case.expect_retain {
        return (!retained).then(|| {
            format!(
                "{}: expected the value to be retained, but update_if_expr is true and expr gives {}",
                label,
                e.expr.eval(&vars)
            )
        });
    }
    let expected = case.expect?;
    if retained {
        return Some(format!(
            "{}: expected {}, but update_if_expr is false so the value is retained",
            label, expected
        ));
    }
    let raw = e.expr.eval(&vars);
    let ok = match served(raw, m.fsuipc_type) {
        Some(actual) if is_float(m.fsuipc_type) => close(actual, expected, 1e-6),
        Some(actual) => actual == expected,
        None => false,
    };
    (!ok).then(|| {
        format!(
            "{}: expected {}, got {} (expr gives {})",
            label,
            expected,
            served(raw, m.fsuipc_type).map_or("nothing served".into(), |v| v.to_string()),
            raw
        )
    })
}

fn run_write(case: &Case, e: &Expr) -> Vec<String> {
    let label = case.label();
    let mut vars = vars(e, case);
    vars.insert("value".into(), case.write.unwrap_or_default());
    let plan = eval_writes(e.write_exprs, e.write_commands, &vars);
    if case.expect_blocked {
        return match plan {
            Ok(plan) => vec![format!(
                "{}: expected the write to be blocked, but it writes {:?} and runs {:?}",
                label, plan.datarefs, plan.commands
            )],
            Err(_) => vec![],
        };
    }
    let plan = match plan {
        Ok(plan) => plan,
        Err(err) => return vec![format!("{}: write was blocked: {}", label, err)],
    };
    let mut failures = Vec::new();
    let expected = case.expect_datarefs.clone().unwrap_or_default();
    for (target, actual) in &plan.datarefs {
        match expected.get(*target) {
            None => failures.push(format!(
                "{}: expect_datarefs is missing target '{}' (written as {})",
                label, target, actual
            )),
            Some(&want) if !close(*actual, want, 1e-9) => failures.push(format!(
                "{}: '{}' expected {}, got {}",
                label, target, want, actual
            )),
            Some(_) => {}
        }
    }
    for (path, runs) in &plan.commands {
        let want = case.expect_commands.get(*path).copied().unwrap_or(0);
        if *runs != want {
            failures.push(format!(
                "{}: command '{}' expected to run {} time(s), ran {}",
                label, path, want, runs
            ));
        }
    }
    failures
}

/// Gaps in the cases for each compound mapping.
fn coverage(mappings: &[DatarefMapping], cases: &[Case]) -> Vec<String> {
    let mut gaps = Vec::new();
    let mut compound: Vec<&DatarefMapping> = mappings.iter().filter(|m| is_compound(m)).collect();
    compound.sort_by_key(|m| m.offset);
    for m in compound {
        let e = expr_parts(m).expect("compound mappings are expression mappings");
        let offset = format!("{:#06X}", m.offset);
        let (writes, reads): (Vec<&Case>, Vec<&Case>) = cases
            .iter()
            .filter(|c| c.offset == m.offset)
            .partition(|c| c.is_write());
        if reads.is_empty() {
            gaps.push(format!("{}: no read case", offset));
        }
        if m.writable && writes.is_empty() {
            gaps.push(format!("{}: no write case", offset));
        }
        let read_vars: BTreeSet<String> = e
            .expr
            .vars()
            .into_iter()
            .chain(e.update_if_expr.map(|c| c.vars()).unwrap_or_default())
            .filter(|v| e.datarefs.contains_key(v))
            .collect();
        for var in read_vars {
            let exercised = reads
                .iter()
                .any(|c| c.state.get(&var).is_some_and(|v| *v != 0.0));
            if !exercised && !reads.is_empty() {
                gaps.push(format!(
                    "{}: '{}' is never non-zero in a read case",
                    offset, var
                ));
            }
        }
        for (target, expr) in e.write_exprs {
            // A constant target (such as `UseSys = "0"`) can't be anything
            // else, and every write case already checks it.
            if expr.vars().is_empty() {
                continue;
            }
            let exercised = writes.iter().any(|c| {
                c.expect_datarefs
                    .as_ref()
                    .and_then(|d| d.get(target))
                    .is_some_and(|v| *v != 0.0)
            });
            if !exercised && !writes.is_empty() {
                gaps.push(format!(
                    "{}: target '{}' is never written non-zero in a write case",
                    offset, target
                ));
            }
        }
        for path in e.write_commands.keys() {
            let exercised = writes
                .iter()
                .any(|c| c.expect_commands.get(path).is_some_and(|n| *n > 0));
            if !exercised && !writes.is_empty() {
                gaps.push(format!(
                    "{}: command '{}' never runs in a write case",
                    offset, path
                ));
            }
        }
    }
    gaps
}

/// Every failing case, invalid case and coverage gap.
fn run(mappings: &[DatarefMapping], cases: &[Case]) -> Vec<String> {
    let mut failures = Vec::new();
    for case in cases {
        let Some(m) = mappings.iter().find(|m| m.offset == case.offset) else {
            failures.push(format!("{}: no mapping at this offset", case.label()));
            continue;
        };
        let Some(e) = expr_parts(m).filter(|_| is_compound(m)) else {
            failures.push(format!("{}: the mapping isn't compound", case.label()));
            continue;
        };
        let invalid = validate(case, m, &e);
        if !invalid.is_empty() {
            failures.extend(invalid);
            continue;
        }
        if case.is_write() {
            failures.extend(run_write(case, &e));
        } else {
            failures.extend(run_read(case, m, &e));
        }
    }
    failures.extend(coverage(mappings, cases));
    failures
}

fn parse_cases(text: &str) -> Vec<Case> {
    toml::from_str::<CaseFile>(text)
        .unwrap_or_else(|e| panic!("cannot parse cases: {}", e))
        .cases
}

fn assert_no_failures(failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} failure(s):\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

fn xplane_uipc_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../xplane_uipc")
}

#[test]
fn shipped_compound_mappings() {
    let config = load_mappings(xplane_uipc_dir().join("mappings.toml")).unwrap();
    assert!(config.load_errors.is_empty(), "{:?}", config.load_errors);
    let path = xplane_uipc_dir().join("mappings.tests.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read '{}': {}", path.display(), e));
    assert_no_failures(&run(&config.mappings, &parse_cases(&text)));
}

// ─── Runner tests, against inline mappings ─────────────────────────────────────

/// Load mappings from inline TOML through a temporary file.
fn inline_mappings(toml: &str) -> Vec<DatarefMapping> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "compound_mappings_{}_{}.toml",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, toml).unwrap();
    let config = load_mappings(&path);
    let _ = std::fs::remove_file(&path);
    let config = config.unwrap();
    assert!(config.load_errors.is_empty(), "{:?}", config.load_errors);
    config.mappings
}

/// Two lights in a bitmask, each with a write and a command, plus a
/// read-only single-dataref expression and an update_if_expr mapping.
const LIGHTS: &str = r#"
[[mapping]]
offset      = 0x0D0C
fsuipc_type = "u16"
datarefs    = { Nav = "sim/test/nav", Logo = "sim/test/logo" }
expr        = "$Nav 1 * $Logo 256 * +"
writable    = true
write_exprs = { Nav = "$value 1 & 0 !=", Logo = "$value 256 & 0 !=" }
write_commands = { "addon/logo_toggle" = "$value 256 & 0 != $Logo !=" }

[[mapping]]
offset      = 0x0574
fsuipc_type = "i32"
datarefs    = { elev = "sim/test/elev" }
expr        = "$elev floor"

[[mapping]]
offset      = 0x030C
fsuipc_type = "i32"
datarefs    = { V = "sim/test/vs", OnGround = "sim/test/on_ground" }
update_if_expr = "$OnGround 0 =="
expr        = "$V 256 *"

[[mapping]]
offset      = 0x3324
fsuipc_type = "i32"
datarefs    = { A = "sim/test/a", B = "sim/test/b" }
expr        = "$A $B +"
"#;

/// Cases that fully cover `LIGHTS`.
const COVERED: &str = r#"
[[case]]
offset = 0x0D0C
name   = "nav and logo"
state  = { Nav = 1, Logo = 1 }
expect = 257

[[case]]
offset = 0x0D0C
name   = "logo on, nav kept"
state  = { Nav = 1 }
write  = 257
expect_datarefs = { Nav = 1, Logo = 1 }
expect_commands = { "addon/logo_toggle" = 1 }

[[case]]
offset = 0x030C
name   = "in the air"
state  = { V = -2 }
expect = -512

[[case]]
offset = 0x030C
name   = "on the ground"
state  = { OnGround = 1 }
expect_retain = true

[[case]]
offset = 0x3324
name   = "sum"
state  = { A = 1023.4999, B = 0.5 }
expect = 1024
"#;

fn run_inline(cases: &str) -> Vec<String> {
    run(&inline_mappings(LIGHTS), &parse_cases(cases))
}

/// The failures that mention `needle`.
fn failures_with(failures: &[String], needle: &str) -> Vec<String> {
    failures
        .iter()
        .filter(|f| f.contains(needle))
        .cloned()
        .collect()
}

#[test]
fn compound_predicate() {
    let mappings = inline_mappings(LIGHTS);
    let compound = |offset| is_compound(mappings.iter().find(|m| m.offset == offset).unwrap());
    assert!(compound(0x0D0C), "bitmask with writes");
    assert!(compound(0x3324), "two datarefs, read-only");
    assert!(!compound(0x0574), "single-dataref read-only expr");
}

#[test]
fn covered_cases_pass() {
    assert_no_failures(&run_inline(COVERED));
}

#[test]
fn read_mismatch_reports_expected_and_actual() {
    let cases = COVERED.replace("expect = 257", "expect = 256");
    let failures = run_inline(&cases);
    assert_eq!(failures.len(), 1, "{:?}", failures);
    assert!(failures[0].contains("'nav and logo'"), "{}", failures[0]);
    assert!(
        failures[0].contains("expected 256, got 257"),
        "{}",
        failures[0]
    );
}

#[test]
fn read_retained_when_value_expected() {
    let cases = COVERED.replace("expect_retain = true", "expect = 0");
    let failures = run_inline(&cases);
    assert_eq!(failures.len(), 1, "{:?}", failures);
    assert!(failures[0].contains("retained"), "{}", failures[0]);
}

#[test]
fn write_missing_target() {
    let cases = COVERED.replace(
        "{ Nav = 1, Logo = 1 }\nexpect_commands",
        "{ Logo = 1 }\nexpect_commands",
    );
    let failures = failures_with(&run_inline(&cases), "'logo on, nav kept'");
    assert_eq!(failures.len(), 1, "{:?}", failures);
    assert!(
        failures[0].contains("missing target 'Nav'"),
        "{}",
        failures[0]
    );
}

#[test]
fn write_changing_another_part_fails() {
    // Writing logo on must leave nav on; an edit that clears nav is caught.
    let mappings = LIGHTS.replace("Nav = \"$value 1 & 0 !=\"", "Nav = \"$value 2 & 0 !=\"");
    let failures = run(&inline_mappings(&mappings), &parse_cases(COVERED));
    assert_eq!(failures.len(), 1, "{:?}", failures);
    assert!(
        failures[0].contains("'Nav' expected 1, got 0"),
        "{}",
        failures[0]
    );
}

#[test]
fn write_unexpected_command() {
    let cases = COVERED.replace(
        "state  = { Nav = 1 }\nwrite",
        "state  = { Nav = 1, Logo = 1 }\nwrite",
    ) + r#"
[[case]]
offset = 0x0D0C
name   = "logo already on, nav off"
state  = { Logo = 1 }
write  = 0
expect_datarefs = { Nav = 0, Logo = 0 }
"#;
    let failures = run_inline(&cases);
    // The first write no longer runs the toggle; the second runs it unexpectedly.
    assert_eq!(
        failures_with(&failures, "'logo already on, nav off'"),
        vec!["case 'logo already on, nav off' (0x0D0C): command 'addon/logo_toggle' expected to run 0 time(s), ran 1".to_string()]
    );
    assert_eq!(
        failures_with(&failures, "'logo on, nav kept'").len(),
        1,
        "{:?}",
        failures
    );
}

#[test]
fn write_blocked() {
    let mappings = LIGHTS.replace("Nav = \"$value 1 & 0 !=\"", "Nav = \"10 $value ^\"");
    let cases = r#"
[[case]]
offset = 0x0D0C
name   = "overflow"
write  = 400
expect_blocked = true

[[case]]
offset = 0x0D0C
name   = "not blocked"
write  = 1
expect_blocked = true
"#;
    let failures = run(&inline_mappings(&mappings), &parse_cases(cases));
    assert!(
        failures_with(&failures, "'overflow'").is_empty(),
        "{:?}",
        failures
    );
    assert_eq!(
        failures_with(&failures, "'not blocked'").len(),
        1,
        "{:?}",
        failures
    );
}

#[test]
fn invalid_cases() {
    let check = |case: &str, needle: &str| {
        let failures = run_inline(&format!("{}\n[[case]]\nname = \"bad\"\n{}", COVERED, case));
        assert_eq!(failures.len(), 1, "{}: {:?}", case, failures);
        assert!(failures[0].contains("'bad'"), "{}", failures[0]);
        assert!(failures[0].contains(needle), "{}", failures[0]);
    };
    check("offset = 0x1234\nexpect = 0", "no mapping");
    check("offset = 0x0574\nexpect = 0", "isn't compound");
    check(
        "offset = 0x0D0C\nstate = { Nv = 1 }\nexpect = 0",
        "'Nv' isn't in datarefs",
    );
    check(
        "offset = 0x3324\nwrite = 1\nexpect_datarefs = {}",
        "isn't writable",
    );
    check(
        "offset = 0x0D0C\nwrite = 1\nexpect_datarefs = { Bcn = 0 }",
        "'Bcn' isn't a write_exprs target",
    );
    check(
        "offset = 0x0D0C\nwrite = 1\nexpect_datarefs = { Nav = 1 }\nexpect_commands = { \"a/b\" = 1 }",
        "'a/b' isn't in write_commands",
    );
    check(
        "offset = 0x0D0C\nwrite = 1",
        "exactly one of expect_datarefs or expect_blocked",
    );
    check(
        "offset = 0x0D0C\nwrite = 1\nexpect = 1\nexpect_blocked = true",
        "can't have expect",
    );
    check("offset = 0x0D0C", "exactly one of expect or expect_retain");
    check(
        "offset = 0x0D0C\nexpect = 0\nexpect_blocked = true",
        "is 'write' missing?",
    );
    check(
        "offset = 0x3324\nexpect_retain = true",
        "without update_if_expr",
    );
}

#[test]
fn misspelled_key_is_a_parse_error() {
    let result =
        toml::from_str::<CaseFile>("[[case]]\noffset = 1\nname = \"x\"\nexpect_dataref = {}\n");
    assert!(result.unwrap_err().to_string().contains("expect_dataref"));
}

#[test]
fn coverage_gaps() {
    // Drop logo from the read case and the whole write case.
    let cases = COVERED
        .replace(
            "state  = { Nav = 1, Logo = 1 }\nexpect = 257",
            "state  = { Nav = 1 }\nexpect = 1",
        )
        .replace(
            "[[case]]\noffset = 0x0D0C\nname   = \"logo on, nav kept\"",
            "[[case]]\noffset = 0x9999\nname   = \"gone\"",
        );
    let failures = run_inline(&cases);
    assert!(
        failures.contains(&"0x0D0C: 'Logo' is never non-zero in a read case".to_string()),
        "{:?}",
        failures
    );
    assert!(
        failures.contains(&"0x0D0C: no write case".to_string()),
        "{:?}",
        failures
    );
}

#[test]
fn coverage_of_write_parts() {
    let cases = COVERED.replace(
        "state  = { Nav = 1 }\nwrite  = 257\nexpect_datarefs = { Nav = 1, Logo = 1 }\nexpect_commands = { \"addon/logo_toggle\" = 1 }",
        "write  = 0\nexpect_datarefs = { Nav = 0, Logo = 0 }",
    );
    let failures = run_inline(&cases);
    assert_eq!(
        failures,
        vec![
            "0x0D0C: target 'Logo' is never written non-zero in a write case".to_string(),
            "0x0D0C: target 'Nav' is never written non-zero in a write case".to_string(),
            "0x0D0C: command 'addon/logo_toggle' never runs in a write case".to_string(),
        ]
    );
}

#[test]
fn empty_case_file_lists_every_compound_offset() {
    let failures = run_inline("");
    assert_eq!(
        failures,
        vec![
            "0x030C: no read case".to_string(),
            "0x0D0C: no read case".to_string(),
            "0x0D0C: no write case".to_string(),
            "0x3324: no read case".to_string(),
        ]
    );
}

#[test]
fn constant_write_target_needs_no_non_zero_case() {
    let mappings = inline_mappings(
        r#"
[[mapping]]
offset      = 0x023B
fsuipc_type = "u8"
datarefs    = { H = "sim/test/h", Z = "sim/test/z", UseSys = "sim/test/use_sys" }
expr        = "$H"
writable    = true
write_exprs = { Z = "$value 3600 * $Z 3600 % +", UseSys = "0" }
"#,
    );
    let cases = r#"
[[case]]
offset = 0x023B
name   = "hour"
state  = { H = 12 }
expect = 12

[[case]]
offset = 0x023B
name   = "write hour"
state  = { Z = 45296.5, UseSys = 1 }
write  = 1
expect_datarefs = { Z = 5696.5, UseSys = 0 }
"#;
    assert_no_failures(&run(&mappings, &parse_cases(cases)));
}
