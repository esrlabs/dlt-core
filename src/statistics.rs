use crate::{
    dlt::{LogLevel, MessageType},
    parse::{
        dlt_extended_header, dlt_standard_header, skip_till_after_next_storage_header,
        validated_payload_length, DltParseError,
    },
};
use buf_redux::{policy::MinBuffered, BufReader as ReduxReader};
use nom::bytes::streaming::take;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::io::{BufRead, Read};

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

#[derive(Serialize, Debug, Default)]
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
}

pub type IdMap = FxHashMap<String, LevelDistribution>;

#[derive(Serialize, Debug)]
pub struct StatisticInfo {
    pub app_ids: Vec<(String, LevelDistribution)>,
    pub context_ids: Vec<(String, LevelDistribution)>,
    pub ecu_ids: Vec<(String, LevelDistribution)>,
    pub contained_non_verbose: bool,
}

#[derive(Serialize, Debug)]
pub struct StatisticRowInfo {
    pub app_id_context_id: Option<(String, String)>,
    pub ecu_id: Option<String>,
    pub level: Option<LogLevel>,
    pub verbose: bool,
}

pub fn read_one_dlt_message_info<T: Read>(
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
