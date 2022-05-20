// Copyright (c) 2021 ESR Labs GmbH. All rights reserved.
//
// NOTICE:  All information contained herein is, and remains
// the property of E.S.R.Labs and its suppliers, if any.
// The intellectual and technical concepts contained herein are
// proprietary to E.S.R.Labs and its suppliers and may be covered
// by German and Foreign Patents, patents in process, and are protected
// by trade secret or copyright law.
// Dissemination of this information or reproduction of this material
// is strictly forbidden unless prior written permission is obtained
// from E.S.R.Labs.

//! # filter definitions for filtering dlt messages
use crate::dlt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io::Read};

/// Describes what DLT message to filter out based on log-level and app/ecu/context-id
///
/// It is possible to define filters like:
/// - `app-id == "abc" && log-level <= WARN OR app-id == "foo" && log-level <= DEBUG`
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DltFilterConfig {
    /// only select log entries with level MIN_LEVEL and more severe
    ///
    /// ``` text
    ///  1 => FATAL
    ///  2 => ERROR
    ///  3 => WARN
    ///  4 => INFO
    ///  5 => DEBUG
    ///  6 => VERBOSE
    /// ```
    /// what app-ids with associated log-levels should be allowed.
    pub app_ids: Option<Vec<(String, u8)>>,
    /// what ecu-ids with associated log-levels should be allowed
    pub ecu_ids: Option<Vec<(String, u8)>>,
    /// what context-ids with associated log-levels should be allowed
    pub context_ids: Option<Vec<(String, u8)>>,
    /// how many app ids exist in total
    pub app_id_count: i64,
    /// how many context ids exist in total
    pub context_id_count: i64,
}

/// A processed version of the filter configuration that can be used to parse dlt.
/// When a `DltFilterConfig` is received (e.g. as serialized json), this can easily
/// be converted into this processed version using `filter_config.into()`
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessedDltFilterConfig {
    pub app_ids: Option<HashMap<String, dlt::LogLevel>>,
    pub ecu_ids: Option<HashMap<String, dlt::LogLevel>>,
    pub context_ids: Option<HashMap<String, dlt::LogLevel>>,
    pub app_id_count: i64,
    pub context_id_count: i64,
}

impl From<DltFilterConfig> for ProcessedDltFilterConfig {
    fn from(cfg: DltFilterConfig) -> Self {
        ProcessedDltFilterConfig {
            app_ids: cfg.app_ids.map(map_filter_levels),
            ecu_ids: cfg.ecu_ids.map(map_filter_levels),
            context_ids: cfg.context_ids.map(map_filter_levels),
            app_id_count: cfg.app_id_count,
            context_id_count: cfg.context_id_count,
        }
    }
}

fn map_filter_levels(list: Vec<(String, u8)>) -> HashMap<String, dlt::LogLevel> {
    let mut map: HashMap<String, dlt::LogLevel> = HashMap::new();
    for (k, v) in list.into_iter() {
        if let Some(level) = dlt::u8_to_log_level(v) {
            map.insert(k, level);
        }
    }
    map
}

/// Read filter config from a json file
pub fn read_filter_options(f: &mut fs::File) -> Option<DltFilterConfig> {
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .ok()
        .and_then(|_| serde_json::from_str(&contents[..]).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dlt::LogLevel;

    fn flatten_str(string: &str) -> String {
        string.replace(" ", "").replace("\n", "")
    }

    fn assert_str(expected: &str, actual: &str) {
        assert_eq!(flatten_str(expected), flatten_str(actual), "\n{}\n", actual);
    }

    const FILTER_CONFIG_JSON: &str = r#"
        {
            "app_ids":[
                ["A1",6],
                ["A2",5],
                ["A3",4]
            ],
            "ecu_ids":[
                ["E1",3]
            ],
            "context_ids":[
                ["C1",2],
                ["C2",1]
            ],
            "app_id_count":3,
            "context_id_count":2
        }
    "#;

    #[test]
    fn test_serialize_filter_config() {
        let config = DltFilterConfig {
            app_ids: Some(vec![
                (String::from("A1"), 6),
                (String::from("A2"), 5),
                (String::from("A3"), 4),
            ]),
            ecu_ids: Some(vec![(String::from("E1"), 3)]),
            context_ids: Some(vec![(String::from("C1"), 2), (String::from("C2"), 1)]),
            app_id_count: 3,
            context_id_count: 2,
        };

        let json = serde_json::to_string(&config).expect("error on serialize");
        assert_str(FILTER_CONFIG_JSON, &json);
    }

    #[test]
    fn test_deserialize_filter_config() {
        let config: DltFilterConfig = serde_json::from_str(FILTER_CONFIG_JSON).unwrap();

        assert_eq!(
            config,
            DltFilterConfig {
                app_ids: Some(vec![
                    (String::from("A1"), 6),
                    (String::from("A2"), 5),
                    (String::from("A3"), 4)
                ]),
                ecu_ids: Some(vec![(String::from("E1"), 3)]),
                context_ids: Some(vec![(String::from("C1"), 2), (String::from("C2"), 1)]),
                app_id_count: 3,
                context_id_count: 2,
            }
        );
    }

    #[test]
    fn test_convert_to_processed_filter_config() {
        let config: DltFilterConfig = serde_json::from_str(FILTER_CONFIG_JSON).unwrap();
        let processed_config: ProcessedDltFilterConfig = config.into();

        assert_eq!(
            processed_config,
            ProcessedDltFilterConfig {
                app_ids: Some(HashMap::from_iter(vec![
                    (String::from("A1"), LogLevel::Verbose),
                    (String::from("A2"), LogLevel::Debug),
                    (String::from("A3"), LogLevel::Info)
                ])),
                ecu_ids: Some(HashMap::from_iter(vec![(
                    String::from("E1"),
                    LogLevel::Warn
                )])),
                context_ids: Some(HashMap::from_iter(vec![
                    (String::from("C1"), LogLevel::Error),
                    (String::from("C2"), LogLevel::Fatal)
                ])),
                app_id_count: 3,
                context_id_count: 2,
            }
        );
    }
}
