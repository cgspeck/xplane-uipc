use std::collections::{HashMap, HashSet};

use uipc_mapping::{DatarefMapping, FsuipcType, MappingSource};

pub struct EvalEngine {
    pub mappings: Vec<DatarefMapping>,
    pub state: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct MappingResult {
    pub offset: u16,
    pub fsuipc_type: FsuipcType,
    pub writable: bool,
    pub source: MappingSource,
    /// (display_key, resolved_path, value) for each input variable
    pub inputs: Vec<(String, String, f64)>,
    pub fsuipc_value: Option<f64>,
}

impl EvalEngine {
    pub fn new(mappings: Vec<DatarefMapping>, state: HashMap<String, f64>) -> Self {
        Self { mappings, state }
    }

    pub fn evaluate_all(&self) -> Vec<MappingResult> {
        self.mappings.iter().map(|m| self.evaluate_one(m)).collect()
    }

    fn evaluate_one(&self, m: &DatarefMapping) -> MappingResult {
        match &m.source {
            MappingSource::Simple {
                dataref_path,
                array_index: _,
                scale,
                offset_add,
            } => {
                let val = self.state.get(dataref_path).copied();
                let inputs = vec![(
                    dataref_path.clone(),
                    dataref_path.clone(),
                    val.unwrap_or(0.0),
                )];
                MappingResult {
                    offset: m.offset,
                    fsuipc_type: m.fsuipc_type,
                    writable: m.writable,
                    source: m.source.clone(),
                    inputs,
                    fsuipc_value: val.map(|v| v * scale + offset_add),
                }
            }
            MappingSource::Expr { datarefs, expr } => {
                let mut vars = HashMap::new();
                let mut inputs = Vec::with_capacity(datarefs.len());
                // datarefs: name -> (path, array_index)
                let mut sorted: Vec<(String, &(String, Option<i32>))> =
                    datarefs.iter().map(|(k, v)| (k.clone(), v)).collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));

                for (name, (path, _idx)) in &sorted {
                    let val = self.state.get(path.as_str()).copied().unwrap_or(0.0);
                    inputs.push((name.clone(), path.clone(), val));
                    vars.insert(name.clone(), val);
                }

                MappingResult {
                    offset: m.offset,
                    fsuipc_type: m.fsuipc_type,
                    writable: m.writable,
                    source: m.source.clone(),
                    inputs,
                    fsuipc_value: Some(expr.eval(&vars)),
                }
            }
            MappingSource::Static { static_value } => MappingResult {
                offset: m.offset,
                fsuipc_type: m.fsuipc_type,
                writable: m.writable,
                source: m.source.clone(),
                inputs: vec![],
                fsuipc_value: Some(*static_value),
            },
            MappingSource::StaticStr { .. } => MappingResult {
                offset: m.offset,
                fsuipc_type: m.fsuipc_type,
                writable: m.writable,
                source: m.source.clone(),
                inputs: vec![],
                fsuipc_value: None,
            },
        }
    }

    pub fn missing_keys(&self) -> Vec<String> {
        let mut keys: HashSet<String> = HashSet::new();
        for m in &self.mappings {
            match &m.source {
                MappingSource::Simple { dataref_path, .. } => {
                    keys.insert(dataref_path.clone());
                }
                MappingSource::Expr { datarefs, .. } => {
                    for (_name, (path, _idx)) in datarefs {
                        keys.insert(path.clone());
                    }
                }
                MappingSource::Static { .. } => {}
                MappingSource::StaticStr { .. } => {}
            }
        }
        let mut missing: Vec<String> = keys
            .into_iter()
            .filter(|k| !self.state.contains_key(k))
            .collect();
        missing.sort();
        missing
    }

    pub fn all_referenced_keys(&self) -> Vec<String> {
        let mut keys = HashSet::new();
        for m in &self.mappings {
            match &m.source {
                MappingSource::Simple { dataref_path, .. } => {
                    keys.insert(dataref_path.clone());
                }
                MappingSource::Expr { datarefs, .. } => {
                    for (_name, (path, _idx)) in datarefs {
                        keys.insert(path.clone());
                    }
                }
                MappingSource::Static { .. } => {}
                MappingSource::StaticStr { .. } => {}
            }
        }
        let mut sorted: Vec<String> = keys.into_iter().collect();
        sorted.sort();
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uipc_mapping::{DatarefMapping, Expr};

    fn make_mapping(
        offset: u16,
        fsuipc_type: FsuipcType,
        source: MappingSource,
        writable: bool,
    ) -> DatarefMapping {
        let size = match fsuipc_type {
            FsuipcType::String => 0,
            _ => fsuipc_type.size(),
        };
        DatarefMapping {
            offset,
            fsuipc_type,
            size,
            source,
            writable,
        }
    }

    #[test]
    fn test_simple_mapping() {
        let mappings = vec![make_mapping(
            0x02BC,
            FsuipcType::I32,
            MappingSource::Simple {
                dataref_path: "sim/test/ias".into(),
                array_index: None,
                scale: 128.0,
                offset_add: 0.0,
            },
            false,
        )];

        let mut state = HashMap::new();
        state.insert("sim/test/ias".into(), 250.0);
        let engine = EvalEngine::new(mappings, state);
        let results = engine.evaluate_all();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].offset, 0x02BC);
        assert_eq!(results[0].fsuipc_value, Some(32000.0));
        assert_eq!(results[0].inputs.len(), 1);
        assert_eq!(results[0].inputs[0].0, "sim/test/ias");
        assert_eq!(results[0].inputs[0].2, 250.0);
    }

    #[test]
    fn test_simple_mapping_missing_key() {
        let mappings = vec![make_mapping(
            0x02BC,
            FsuipcType::I32,
            MappingSource::Simple {
                dataref_path: "sim/test/ias".into(),
                array_index: None,
                scale: 128.0,
                offset_add: 0.0,
            },
            false,
        )];

        let state = HashMap::new();
        let engine = EvalEngine::new(mappings, state);
        let results = engine.evaluate_all();

        assert_eq!(results[0].fsuipc_value, None);
    }

    #[test]
    fn test_expr_mapping() {
        let mut datarefs = HashMap::new();
        datarefs.insert("Nav".into(), ("sim/test/nav".into(), None));
        datarefs.insert("Bcn".into(), ("sim/test/bcn".into(), None));

        let mappings = vec![make_mapping(
            0x0D0C,
            FsuipcType::U16,
            MappingSource::Expr {
                datarefs,
                expr: Expr::parse("$Nav 1 * $Bcn 2 * +").unwrap(),
            },
            false,
        )];

        let mut state = HashMap::new();
        state.insert("sim/test/nav".into(), 1.0);
        state.insert("sim/test/bcn".into(), 1.0);
        let engine = EvalEngine::new(mappings, state);
        let results = engine.evaluate_all();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].fsuipc_value, Some(3.0));
        assert_eq!(results[0].inputs.len(), 2);
    }

    #[test]
    fn test_expr_mapping_missing_var() {
        let mut datarefs = HashMap::new();
        datarefs.insert("Nav".into(), ("sim/test/nav".into(), None));

        let mappings = vec![make_mapping(
            0x0D0C,
            FsuipcType::U16,
            MappingSource::Expr {
                datarefs,
                expr: Expr::parse("$Nav 1 * 2 +").unwrap(),
            },
            false,
        )];

        let state = HashMap::new();
        let engine = EvalEngine::new(mappings, state);
        let results = engine.evaluate_all();

        assert_eq!(results[0].fsuipc_value, Some(2.0)); // Nav defaults to 0
    }

    #[test]
    fn test_static_mapping() {
        let mappings = vec![make_mapping(
            0x3304,
            FsuipcType::U32,
            MappingSource::Static {
                static_value: 0x50000008 as f64,
            },
            false,
        )];

        let state = HashMap::new();
        let engine = EvalEngine::new(mappings, state);
        let results = engine.evaluate_all();

        assert_eq!(results[0].fsuipc_value, Some(0x50000008 as f64));
        assert!(results[0].inputs.is_empty());
    }

    #[test]
    fn test_missing_keys() {
        let mappings = vec![
            make_mapping(
                0x02BC,
                FsuipcType::I32,
                MappingSource::Simple {
                    dataref_path: "sim/test/ias".into(),
                    array_index: None,
                    scale: 1.0,
                    offset_add: 0.0,
                },
                false,
            ),
            make_mapping(
                0x02C8,
                FsuipcType::I32,
                MappingSource::Simple {
                    dataref_path: "sim/test/vvi".into(),
                    array_index: None,
                    scale: 1.0,
                    offset_add: 0.0,
                },
                false,
            ),
        ];

        let mut state = HashMap::new();
        state.insert("sim/test/ias".into(), 100.0);
        let engine = EvalEngine::new(mappings, state);
        let missing = engine.missing_keys();

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "sim/test/vvi");
    }

    #[test]
    fn test_all_referenced_keys() {
        let mut datarefs = HashMap::new();
        datarefs.insert("Nav".into(), ("sim/test/nav".into(), None));
        datarefs.insert("Bcn".into(), ("sim/test/bcn".into(), None));

        let mappings = vec![
            make_mapping(
                0x02BC,
                FsuipcType::I32,
                MappingSource::Simple {
                    dataref_path: "sim/test/ias".into(),
                    array_index: None,
                    scale: 1.0,
                    offset_add: 0.0,
                },
                false,
            ),
            make_mapping(
                0x0D0C,
                FsuipcType::U16,
                MappingSource::Expr {
                    datarefs,
                    expr: Expr::parse("$Nav 1 * $Bcn 2 * +").unwrap(),
                },
                false,
            ),
            make_mapping(
                0x3304,
                FsuipcType::U32,
                MappingSource::Static { static_value: 1.0 },
                false,
            ),
        ];

        let engine = EvalEngine::new(mappings, HashMap::new());
        let keys = engine.all_referenced_keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"sim/test/ias".into()));
        assert!(keys.contains(&"sim/test/nav".into()));
        assert!(keys.contains(&"sim/test/bcn".into()));
    }
}
