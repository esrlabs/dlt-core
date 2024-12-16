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

//! # rapidly gather statistics info of a dlt source
use crate::{
    dlt::{LogLevel, MessageType},
    parse::{
        dlt_consume_msg, dlt_extended_header, dlt_standard_header,
        skip_till_after_next_storage_header, validated_payload_length, DltParseError,
    },
};
use buf_redux::{policy::MinBuffered, BufReader as ReduxReader};
use nom::bytes::streaming::take;
use rustc_hash::FxHashMap;
use std::{
    fs,
    io::{BufRead, Read},
    path::Path,
};

pub(crate) const BIN_READER_CAPACITY: usize = 10 * 1024 * 1024;
pub(crate) const BIN_MIN_BUFFER_SPACE: usize = 10 * 1024;

/// Parse out the `StatisticRowInfo` for the next DLT message in a byte array
pub fn dlt_statistic_row_info(
    input: &[u8],
    with_storage_header: bool,
) -> Result<(&[u8], StatisticRowInfo), DltParseError> {
    let (after_storage_header, _) = if with_storage_header {
        skip_till_after_next_storage_header(input)?
    } else {
        (input, 0)
    };
    let (after_storage_and_normal_header, header) = dlt_standard_header(after_storage_header)?;

    let payload_length = match validated_payload_length(&header, input.len()) {
        Ok(length) => length,
        Err(_e) => {
            return Ok((
                after_storage_and_normal_header,
                StatisticRowInfo {
                    app_id_context_id: None,
                    ecu_id: header.ecu_id,
                    level: None,
                    verbose: false,
                },
            ));
        }
    };
    if !header.has_extended_header {
        // no app id, skip rest
        let (after_message, _) =
            take::<u16, &[u8], DltParseError>(payload_length)(after_storage_and_normal_header)?;
        return Ok((
            after_message,
            StatisticRowInfo {
                app_id_context_id: None,
                ecu_id: header.ecu_id,
                level: None,
                verbose: false,
            },
        ));
    }

    let (after_headers, extended_header) = dlt_extended_header(after_storage_and_normal_header)?;
    // skip payload
    let (after_message, _) = take::<u16, &[u8], DltParseError>(payload_length)(after_headers)?;
    let level = match extended_header.message_type {
        MessageType::Log(level) => Some(level),
        _ => None,
    };
    Ok((
        after_message,
        StatisticRowInfo {
            app_id_context_id: Some((extended_header.application_id, extended_header.context_id)),
            ecu_id: header.ecu_id,
            level,
            verbose: extended_header.verbose,
        },
    ))
}

/// Shows how many messages per log level where found
#[cfg_attr(
    feature = "serde-support",
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

type IdMap = FxHashMap<String, LevelDistribution>;

/// Includes the `LevelDistribution` for all `app-ids`, `context-ids` and
/// `ecu_ids`
#[cfg_attr(
    feature = "serde-support",
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
            if let Some((_, existed)) = owner.iter_mut().find(|(owner_id, _)| owner_id == income_id)
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

/// Stats about a row in a DLT file
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug)]
pub struct StatisticRowInfo {
    pub app_id_context_id: Option<(String, String)>,
    pub ecu_id: Option<String>,
    pub level: Option<LogLevel>,
    pub verbose: bool,
}

/// Read in a DLT file and collect some statistics about it
pub fn collect_dlt_stats(in_file: &Path) -> Result<StatisticInfo, DltParseError> {
    let f = fs::File::open(in_file)?;

    let mut reader = ReduxReader::with_capacity(BIN_READER_CAPACITY, f)
        .set_policy(MinBuffered(BIN_MIN_BUFFER_SPACE));

    let mut app_ids: IdMap = FxHashMap::default();
    let mut context_ids: IdMap = FxHashMap::default();
    let mut ecu_ids: IdMap = FxHashMap::default();
    let mut contained_non_verbose = false;
    loop {
        match read_one_dlt_message_info(&mut reader, true) {
            Ok(Some((
                consumed,
                StatisticRowInfo {
                    app_id_context_id: Some((app_id, context_id)),
                    ecu_id: ecu,
                    level,
                    verbose,
                },
            ))) => {
                contained_non_verbose = contained_non_verbose || !verbose;
                reader.consume(consumed as usize);
                add_for_level(level, &mut app_ids, app_id);
                add_for_level(level, &mut context_ids, context_id);
                match ecu {
                    Some(id) => add_for_level(level, &mut ecu_ids, id),
                    None => add_for_level(level, &mut ecu_ids, "NONE".to_string()),
                };
            }
            Ok(Some((
                consumed,
                StatisticRowInfo {
                    app_id_context_id: None,
                    ecu_id: ecu,
                    level,
                    verbose,
                },
            ))) => {
                contained_non_verbose = contained_non_verbose || !verbose;
                reader.consume(consumed as usize);
                add_for_level(level, &mut app_ids, "NONE".to_string());
                add_for_level(level, &mut context_ids, "NONE".to_string());
                match ecu {
                    Some(id) => add_for_level(level, &mut ecu_ids, id),
                    None => add_for_level(level, &mut ecu_ids, "NONE".to_string()),
                };
            }
            Ok(None) => {
                break;
            }
            Err(e) => {
                // we couldn't parse the message. try to skip it and find the next.
                debug!("stats...try to skip and continue parsing: {}", e);
                match e {
                    DltParseError::ParsingHickup(reason) => {
                        // we couldn't parse the message. try to skip it and find the next.
                        reader.consume(4); // at least skip the magic DLT pattern
                        debug!(
                            "error parsing 1 dlt message, try to continue parsing: {}",
                            reason
                        );
                    }
                    _ => return Err(e),
                }
            }
        }
    }
    let res = StatisticInfo {
        app_ids: app_ids
            .into_iter()
            .collect::<Vec<(String, LevelDistribution)>>(),
        context_ids: context_ids
            .into_iter()
            .collect::<Vec<(String, LevelDistribution)>>(),
        ecu_ids: ecu_ids
            .into_iter()
            .collect::<Vec<(String, LevelDistribution)>>(),
        contained_non_verbose,
    };
    Ok(res)
}

fn read_one_dlt_message_info<T: Read>(
    reader: &mut ReduxReader<T, MinBuffered>,
    with_storage_header: bool,
) -> Result<Option<(u64, StatisticRowInfo)>, DltParseError> {
    match reader.fill_buf() {
        Ok(content) => {
            if content.is_empty() {
                return Ok(None);
            }
            let available = content.len();
            let r = dlt_statistic_row_info(content, with_storage_header)?;
            let consumed = available - r.0.len();
            Ok(Some((consumed as u64, r.1)))
        }
        Err(e) => Err(DltParseError::ParsingHickup(format!(
            "error while parsing dlt messages: {}",
            e
        ))),
    }
}

fn add_for_level(level: Option<LogLevel>, ids: &mut IdMap, id: String) {
    if let Some(n) = ids.get_mut(&id) {
        match level {
            Some(LogLevel::Fatal) => {
                *n = LevelDistribution {
                    log_fatal: n.log_fatal + 1,
                    ..*n
                }
            }
            Some(LogLevel::Error) => {
                *n = LevelDistribution {
                    log_error: n.log_error + 1,
                    ..*n
                }
            }
            Some(LogLevel::Warn) => {
                *n = LevelDistribution {
                    log_warning: n.log_warning + 1,
                    ..*n
                }
            }
            Some(LogLevel::Info) => {
                *n = LevelDistribution {
                    log_info: n.log_info + 1,
                    ..*n
                }
            }
            Some(LogLevel::Debug) => {
                *n = LevelDistribution {
                    log_debug: n.log_debug + 1,
                    ..*n
                };
            }
            Some(LogLevel::Verbose) => {
                *n = LevelDistribution {
                    log_verbose: n.log_verbose + 1,
                    ..*n
                };
            }
            Some(LogLevel::Invalid(_)) => {
                *n = LevelDistribution {
                    log_invalid: n.log_invalid + 1,
                    ..*n
                };
            }
            None => {
                *n = LevelDistribution {
                    non_log: n.non_log + 1,
                    ..*n
                };
            }
        }
    } else {
        ids.insert(id, LevelDistribution::new(level));
    }
}

/// Count the dlt messages in a file. This assumes that messages are stored with using a `StorageHeader`
pub fn count_dlt_messages(input: &Path) -> Result<u64, DltParseError> {
    if input.exists() {
        let f = fs::File::open(input)?;

        let mut reader = ReduxReader::with_capacity(BIN_READER_CAPACITY, f)
            .set_policy(MinBuffered(BIN_MIN_BUFFER_SPACE));

        let mut msg_cnt: u64 = 0;
        loop {
            match reader.fill_buf() {
                Ok(content) => {
                    if content.is_empty() {
                        break;
                    }
                    if let Ok((_rest, Some(consumed))) = dlt_consume_msg(content) {
                        reader.consume(consumed as usize);
                        msg_cnt += 1;
                    } else {
                        break;
                    }
                }
                Err(e) => {
                    trace!("no more content");
                    return Err(DltParseError::Unrecoverable(format!(
                        "error for filling buffer with dlt messages: {:?}",
                        e
                    )));
                }
            }
        }
        Ok(msg_cnt)
    } else {
        Err(DltParseError::Unrecoverable(format!(
            "Couldn't find dlt file: {:?}",
            input
        )))
    }
}
