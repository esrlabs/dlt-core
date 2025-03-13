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

//! # rapidly gather statistics info of a dlt source
use crate::{
    dlt::{ExtendedHeader, LogLevel, MessageType, StandardHeader, StorageHeader},
    parse::{dlt_extended_header, dlt_standard_header, dlt_storage_header, DltParseError},
    read::DltMessageReader,
};
use std::io::Read;

/// Trait for a DLT statistics collector.
pub trait StatisticCollector {
    fn collect_statistic(&mut self, statistic: Statistic) -> Result<(), DltParseError>;
}

/// Available statistics on a DLT message.
pub struct Statistic<'a> {
    /// The `LogLevel` of the message, if any.
    pub log_level: Option<LogLevel>,
    /// The `StorageHeader` of the message, if any.
    pub storage_header: Option<StorageHeader>,
    /// The `StandardHeader` of the message.
    pub standard_header: StandardHeader,
    /// The `ExtendedHeader` of the message, if any.
    pub extended_header: Option<ExtendedHeader>,
    /// The remaining payload of the message after all headers.
    pub payload: &'a [u8],
    /// Answers if the message's payload is verbose.
    pub is_verbose: bool,
}

/// Collect DLT statistics from the given reader.
pub fn collect_statistics<S: Read>(
    reader: &mut DltMessageReader<S>,
    collector: &mut impl StatisticCollector,
) -> Result<(), DltParseError> {
    let with_storage_header = reader.with_storage_header();

    loop {
        let slice = reader.next_message_slice()?;
        if slice.is_empty() {
            break;
        }

        let (rest_before_standard_header, storage_header) = if with_storage_header {
            let result = dlt_storage_header(slice)?;
            let rest = result.0;
            let header = if let Some(header) = result.1 {
                Some(header.0)
            } else {
                None
            };
            (rest, header)
        } else {
            (slice, None)
        };

        let (rest_after_standard_header, standard_header) =
            dlt_standard_header(rest_before_standard_header)?;

        let (rest_after_all_headers, extended_header, log_level, is_verbose) =
            if standard_header.has_extended_header {
                let result = dlt_extended_header(rest_after_standard_header)?;
                let rest = result.0;
                let header = result.1;
                let level = match header.message_type {
                    MessageType::Log(level) => Some(level),
                    _ => None,
                };
                let verbose = header.verbose;
                (rest, Some(header), level, verbose)
            } else {
                (rest_after_standard_header, None, None, false)
            };

        collector.collect_statistic(Statistic {
            log_level,
            storage_header,
            standard_header,
            extended_header,
            payload: rest_after_all_headers,
            is_verbose,
        })?;
    }

    Ok(())
}

/// Contains the common DLT statistics.
pub mod common {
    use super::*;
    use rustc_hash::FxHashMap;

    type IdMap = FxHashMap<String, LevelDistribution>;

    /// Collector for the `StatisticInfo` statistics.
    #[derive(Default)]
    pub struct StatisticInfoCollector {
        app_ids: IdMap,
        context_ids: IdMap,
        ecu_ids: IdMap,
        contained_non_verbose: bool,
    }

    impl StatisticInfoCollector {
        /// Finalize and return the collected statistics.
        pub fn collect(self) -> StatisticInfo {
            StatisticInfo {
                app_ids: self
                    .app_ids
                    .into_iter()
                    .collect::<Vec<(String, LevelDistribution)>>(),
                context_ids: self
                    .context_ids
                    .into_iter()
                    .collect::<Vec<(String, LevelDistribution)>>(),
                ecu_ids: self
                    .ecu_ids
                    .into_iter()
                    .collect::<Vec<(String, LevelDistribution)>>(),
                contained_non_verbose: self.contained_non_verbose,
            }
        }
    }

    impl StatisticCollector for StatisticInfoCollector {
        fn collect_statistic(&mut self, statistic: Statistic) -> Result<(), DltParseError> {
            let log_level = statistic.log_level;

            match statistic.standard_header.ecu_id {
                Some(id) => add_for_level(log_level, &mut self.ecu_ids, id),
                None => add_for_level(log_level, &mut self.ecu_ids, "NONE".to_string()),
            };

            if let Some(extended_header) = statistic.extended_header {
                add_for_level(log_level, &mut self.app_ids, extended_header.application_id);
                add_for_level(log_level, &mut self.context_ids, extended_header.context_id);
            }

            self.contained_non_verbose = self.contained_non_verbose || !statistic.is_verbose;

            Ok(())
        }
    }

    /// Some common statistics about collected messages.
    /// Includes the `LevelDistribution` for `app-ids`, `context-ids` and `ecu_ids`.
    #[cfg_attr(
        feature = "serialization",
        derive(serde::Serialize, serde::Deserialize)
    )]
    #[derive(Debug)]
    pub struct StatisticInfo {
        pub app_ids: Vec<(String, LevelDistribution)>,
        pub context_ids: Vec<(String, LevelDistribution)>,
        pub ecu_ids: Vec<(String, LevelDistribution)>,
        pub contained_non_verbose: bool,
    }

    impl StatisticInfo {
        pub fn new() -> Self {
            Self {
                app_ids: vec![],
                context_ids: vec![],
                ecu_ids: vec![],
                contained_non_verbose: false,
            }
        }

        pub fn merge(&mut self, stat: StatisticInfo) {
            StatisticInfo::merge_levels(&mut self.app_ids, stat.app_ids);
            StatisticInfo::merge_levels(&mut self.context_ids, stat.context_ids);
            StatisticInfo::merge_levels(&mut self.ecu_ids, stat.ecu_ids);
            self.contained_non_verbose = self.contained_non_verbose || stat.contained_non_verbose;
        }

        fn merge_levels(
            owner: &mut Vec<(String, LevelDistribution)>,
            incomes: Vec<(String, LevelDistribution)>,
        ) {
            incomes.iter().for_each(|(income_id, income)| {
                if let Some((_, existed)) =
                    owner.iter_mut().find(|(owner_id, _)| owner_id == income_id)
                {
                    existed.merge(income);
                } else {
                    owner.push((income_id.to_owned(), income.clone()));
                }
            });
        }
    }

    impl Default for StatisticInfo {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Shows how many messages per log level where found
    #[cfg_attr(
        feature = "serialization",
        derive(serde::Serialize, serde::Deserialize)
    )]
    #[derive(Debug, Default, Clone)]
    pub struct LevelDistribution {
        pub non_log: usize,
        pub log_fatal: usize,
        pub log_error: usize,
        pub log_warning: usize,
        pub log_info: usize,
        pub log_debug: usize,
        pub log_verbose: usize,
        pub log_invalid: usize,
    }

    impl LevelDistribution {
        pub fn new(level: Option<LogLevel>) -> LevelDistribution {
            let all_zero = Default::default();
            match level {
                None => LevelDistribution {
                    non_log: 1,
                    ..all_zero
                },
                Some(LogLevel::Fatal) => LevelDistribution {
                    log_fatal: 1,
                    ..all_zero
                },
                Some(LogLevel::Error) => LevelDistribution {
                    log_error: 1,
                    ..all_zero
                },
                Some(LogLevel::Warn) => LevelDistribution {
                    log_warning: 1,
                    ..all_zero
                },
                Some(LogLevel::Info) => LevelDistribution {
                    log_info: 1,
                    ..all_zero
                },
                Some(LogLevel::Debug) => LevelDistribution {
                    log_debug: 1,
                    ..all_zero
                },
                Some(LogLevel::Verbose) => LevelDistribution {
                    log_verbose: 1,
                    ..all_zero
                },
                _ => LevelDistribution {
                    log_invalid: 1,
                    ..all_zero
                },
            }
        }

        pub fn merge(&mut self, outside: &LevelDistribution) {
            self.non_log += outside.non_log;
            self.log_fatal += outside.log_fatal;
            self.log_error += outside.log_error;
            self.log_warning += outside.log_warning;
            self.log_info += outside.log_info;
            self.log_debug += outside.log_debug;
            self.log_verbose += outside.log_verbose;
            self.log_invalid += outside.log_invalid;
        }
    }

    fn add_for_level(level: Option<LogLevel>, ids: &mut IdMap, id: String) {
        if let Some(n) = ids.get_mut(&id) {
            match level {
                Some(LogLevel::Fatal) => {
                    n.log_fatal += 1;
                }
                Some(LogLevel::Error) => {
                    n.log_error += 1;
                }
                Some(LogLevel::Warn) => {
                    n.log_warning += 1;
                }
                Some(LogLevel::Info) => {
                    n.log_info += 1;
                }
                Some(LogLevel::Debug) => {
                    n.log_debug += 1;
                }
                Some(LogLevel::Verbose) => {
                    n.log_verbose += 1;
                }
                Some(LogLevel::Invalid(_)) => {
                    n.log_invalid += 1;
                }
                None => {
                    n.non_log += 1;
                }
            }
        } else {
            ids.insert(id, LevelDistribution::new(level));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{common::*, *};
    use crate::tests::{DLT_MESSAGE, DLT_MESSAGE_WITH_STORAGE_HEADER};

    #[test]
    fn test_empty_statistics() {
        let collector = StatisticInfoCollector::default();
        let stats = collector.collect();

        assert_eq!(0, stats.app_ids.len());
        assert_eq!(0, stats.context_ids.len());
        assert_eq!(0, stats.ecu_ids.len());
        assert!(!stats.contained_non_verbose);
    }

    #[test]
    fn test_collect_statistics() {
        let messages_with_storage = [
            (DLT_MESSAGE, false),
            (DLT_MESSAGE_WITH_STORAGE_HEADER, true),
        ];

        for message_with_storage in &messages_with_storage {
            let bytes = message_with_storage.0;
            let with_storage_header = message_with_storage.1;

            let mut reader = DltMessageReader::new(bytes, with_storage_header);
            let mut collector = StatisticInfoCollector::default();

            collect_statistics(&mut reader, &mut collector).expect("collect statistics");
            let stats = collector.collect();

            assert_eq!(1, stats.app_ids.len());
            assert_eq!(1, stats.context_ids.len());
            assert_eq!(1, stats.ecu_ids.len());
            assert!(!stats.contained_non_verbose);
        }
    }
}
