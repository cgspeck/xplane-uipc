use serde::Deserialize;
use std::collections::HashMap;
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
        array_index: i32,
        scale: f64,
        offset_add: f64,
    },
    Static {
        static_value: f64,
    },
    Expr {
        datarefs: HashMap<String, (String, i32)>,
        expr: Expr,
    },
}

#[derive(Debug, Clone)]
pub struct DatarefMapping {
    pub offset: u16,
    pub fsuipc_type: FsuipcType,
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

    static_value: Option<f64>,

    dataref: Option<String>,
    #[serde(default = "default_scale")]
    scale: f64,
    #[serde(default = "default_offset_add")]
    offset_add: f64,

    datarefs: Option<HashMap<String, String>>,
    expr: Option<String>,

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
        let end = r.offset as usize + r.fsuipc_type.size();
        if end > crate::FSUIPC_DATA_SIZE {
            load_errors.push(format!(
                "offset 0x{:04X} + {} bytes exceeds FSUIPC_DATA_SIZE (0x10000)",
                r.offset,
                r.fsuipc_type.size()
            ));
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
            let mut datarefs = HashMap::new();
            for (name, path_str) in raw_refs {
                let (p, idx) = parse_dataref_with_index(&path_str);
                datarefs.insert(name, (p, idx));
            }
            MappingSource::Expr { datarefs, expr }
        } else if let Some(dr) = r.dataref {
            let (path, idx) = parse_dataref_with_index(&dr);
            MappingSource::Simple {
                dataref_path: path,
                array_index: idx,
                scale: r.scale,
                offset_add: r.offset_add,
            }
        } else if let Some(sv) = r.static_value {
            MappingSource::Static { static_value: sv }
        } else {
            load_errors.push(format!(
                "offset 0x{:04X}: must have 'dataref', 'expr', or 'static_value'",
                r.offset
            ));
            continue;
        };

        mappings.push(DatarefMapping {
            offset: r.offset,
            fsuipc_type: r.fsuipc_type,
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

pub fn parse_dataref_with_index(s: &str) -> (String, i32) {
    if let Some(bracket) = s.rfind('[') {
        if s.ends_with(']') {
            let idx_str = &s[bracket + 1..s.len() - 1];
            if let Ok(idx) = idx_str.parse::<i32>() {
                return (s[..bracket].to_string(), idx);
            }
        }
    }
    (s.to_string(), -1)
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
}
