// Copyright 2021 by Accenture ESR
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # filter definitions for filtering dlt messages
use crate::dlt;
use std::{collections::HashSet, iter::FromIterator};

/// Describes what DLT message to filter out based on log-level and app/ecu/context-id
///
/// In the current form each filter element is independent from another, i.e. it is
/// not possible to define filters like:
/// - `app-id == "abc" && log-level <= WARN OR app-id == "foo" && log-level <= DEBUG`
///
/// only this is possible:
/// - `app-id is_one_of ["abc","foo"] AND log-level <= DEBUG`
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, Clone)]
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
    pub min_log_level: Option<u8>,
    /// what app ids should be allowed.
    pub app_ids: Option<Vec<String>>,
    /// what ecu ids should be allowed
    pub ecu_ids: Option<Vec<String>>,
    /// what context ids should be allowed
    pub context_ids: Option<Vec<String>>,
    /// how many app ids exist in total
    pub app_id_count: i64,
    /// how many context ids exist in total
    pub context_id_count: i64,
}

/// A processed version of the filter configuration that can be used to parse dlt.
///
/// When a `DltFilterConfig` is received (e.g. as serialized json), this can easily
/// be converted into this processed version using `filter_config.into()`
#[derive(Clone, Debug)]
pub struct ProcessedDltFilterConfig {
    pub min_log_level: Option<dlt::LogLevel>,
    pub app_ids: Option<HashSet<String>>,
    pub ecu_ids: Option<HashSet<String>>,
    pub context_ids: Option<HashSet<String>>,
    pub app_id_count: i64,
    pub context_id_count: i64,
}

impl From<DltFilterConfig> for ProcessedDltFilterConfig {
    fn from(cfg: DltFilterConfig) -> Self {
        ProcessedDltFilterConfig {
            min_log_level: cfg.min_log_level.and_then(dlt::u8_to_log_level),
            app_ids: cfg.app_ids.map(HashSet::from_iter),
            ecu_ids: cfg.ecu_ids.map(HashSet::from_iter),
            context_ids: cfg.context_ids.map(HashSet::from_iter),
            app_id_count: cfg.app_id_count,
            context_id_count: cfg.context_id_count,
        }
    }
}

impl From<&DltFilterConfig> for ProcessedDltFilterConfig {
    fn from(cfg: &DltFilterConfig) -> Self {
        ProcessedDltFilterConfig {
            min_log_level: cfg.min_log_level.and_then(dlt::u8_to_log_level),
            app_ids: cfg.app_ids.as_ref().map(|s| HashSet::from_iter(s.clone())),
            ecu_ids: cfg.ecu_ids.as_ref().map(|s| HashSet::from_iter(s.clone())),
            context_ids: cfg
                .context_ids
                .as_ref()
                .map(|s| HashSet::from_iter(s.clone())),
            app_id_count: cfg.app_id_count,
            context_id_count: cfg.context_id_count,
        }
    }
}

/// Read filter config from a json file. Available only with feature "serialization"
#[cfg(feature = "serialization")]
pub fn read_filter_options(f: &mut std::fs::File) -> Option<DltFilterConfig> {
    use std::io::Read;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .ok()
        .and_then(|_| serde_json::from_str(&contents[..]).ok())
}
