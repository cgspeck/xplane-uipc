use std::collections::HashMap;
use std::path::Path;

use crate::eval::MappingResult;

pub fn load_state<P: AsRef<Path>>(path: P) -> Result<HashMap<String, f64>, String> {
    let content = std::fs::read_to_string(path.as_ref())
        .map_err(|e| format!("cannot read '{}': {}", path.as_ref().display(), e))?;

    let mut map = HashMap::new();
    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, val_str) = line.split_once(',').ok_or_else(|| {
            format!(
                "{}:{}: expected 'key,value' format",
                path.as_ref().display(),
                lineno + 1
            )
        })?;

        let key = key.trim().to_string();
        let val: f64 = val_str.trim().parse().map_err(|e| {
            format!(
                "{}:{}: cannot parse value '{}': {}",
                path.as_ref().display(),
                lineno + 1,
                val_str.trim(),
                e
            )
        })?;
        map.insert(key, val);
    }
    Ok(map)
}

pub fn write_state<P: AsRef<Path>>(
    path: P,
    state: &HashMap<String, f64>,
    all_keys: &[String],
) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::File::create(path.as_ref())
        .map_err(|e| format!("cannot create '{}': {}", path.as_ref().display(), e))?;

    for key in all_keys {
        let val = state.get(key).copied().unwrap_or(0.0);
        writeln!(file, "{},{}", key, val).map_err(|e| format!("write error: {}", e))?;
    }
    Ok(())
}

pub fn write_fsuipc_output<P: AsRef<Path>>(
    path: P,
    results: &[MappingResult],
) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::File::create(path.as_ref())
        .map_err(|e| format!("cannot create '{}': {}", path.as_ref().display(), e))?;

    writeln!(file, "offset,type,value,writable").map_err(|e| format!("write error: {}", e))?;

    for r in results {
        let val = r.fsuipc_value.unwrap_or(0.0);
        writeln!(file, "0x{:04X},{},{}", r.offset, r.fsuipc_type_str(), val)
            .map_err(|e| format!("write error: {}", e))?;
    }
    Ok(())
}

impl MappingResult {
    fn fsuipc_type_str(&self) -> &'static str {
        use uipc_mapping::FsuipcType;
        match self.fsuipc_type {
            FsuipcType::I8 => "i8",
            FsuipcType::U8 => "u8",
            FsuipcType::I16 => "i16",
            FsuipcType::U16 => "u16",
            FsuipcType::I32 => "i32",
            FsuipcType::U32 => "u32",
            FsuipcType::I64 => "i64",
            FsuipcType::U64 => "u64",
            FsuipcType::F32 => "f32",
            FsuipcType::F64 => "f64",
            FsuipcType::String => "string",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_state_basic() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("uipc_debug_test_state_{}", std::process::id()));
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(f, "sim/test/ias,250.0").unwrap();
        writeln!(f, "sim/test/vvi,-1.5").unwrap();
        writeln!(f, "  # this is a comment").unwrap();
        writeln!(f, "sim/test/nav,1").unwrap();

        let map = load_state(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(map.get("sim/test/ias"), Some(&250.0));
        assert_eq!(map.get("sim/test/vvi"), Some(&-1.5));
        assert_eq!(map.get("sim/test/nav"), Some(&1.0));
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_load_state_invalid_line() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("uipc_debug_test_bad_{}", std::process::id()));
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(f, "no comma here").unwrap();

        let result = load_state(&tmp);
        std::fs::remove_file(&tmp).ok();
        assert!(result.is_err());
    }

    #[test]
    fn test_load_state_invalid_number() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("uipc_debug_test_badnum_{}", std::process::id()));
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(f, "sim/test/ias,not_a_number").unwrap();

        let result = load_state(&tmp);
        std::fs::remove_file(&tmp).ok();
        assert!(result.is_err());
    }

    #[test]
    fn test_write_state_round_trip() {
        let mut state = HashMap::new();
        state.insert("sim/test/a".into(), 1.0);
        state.insert("sim/test/b".into(), 2.0);

        let all_keys = vec![
            "sim/test/a".into(),
            "sim/test/b".into(),
            "sim/test/c".into(),
        ];

        let mut tmp = std::env::temp_dir();
        tmp.push(format!("uipc_debug_test_write_{}", std::process::id()));

        write_state(&tmp, &state, &all_keys).unwrap();

        let loaded = load_state(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(loaded.get("sim/test/a"), Some(&1.0));
        assert_eq!(loaded.get("sim/test/b"), Some(&2.0));
        assert_eq!(loaded.get("sim/test/c"), Some(&0.0)); // filled with 0
    }
}
