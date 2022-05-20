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

//! # load filter definitions from dlf files
use crate::filtering::DltFilterConfig;
use quick_xml::{
    events::{BytesStart, Event as XmlEvent},
    Reader as XmlReader,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};
use thiserror::Error;

/// Different kinds of errors.
#[derive(Error, Debug)]
pub enum DlfError {
    /// Error while parsing.
    #[error("{0}")]
    Parse(String),
    /// Error on the XML.
    #[error("{0:?}")]
    Xml(#[from] quick_xml::Error),
}

/// Parser for DLF xml.
pub struct DlfParser;

impl DlfParser {
    /// Returns a filter-config parsend from the given reader or an error.
    ///
    /// Example
    /// ```
    /// # use std::io::BufReader;
    /// # use stringreader::StringReader;
    /// # use dlt_core::dlf::*;
    /// # let string = "";
    /// # let input = BufReader::new(StringReader::new(string));
    /// let reader = DlfReader::from_reader(input)?;
    /// let config = DlfParser::parse(reader)?;
    /// # Ok::<(), DlfError>(())
    /// ```
    pub fn parse<R: Read>(reader: DlfReader<BufReader<R>>) -> Result<DltFilterConfig, DlfError> {
        parser::parse_dlf(reader)
    }
}

mod parser {
    use super::reader::DlfEvent;
    use super::*;

    pub(super) fn parse_dlf<R: Read>(
        mut reader: DlfReader<BufReader<R>>,
    ) -> Result<DltFilterConfig, DlfError> {
        let mut ecu_ids: Vec<(String, u8)> = Vec::new();
        let mut app_ids: Vec<(String, u8)> = Vec::new();
        let mut context_ids: Vec<(String, u8)> = Vec::new();
        let mut app_id_count: i64 = 0;
        let mut context_id_count: i64 = 0;

        loop {
            match reader.read()? {
                DlfEvent::FilterStart => {
                    if let Some(filter) = parse_filter_definition(&mut reader)? {
                        if let Some(ecu_id) = filter.ecu_id {
                            ecu_ids.push((ecu_id, filter.log_level));
                        }
                        if let Some(app_id) = filter.app_id {
                            app_ids.push((app_id, filter.log_level));
                            app_id_count += 1;
                        }
                        if let Some(context_id) = filter.context_id {
                            context_ids.push((context_id, filter.log_level));
                            context_id_count += 1;
                        }
                    }
                }
                DlfEvent::Eof => break,
                _ => {}
            }
        }

        Ok(DltFilterConfig {
            app_ids: if app_ids.is_empty() {
                None
            } else {
                Some(app_ids)
            },
            ecu_ids: if ecu_ids.is_empty() {
                None
            } else {
                Some(ecu_ids)
            },
            context_ids: if context_ids.is_empty() {
                None
            } else {
                Some(context_ids)
            },
            app_id_count,
            context_id_count,
        })
    }

    struct DlfFilterDefinition {
        ecu_id: Option<String>,
        app_id: Option<String>,
        context_id: Option<String>,
        log_level: u8,
    }

    fn parse_filter_definition<R: Read>(
        reader: &mut DlfReader<BufReader<R>>,
    ) -> Result<Option<DlfFilterDefinition>, DlfError> {
        let mut ecu_id: Option<String> = None;
        let mut app_id: Option<String> = None;
        let mut context_id: Option<String> = None;
        let mut log_level_max: Option<u8> = None;
        let mut enable_ecu_id: bool = false;
        let mut enable_app_id: bool = false;
        let mut enable_context_id: bool = false;
        let mut enable_log_level_max: bool = false;
        let mut enable_filter: bool = false;

        loop {
            match reader.read()? {
                DlfEvent::EcuId(id) => {
                    ecu_id = Some(id);
                }
                DlfEvent::AppId(id) => {
                    app_id = Some(id);
                }
                DlfEvent::ContextId(id) => {
                    context_id = Some(id);
                }
                DlfEvent::LogLevelMax(level) => {
                    log_level_max = Some(parse_number(reader, &level)? as u8);
                }
                DlfEvent::EnableEcuId(value) => {
                    enable_ecu_id = parse_number(reader, &value)? == 1usize;
                }
                DlfEvent::EnableAppId(value) => {
                    enable_app_id = parse_number(reader, &value)? == 1usize;
                }
                DlfEvent::EnableContextId(value) => {
                    enable_context_id = parse_number(reader, &value)? == 1usize;
                }
                DlfEvent::EnableLogLevelMax(value) => {
                    enable_log_level_max = parse_number(reader, &value)? == 1usize;
                }
                DlfEvent::EnableFilter(value) => {
                    enable_filter = parse_number(reader, &value)? == 1usize;
                }
                DlfEvent::FilterEnd => {
                    if enable_filter && enable_log_level_max {
                        if let Some(log_level) = log_level_max {
                            return Ok(Some(DlfFilterDefinition {
                                ecu_id: if enable_ecu_id { ecu_id } else { None },
                                app_id: if enable_app_id { app_id } else { None },
                                context_id: if enable_context_id { context_id } else { None },
                                log_level,
                            }));
                        }
                    }
                    return Ok(None);
                }
                DlfEvent::Eof => {
                    return Ok(None);
                }
                _ => {}
            }
        }
    }

    fn parse_number<R: Read>(
        reader: &DlfReader<BufReader<R>>,
        value: &str,
    ) -> Result<usize, DlfError> {
        if let Ok(result) = value.parse::<usize>() {
            return Ok(result);
        }

        Err(DlfError::Parse(format!(
            "Invalid number {} at {}",
            value,
            reader.position(),
        )))
    }
}

/// Reader for DLF xml.
pub struct DlfReader<B: BufRead> {
    #[doc(hidden)]
    reader: XmlReader<B>,
    #[doc(hidden)]
    buffer1: Vec<u8>,
    #[doc(hidden)]
    buffer2: Vec<u8>,
}

impl<B: BufRead> DlfReader<B> {
    /// Returns a new reader for the given input or an error.
    ///
    /// Example
    /// ```
    /// # use std::io::BufReader;
    /// # use stringreader::StringReader;
    /// # use dlt_core::dlf::*;
    /// # let string = "";
    /// let input = BufReader::new(StringReader::new(string));
    /// let reader = DlfReader::from_reader(input)?;
    /// # Ok::<(), DlfError>(())
    /// ```
    pub fn from_reader(input: B) -> Result<Self, DlfError> {
        Ok(DlfReader {
            reader: XmlReader::from_reader(input),
            buffer1: Vec::new(),
            buffer2: Vec::new(),
        })
    }

    #[doc(hidden)]
    fn read(&mut self) -> Result<reader::DlfEvent, DlfError> {
        reader::read_dlf(&mut self.reader, &mut self.buffer1, &mut self.buffer2)
    }

    #[doc(hidden)]
    fn position(&self) -> usize {
        self.reader.buffer_position()
    }
}

impl DlfReader<BufReader<File>> {
    /// Returns a new reader for the given file or an error.
    ///
    /// Example
    /// ```
    /// # use std::path::PathBuf;
    /// # use dlt_core::dlf::*;
    /// # let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/test.dlf");
    /// let file = PathBuf::from(path);
    /// let reader = DlfReader::from_file(file)?;
    /// # Ok::<(), DlfError>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DlfError> {
        Ok(DlfReader {
            reader: XmlReader::from_file(path)?,
            buffer1: Vec::new(),
            buffer2: Vec::new(),
        })
    }
}

mod reader {
    use super::*;

    const B_FILTER: &[u8] = b"filter";
    const B_ECU_ID: &[u8] = b"ecuid";
    const B_APP_ID: &[u8] = b"applicationid";
    const B_CONTEXT_ID: &[u8] = b"contextid";
    const B_LOG_LEVEL_MAX: &[u8] = b"logLevelMax";
    const B_ENABLE_ECU_ID: &[u8] = b"enableecuid";
    const B_ENABLE_APP_ID: &[u8] = b"enableapplicationid";
    const B_ENABLE_CONTEXT_ID: &[u8] = b"enablecontextid";
    const B_ENABLE_LOG_LEVEL_MAX: &[u8] = b"enableLogLevelMax";
    const B_ENABLE_FILTER: &[u8] = b"enablefilter";

    #[derive(Debug)]
    pub(super) enum DlfEvent {
        FilterStart,
        FilterEnd,
        EcuId(String),
        AppId(String),
        ContextId(String),
        LogLevelMax(String),
        EnableEcuId(String),
        EnableAppId(String),
        EnableContextId(String),
        EnableLogLevelMax(String),
        EnableFilter(String),
        Eof,
    }

    pub(super) fn read_dlf<B: BufRead>(
        reader: &mut XmlReader<B>,
        buffer1: &mut Vec<u8>,
        buffer2: &mut Vec<u8>,
    ) -> Result<DlfEvent, DlfError> {
        loop {
            match reader.read_event(buffer1)? {
                XmlEvent::Start(ref event) => match event.local_name() {
                    B_FILTER => {
                        return Ok(DlfEvent::FilterStart);
                    }
                    B_ECU_ID => {
                        return Ok(DlfEvent::EcuId(get_text(reader, buffer2, event)?));
                    }
                    B_APP_ID => {
                        return Ok(DlfEvent::AppId(get_text(reader, buffer2, event)?));
                    }
                    B_CONTEXT_ID => {
                        return Ok(DlfEvent::ContextId(get_text(reader, buffer2, event)?));
                    }
                    B_LOG_LEVEL_MAX => {
                        return Ok(DlfEvent::LogLevelMax(get_text(reader, buffer2, event)?));
                    }
                    B_ENABLE_ECU_ID => {
                        return Ok(DlfEvent::EnableEcuId(get_text(reader, buffer2, event)?));
                    }
                    B_ENABLE_APP_ID => {
                        return Ok(DlfEvent::EnableAppId(get_text(reader, buffer2, event)?));
                    }
                    B_ENABLE_CONTEXT_ID => {
                        return Ok(DlfEvent::EnableContextId(get_text(reader, buffer2, event)?));
                    }
                    B_ENABLE_LOG_LEVEL_MAX => {
                        return Ok(DlfEvent::EnableLogLevelMax(get_text(
                            reader, buffer2, event,
                        )?));
                    }
                    B_ENABLE_FILTER => {
                        return Ok(DlfEvent::EnableFilter(get_text(reader, buffer2, event)?));
                    }
                    _ => {}
                },
                XmlEvent::End(ref event) => {
                    if let B_FILTER = event.local_name() {
                        return Ok(DlfEvent::FilterEnd);
                    }
                }
                XmlEvent::Eof => return Ok(DlfEvent::Eof),
                _ => {}
            }
        }
    }

    fn get_text<B: BufRead>(
        reader: &mut XmlReader<B>,
        buffer: &mut Vec<u8>,
        event: &BytesStart<'_>,
    ) -> Result<String, DlfError> {
        Ok(reader.read_text(event.name(), buffer)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    #[test]
    fn test_parse_dlf_filter() {
        use stringreader::StringReader;

        let xml = r#"
            <dltfilter>
                <filter>
                    <ecuid>E1</ecuid>
                    <applicationid>A1</applicationid>
                    <contextid>C1</contextid>
                    <logLevelMax>7</logLevelMax>
                    <enableecuid>1</enableecuid>
                    <enableapplicationid>1</enableapplicationid>
                    <enablecontextid>1</enablecontextid>
                    <enableLogLevelMax>1</enableLogLevelMax>
                    <enablefilter>1</enablefilter>
                </filter>
            </dltfilter>
        "#;

        let reader = DlfReader::from_reader(BufReader::new(StringReader::new(xml))).unwrap();
        let config = DlfParser::parse(reader).expect("parse failed");

        assert_eq!(
            config,
            DltFilterConfig {
                app_ids: Some(vec![(String::from("A1"), 7)]),
                ecu_ids: Some(vec![(String::from("E1"), 7)]),
                context_ids: Some(vec![(String::from("C1"), 7)]),
                app_id_count: 1,
                context_id_count: 1,
            }
        );
    }

    #[test]
    fn test_parse_dlf_file() {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/test.dlf");

        let reader = DlfReader::from_file(file).unwrap();
        let config = DlfParser::parse(reader).expect("parse failed");

        assert_eq!(
            config,
            DltFilterConfig {
                app_ids: Some(vec![(String::from("A1"), 7)]),
                ecu_ids: Some(vec![(String::from("E1"), 7)]),
                context_ids: Some(vec![(String::from("C1"), 7)]),
                app_id_count: 1,
                context_id_count: 1,
            }
        );
    }
}
