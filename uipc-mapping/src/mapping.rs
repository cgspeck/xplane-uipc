use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::Expr;
use crate::types::FsuipcType;

fn default_scale() -> f64 {
    1.0
}
fn default_offset_add() -> f64 {
    0.0
}
fn default_writable() -> bool {
    false
}
fn default_update_rate() -> f64 {
    20.0
}
fn parse_hex_or_dec<'de, D: serde::Deserializer<'de>>(de: D) -> Result<u16, D::Error> {
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrInt {
        Str(String),
        Int(u16),
    }
    match StrOrInt::deserialize(de)? {
        StrOrInt::Int(n) => Ok(n),
        StrOrInt::Str(s) => {
            let s = s.trim();
            if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                u16::from_str_radix(hex, 16).map_err(|e| D::Error::custom(e.to_string()))
            } else {
                s.parse::<u16>()
                    .map_err(|e| D::Error::custom(e.to_string()))
            }
        }
    }
}

fn parse_fsuipc_type<'de, D: serde::Deserializer<'de>>(de: D) -> Result<FsuipcType, D::Error> {
    use serde::de::Error;
    let s = String::deserialize(de)?;
    s.parse::<FsuipcType>().map_err(D::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobalSettings {
    #[serde(default = "default_update_rate")]
    pub update_rate_hz: f64,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            update_rate_hz: default_update_rate(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MappingSource {
    Simple {
        dataref_path: String,
        array_index: Option<i32>,
        scale: f64,
        offset_add: f64,
    },
    Static {
        static_value: f64,
    },
    StaticStr {
        static_str: String,
    },
    Expr {
        datarefs: HashMap<String, (String, Option<i32>)>,
        expr: Expr,
        update_if_expr: Option<Expr>,
        /// Target name (a key of `datarefs`) → expression for the value to
        /// write there. Sorted so writes go out in a repeatable order; empty
        /// for read-only mappings.
        write_exprs: BTreeMap<String, Expr>,
        /// Command path → expression for how many times to run it. Sorted;
        /// empty for read-only mappings.
        write_commands: BTreeMap<String, Expr>,
    },
}

#[derive(Debug, Clone)]
pub struct DatarefMapping {
    pub offset: u16,
    pub fsuipc_type: FsuipcType,
    pub size: usize,
    pub source: MappingSource,
    pub writable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMapping {
    #[serde(deserialize_with = "parse_hex_or_dec")]
    offset: u16,
    #[serde(deserialize_with = "parse_fsuipc_type")]
    fsuipc_type: FsuipcType,

    size: Option<usize>,

    static_value: Option<f64>,
    static_value_str: Option<String>,

    dataref: Option<String>,
    #[serde(default = "default_scale")]
    scale: f64,
    #[serde(default = "default_offset_add")]
    offset_add: f64,

    datarefs: Option<HashMap<String, String>>,
    expr: Option<String>,
    update_if_expr: Option<String>,
    write_exprs: Option<HashMap<String, String>>,
    write_commands: Option<HashMap<String, String>>,

    #[serde(default = "default_writable")]
    writable: bool,
}

#[derive(Debug, Deserialize)]
struct MappingFile {
    #[serde(default, rename = "mapping")]
    mappings: Vec<RawMapping>,
}

#[derive(Debug)]
pub struct MappingConfig {
    pub mappings: Vec<DatarefMapping>,
    pub load_errors: Vec<String>,
}

pub fn load_mappings<P: AsRef<Path>>(path: P) -> Result<MappingConfig, String> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read '{}': {}", path.display(), e))?;
    let raw: MappingFile = toml::from_str(&text)
        .map_err(|e| format!("TOML parse error in '{}': {}", path.display(), e))?;

    let mut mappings = Vec::with_capacity(raw.mappings.len());
    let mut load_errors = Vec::new();

    for r in raw.mappings {
        let raw_size = match r.fsuipc_type {
            FsuipcType::String => r.size.unwrap_or(0),
            _ => r.fsuipc_type.size(),
        };
        let end = r.offset as usize + raw_size;
        if end > crate::FSUIPC_DATA_SIZE {
            load_errors.push(format!(
                "offset 0x{:04X} + {} bytes exceeds FSUIPC_DATA_SIZE (0x10000)",
                r.offset, raw_size
            ));
            continue;
        }

        if r.fsuipc_type == FsuipcType::String {
            if r.size.is_none() {
                load_errors.push(format!(
                    "offset 0x{:04X}: string type requires 'size' field",
                    r.offset
                ));
                continue;
            }
            if r.expr.is_some() {
                load_errors.push(format!(
                    "offset 0x{:04X}: string type does not support 'expr'",
                    r.offset
                ));
                continue;
            }
            if r.static_value.is_some() {
                load_errors.push(format!(
                    "offset 0x{:04X}: string type uses 'static_value_str' instead of 'static_value'",
                    r.offset
                ));
                continue;
            }
            if r.dataref.is_none() && r.static_value_str.is_none() {
                load_errors.push(format!(
                    "offset 0x{:04X}: string type requires 'dataref' or 'static_value_str'",
                    r.offset
                ));
                continue;
            }
            if r.dataref.is_some() && r.static_value_str.is_some() {
                load_errors.push(format!(
                    "offset 0x{:04X}: string type cannot have both 'dataref' and 'static_value_str'",
                    r.offset
                ));
                continue;
            }
        }

        if r.dataref.is_some() && r.expr.is_some() {
            load_errors.push(format!(
                "offset 0x{:04X}: cannot have both 'dataref' and 'expr'",
                r.offset
            ));
            continue;
        }
        let present_tables = [
            (WriteTable::Exprs, r.write_exprs.is_some()),
            (WriteTable::Commands, r.write_commands.is_some()),
        ];
        let table_error = present_tables.iter().find_map(|&(table, present)| {
            if !present {
                None
            } else if r.expr.is_none() {
                Some(format!("'{}' requires 'expr'", table.name()))
            } else if !r.writable {
                Some(format!("'{}' requires 'writable = true'", table.name()))
            } else {
                None
            }
        });
        if let Some(e) = table_error {
            load_errors.push(format!("offset 0x{:04X}: {}", r.offset, e));
            continue;
        }

        let source = if let Some(expr_src) = r.expr {
            let expr = match Expr::parse(&expr_src) {
                Ok(e) => e,
                Err(e) => {
                    load_errors.push(format!(
                        "offset 0x{:04X}: expr parse error: {}",
                        r.offset, e
                    ));
                    continue;
                }
            };

            let raw_refs = r.datarefs.unwrap_or_default();
            let mut datarefs: HashMap<String, (String, Option<i32>)> = HashMap::new();
            for (name, path_str) in raw_refs {
                let (p, idx) = parse_dataref_with_index(&path_str);
                datarefs.insert(name, (p, idx));
            }

            let update_if_expr = match r.update_if_expr {
                Some(v) => match Expr::parse(&v) {
                    Ok(e) => Some(e),
                    Err(e) => {
                        load_errors.push(format!(
                            "offset 0x{:04X}: update_if_expr parse error: {}",
                            r.offset, e
                        ));
                        continue;
                    }
                },
                None => None,
            };

            if r.writable && r.write_exprs.is_none() && r.write_commands.is_none() {
                load_errors.push(format!(
                    "offset 0x{:04X}: a writable expression mapping needs 'write_exprs' or 'write_commands'",
                    r.offset
                ));
                continue;
            }
            let parsed = parse_write_table(WriteTable::Exprs, r.write_exprs, &datarefs).and_then(
                |write_exprs| {
                    parse_write_table(WriteTable::Commands, r.write_commands, &datarefs)
                        .map(|write_commands| (write_exprs, write_commands))
                },
            );
            let (write_exprs, write_commands) = match parsed {
                Ok(tables) => tables,
                Err(e) => {
                    load_errors.push(format!("offset 0x{:04X}: {}", r.offset, e));
                    continue;
                }
            };

            MappingSource::Expr {
                datarefs,
                expr,
                update_if_expr,
                write_exprs,
                write_commands,
            }
        } else if let Some(dr) = r.dataref {
            // The read is the constant `offset_add`, so a write can't be reversed.
            if r.writable && r.scale == 0.0 {
                load_errors.push(format!(
                    "offset 0x{:04X}: a writable mapping can't have 'scale = 0'",
                    r.offset
                ));
                continue;
            }
            let (path, idx) = parse_dataref_with_index(&dr);
            MappingSource::Simple {
                dataref_path: path,
                array_index: idx,
                scale: r.scale,
                offset_add: r.offset_add,
            }
        } else if let Some(s) = r.static_value_str {
            MappingSource::StaticStr { static_str: s }
        } else if let Some(sv) = r.static_value {
            MappingSource::Static { static_value: sv }
        } else {
            load_errors.push(format!(
                "offset 0x{:04X}: must have 'dataref', 'expr', 'static_value', or 'static_value_str'",
                r.offset
            ));
            continue;
        };

        let size = match r.fsuipc_type {
            FsuipcType::String => r.size.unwrap_or(0),
            _ => r.fsuipc_type.size(),
        };

        mappings.push(DatarefMapping {
            offset: r.offset,
            fsuipc_type: r.fsuipc_type,
            size,
            source,
            writable: r.writable,
        });
    }

    if mappings.is_empty() && !load_errors.is_empty() {
        return Err(format!(
            "no mappings could be loaded from '{}' ({} errors): {}",
            path.display(),
            load_errors.len(),
            load_errors.join("; ")
        ));
    }

    Ok(MappingConfig {
        mappings,
        load_errors,
    })
}

/// The two write tables of an expression mapping.
#[derive(Clone, Copy)]
enum WriteTable {
    /// `write_exprs`: datarefs name → value to write.
    Exprs,
    /// `write_commands`: command path → how many times to run it.
    Commands,
}

impl WriteTable {
    fn name(self) -> &'static str {
        match self {
            WriteTable::Exprs => "write_exprs",
            WriteTable::Commands => "write_commands",
        }
    }
}

/// Parse and check one of a mapping's write tables against its `datarefs`.
/// An absent table is empty.
///
/// Unlike read expressions, where an unknown variable quietly reads as 0.0,
/// a typo here would write zero to a real dataref or run the wrong command,
/// so it's an error.
fn parse_write_table(
    table: WriteTable,
    raw: Option<HashMap<String, String>>,
    datarefs: &HashMap<String, (String, Option<i32>)>,
) -> Result<BTreeMap<String, Expr>, String> {
    let name = table.name();
    let Some(raw) = raw else {
        return Ok(BTreeMap::new());
    };
    if raw.is_empty() {
        return Err(format!("'{}' is empty", name));
    }
    if datarefs.contains_key("value") {
        return Err(format!(
            "'datarefs' can't have an entry named 'value' when '{}' is present",
            name
        ));
    }
    let mut parsed = BTreeMap::new();
    for (key, src) in raw.into_iter().collect::<BTreeMap<_, _>>() {
        match table {
            WriteTable::Exprs if !datarefs.contains_key(&key) => {
                return Err(format!(
                    "{} target '{}' is not a name in 'datarefs'",
                    name, key
                ));
            }
            WriteTable::Commands if !key.contains('/') => {
                return Err(format!("{} key '{}' is not a command path", name, key));
            }
            _ => {}
        }
        let expr =
            Expr::parse(&src).map_err(|e| format!("{} '{}' parse error: {}", name, key, e))?;
        if let Some(var) = expr
            .vars()
            .into_iter()
            .find(|v| v != "value" && !datarefs.contains_key(v))
        {
            return Err(format!(
                "{} '{}' uses unknown variable '${}'",
                name, key, var
            ));
        }
        parsed.insert(key, expr);
    }
    Ok(parsed)
}

pub fn parse_dataref_with_index(s: &str) -> (String, Option<i32>) {
    if let Some(bracket) = s.rfind('[')
        && s.ends_with(']')
    {
        let idx_str = &s[bracket + 1..s.len() - 1];
        if let Ok(idx) = idx_str.parse::<i32>() {
            return (s[..bracket].to_string(), Some(idx));
        }
    }
    (s.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn test_toml(content: &str) -> (std::path::PathBuf, String) {
        let mut dir = std::env::temp_dir();
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let name = format!("uipc_test_{}_{}.toml", std::process::id(), id);
        dir.push(&name);
        let mut f = std::fs::File::create(&dir).unwrap();
        write!(f, "{}", content).unwrap();
        (dir, name)
    }

    #[test]
    fn static_value_alone() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
static_value = 42.0
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.mappings.len(), 1);
        let m = &config.mappings[0];
        assert_eq!(m.offset, 0x1000);
        match &m.source {
            MappingSource::Static { static_value } => assert_eq!(*static_value, 42.0),
            _ => panic!("expected Static source"),
        }
    }

    #[test]
    fn static_value_negative_and_zero() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"f64\"
static_value = -1.5

[[mapping]]
offset      = 0x1008
fsuipc_type = \"i32\"
static_value = 0.0
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.mappings.len(), 2);
        match &config.mappings[0].source {
            MappingSource::Static { static_value } => assert_eq!(*static_value, -1.5),
            _ => panic!("expected Static source"),
        }
        match &config.mappings[1].source {
            MappingSource::Static { static_value } => assert_eq!(*static_value, 0.0),
            _ => panic!("expected Static source"),
        }
    }

    #[test]
    fn priority_expr_over_static() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u32\"
static_value = 99.0
expr        = \"1 2 +\"
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        match &config.mappings[0].source {
            MappingSource::Expr { .. } => {}
            _ => panic!("expected Expr source (priority over Static)"),
        }
    }

    #[test]
    fn priority_dataref_over_static() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"f32\"
dataref     = \"sim/test/dr\"
static_value = 99.0
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        match &config.mappings[0].source {
            MappingSource::Simple { dataref_path, .. } => {
                assert_eq!(dataref_path, "sim/test/dr");
            }
            _ => panic!("expected Simple source (priority over Static)"),
        }
    }

    #[test]
    fn no_source_fields_errors() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u8\"
",
        );
        let result = load_mappings(&path);
        let _ = std::fs::remove_file(&path);

        let err = result.unwrap_err();
        assert!(
            err.contains("static_value"),
            "error should mention static_value: {}",
            err
        );
        assert!(
            err.contains("dataref"),
            "error should mention dataref: {}",
            err
        );
        assert!(err.contains("expr"), "error should mention expr: {}", err);
    }

    #[test]
    fn partial_success_one_bad_mapping() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
static_value = 42.0

[[mapping]]
offset      = 0x1002
fsuipc_type = \"u8\"

[[mapping]]
offset      = 0x1003
fsuipc_type = \"u8\"
static_value = 10.0
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.mappings.len(), 2);
        assert_eq!(config.load_errors.len(), 1);
        assert!(config.load_errors[0].contains("0x1002"));
    }

    #[test]
    fn all_mappings_fail_returns_error() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u8\"

[[mapping]]
offset      = 0x1001
fsuipc_type = \"u8\"
",
        );
        let result = load_mappings(&path);
        let _ = std::fs::remove_file(&path);

        let err = result.unwrap_err();
        assert!(err.contains("no mappings could be loaded"));
        assert!(err.contains("0x1000"));
        assert!(err.contains("0x1001"));
    }

    #[test]
    fn valid_mappings_no_errors() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
static_value = 1.0

[[mapping]]
offset      = 0x1002
fsuipc_type = \"u8\"
dataref     = \"sim/test/dr\"
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.mappings.len(), 2);
        assert!(config.load_errors.is_empty());
    }

    #[test]
    fn expr_parse_error_collected_not_fatal() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
static_value = 5.0

[[mapping]]
offset      = 0x1002
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"invalid @@ expr\"

[[mapping]]
offset      = 0x1004
fsuipc_type = \"u8\"
static_value = 99.0
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.mappings.len(), 2);
        assert_eq!(config.load_errors.len(), 1);
        assert!(config.load_errors[0].contains("0x1002"));
        assert!(config.load_errors[0].to_lowercase().contains("expr"));
    }

    #[test]
    fn string_static_value_str() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x3160
fsuipc_type = \"string\"
size        = 24
static_value_str = \"hello\"
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.mappings.len(), 1);
        let m = &config.mappings[0];
        assert_eq!(m.offset, 0x3160);
        assert_eq!(m.fsuipc_type, FsuipcType::String);
        assert_eq!(m.size, 24);
        match &m.source {
            MappingSource::StaticStr { static_str } => assert_eq!(static_str, "hello"),
            _ => panic!("expected StaticStr source"),
        }
    }

    #[test]
    fn string_dataref() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x3180
fsuipc_type = \"string\"
size        = 40
dataref     = \"sim/test/string_dr\"
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.mappings.len(), 1);
        let m = &config.mappings[0];
        assert_eq!(m.fsuipc_type, FsuipcType::String);
        assert_eq!(m.size, 40);
        match &m.source {
            MappingSource::Simple { dataref_path, .. } => {
                assert_eq!(dataref_path, "sim/test/string_dr")
            }
            _ => panic!("expected Simple source"),
        }
    }

    #[test]
    fn string_missing_size() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x3200
fsuipc_type = \"string\"
static_value_str = \"x\"
",
        );
        let result = load_mappings(&path);
        let _ = std::fs::remove_file(&path);

        let err = result.unwrap_err();
        assert!(err.contains("string type requires 'size'"));
    }

    #[test]
    fn string_with_expr() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x3200
fsuipc_type = \"string\"
size        = 10
expr        = \"1 2 +\"
",
        );
        let result = load_mappings(&path);
        let _ = std::fs::remove_file(&path);

        let err = result.unwrap_err();
        assert!(err.contains("string type does not support 'expr'"));
    }

    #[test]
    fn string_with_static_value_f64() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x3200
fsuipc_type = \"string\"
size        = 10
static_value = 42.0
",
        );
        let result = load_mappings(&path);
        let _ = std::fs::remove_file(&path);

        let err = result.unwrap_err();
        assert!(err.contains("static_value_str' instead of 'static_value'"));
    }

    #[test]
    fn string_with_both_dataref_and_static_value_str() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x3200
fsuipc_type = \"string\"
size        = 10
dataref     = \"sim/test/dr\"
static_value_str = \"hello\"
",
        );
        let result = load_mappings(&path);
        let _ = std::fs::remove_file(&path);

        let err = result.unwrap_err();
        assert!(err.contains("cannot have both 'dataref' and 'static_value_str'"));
    }

    /// Load a file holding one mapping that should be rejected; return the error.
    fn load_error(content: &str) -> String {
        let (path, _name) = test_toml(content);
        let result = load_mappings(&path);
        let _ = std::fs::remove_file(&path);
        let err = result.unwrap_err();
        assert!(
            err.contains("0x1000"),
            "error should name the offset: {}",
            err
        );
        err
    }

    #[test]
    fn write_exprs_fan_out() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x0D0C
fsuipc_type = \"u16\"
datarefs    = { Nav = \"sim/test/nav\", Bcn = \"sim/test/bcn\" }
expr        = \"$Nav 1 * $Bcn 2 * +\"
writable    = true
write_exprs = { Nav = \"$value 1 & 0 !=\", Bcn = \"$value 2 & $Bcn +\" }
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(config.load_errors.is_empty());
        match &config.mappings[0].source {
            MappingSource::Expr { write_exprs, .. } => {
                let targets: Vec<&str> = write_exprs.keys().map(String::as_str).collect();
                assert_eq!(targets, vec!["Bcn", "Nav"]);
            }
            _ => panic!("expected Expr source"),
        }
    }

    #[test]
    fn read_only_expr_has_no_write_exprs() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        match &config.mappings[0].source {
            MappingSource::Expr { write_exprs, .. } => assert!(write_exprs.is_empty()),
            _ => panic!("expected Expr source"),
        }
    }

    #[test]
    fn write_exprs_without_expr() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
static_value = 1.0
writable    = true
write_exprs = { X = \"$value\" }
",
        );
        assert!(err.contains("'write_exprs' requires 'expr'"), "{}", err);
    }

    #[test]
    fn write_exprs_on_simple_mapping() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
dataref     = \"sim/test/dr\"
writable    = true
write_exprs = { X = \"$value\" }
",
        );
        assert!(err.contains("'write_exprs' requires 'expr'"), "{}", err);
    }

    #[test]
    fn write_exprs_not_writable() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
write_exprs = { X = \"$value\" }
",
        );
        assert!(
            err.contains("'write_exprs' requires 'writable = true'"),
            "{}",
            err
        );
    }

    #[test]
    fn writable_expr_without_write_exprs() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
writable    = true
",
        );
        assert!(
            err.contains("needs 'write_exprs' or 'write_commands'"),
            "{}",
            err
        );
    }

    #[test]
    fn write_exprs_empty() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
writable    = true
write_exprs = {}
",
        );
        assert!(err.contains("'write_exprs' is empty"), "{}", err);
    }

    #[test]
    fn write_exprs_unknown_target() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u32\"
datarefs    = { SB = \"sim/test/dr\" }
expr        = \"$SB\"
writable    = true
write_exprs = { Sb = \"$value\" }
",
        );
        assert!(err.contains("'Sb'"), "{}", err);
    }

    #[test]
    fn write_exprs_unknown_variable() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
writable    = true
write_exprs = { X = \"$valu 2 *\" }
",
        );
        assert!(err.contains("valu"), "{}", err);
    }

    #[test]
    fn write_exprs_parse_error() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
writable    = true
write_exprs = { X = \"$value @@\" }
",
        );
        assert!(err.contains("parse error"), "{}", err);
    }

    #[test]
    fn write_exprs_dataref_named_value() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { value = \"sim/test/dr\" }
expr        = \"$value\"
writable    = true
write_exprs = { value = \"$value\" }
",
        );
        assert!(err.contains("named 'value'"), "{}", err);
    }

    #[test]
    fn write_exprs_without_datarefs() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
expr        = \"5\"
writable    = true
write_exprs = { X = \"$value\" }
",
        );
        assert!(err.contains("'X'"), "{}", err);
    }

    #[test]
    fn writable_zero_scale() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
dataref     = \"sim/test/dr\"
scale       = 0.0
writable    = true
",
        );
        assert!(err.contains("'scale = 0'"), "{}", err);
    }

    #[test]
    fn read_only_zero_scale_loads() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
dataref     = \"sim/test/dr\"
scale       = 0.0
offset_add  = 5.0
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(config.load_errors.is_empty());
        assert_eq!(config.mappings.len(), 1);
    }

    #[test]
    fn dataref_and_expr() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
dataref     = \"sim/a\"
scale       = 100.0
datarefs    = { A = \"sim/a\" }
expr        = \"$A\"
",
        );
        assert!(
            err.contains("cannot have both 'dataref' and 'expr'"),
            "{}",
            err
        );
    }

    #[test]
    fn write_commands_only() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x281C
fsuipc_type = \"u32\"
datarefs    = { Bat = \"sim/test/bat\" }
expr        = \"$Bat\"
writable    = true
write_commands = { \"addon/battery_toggle\" = \"$value 0 != $Bat 0 != !=\" }
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(config.load_errors.is_empty(), "{:?}", config.load_errors);
        match &config.mappings[0].source {
            MappingSource::Expr {
                write_exprs,
                write_commands,
                ..
            } => {
                assert!(write_exprs.is_empty());
                let paths: Vec<&str> = write_commands.keys().map(String::as_str).collect();
                assert_eq!(paths, vec!["addon/battery_toggle"]);
            }
            _ => panic!("expected Expr source"),
        }
    }

    #[test]
    fn write_exprs_and_write_commands() {
        let (path, _name) = test_toml(
            "[[mapping]]
offset      = 0x0D0C
fsuipc_type = \"u16\"
datarefs    = { Land = \"sim/test/land\", ZLand = \"addon/land_pos\" }
expr        = \"$Land\"
writable    = true
write_exprs = { Land = \"$value 4 & 0 !=\" }
write_commands = { \"addon/land_toggle\" = \"$value 4 & 0 != $ZLand 0 != !=\" }
",
        );
        let config = load_mappings(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(config.load_errors.is_empty(), "{:?}", config.load_errors);
        match &config.mappings[0].source {
            MappingSource::Expr {
                write_exprs,
                write_commands,
                ..
            } => {
                assert_eq!(write_exprs.len(), 1);
                assert_eq!(write_commands.len(), 1);
            }
            _ => panic!("expected Expr source"),
        }
    }

    #[test]
    fn write_commands_key_not_a_path() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
writable    = true
write_commands = { Toggle = \"1\" }
",
        );
        assert!(err.contains("'Toggle' is not a command path"), "{}", err);
    }

    #[test]
    fn write_commands_unknown_variable() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
writable    = true
write_commands = { \"addon/toggle\" = \"$valu\" }
",
        );
        assert!(err.contains("write_commands"), "{}", err);
        assert!(err.contains("valu"), "{}", err);
    }

    #[test]
    fn write_commands_empty() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
writable    = true
write_commands = {}
",
        );
        assert!(err.contains("'write_commands' is empty"), "{}", err);
    }

    #[test]
    fn write_commands_not_writable() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
datarefs    = { X = \"sim/test/dr\" }
expr        = \"$X\"
write_commands = { \"addon/toggle\" = \"1\" }
",
        );
        assert!(
            err.contains("'write_commands' requires 'writable = true'"),
            "{}",
            err
        );
    }

    #[test]
    fn write_commands_without_expr() {
        let err = load_error(
            "[[mapping]]
offset      = 0x1000
fsuipc_type = \"u16\"
dataref     = \"sim/test/dr\"
writable    = true
write_commands = { \"addon/toggle\" = \"1\" }
",
        );
        assert!(err.contains("'write_commands' requires 'expr'"), "{}", err);
    }

    #[test]
    fn shipped_mappings_load_cleanly() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../xplane_uipc/mappings.toml");
        let config = load_mappings(path).unwrap();
        assert!(
            config.load_errors.is_empty(),
            "load errors: {:?}",
            config.load_errors
        );
    }

    /// Each shipped BCD radio mapping serves the dataref as BCD, and writing
    /// that BCD back gives the same dataref value.
    #[test]
    fn shipped_bcd_radios_round_trip() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../xplane_uipc/mappings.toml");
        let config = load_mappings(path).unwrap();
        let cases: &[(u16, f64, u32)] = &[
            (0x0354, 2770.0, 0x2770),  // transponder
            (0x034E, 12345.0, 0x2345), // COM1 123.45
            (0x3118, 11850.0, 0x1850), // COM2 118.50
            (0x311A, 13690.0, 0x3690), // COM1 standby
            (0x311C, 12180.0, 0x2180), // COM2 standby
            (0x0350, 11345.0, 0x1345), // NAV1 113.45
            (0x0352, 10800.0, 0x0800), // NAV2 108.00
            (0x311E, 11790.0, 0x1790), // NAV1 standby
            (0x3120, 10935.0, 0x0935), // NAV2 standby
        ];
        for &(offset, dataref, bcd) in cases {
            let m = config
                .mappings
                .iter()
                .find(|m| m.offset == offset)
                .unwrap_or_else(|| panic!("no mapping for {:#06X}", offset));
            assert!(m.writable, "{:#06X} not writable", offset);
            let MappingSource::Expr {
                datarefs,
                expr,
                write_exprs,
                write_commands,
                ..
            } = &m.source
            else {
                panic!("{:#06X} isn't an expression mapping", offset);
            };
            let name = datarefs.keys().next().unwrap().clone();
            let read_vars = HashMap::from([(name.clone(), dataref)]);
            assert_eq!(expr.eval(&read_vars), bcd as f64, "read {:#06X}", offset);

            let mut write_vars = read_vars.clone();
            write_vars.insert("value".into(), bcd as f64);
            let plan = crate::eval_writes(write_exprs, write_commands, &write_vars).unwrap();
            assert_eq!(
                plan.datarefs,
                vec![(name.as_str(), dataref)],
                "write {:#06X}",
                offset
            );
        }
    }
}
