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

//! # Support for FIBEX files
//!
//! `fibex` contains support for non-verbose message information
//! that is stored in FIBEX files (Field Bus Exchange Format)
use crate::dlt::{ExtendedHeader, FloatWidth, StringCoding, TypeInfo, TypeInfoKind, TypeLength};
use quick_xml::{
    events::{
        attributes::{AttrError, Attributes},
        BytesStart, Event as XmlEvent,
    },
    Reader as XmlReader,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    fs::File,
    hash::Hash,
    io::{BufRead, BufReader},
    mem,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// FIBEX related error types
#[derive(Error, Debug)]
pub enum Error {
    /// Some structural problem with the fibex file
    #[error("Fibex structure wrong: {0}")]
    FibexStructure(String),
    /// Problems parsing the fibex file
    #[error("Problems parsing: {0}")]
    Parse(String),
    /// Reading the xml failed
    #[error("XML error: {0:?}")]
    Xml(#[from] quick_xml::Error),
    /// Reading an attribute failed
    #[error("Attribute error: {0:?}")]
    Attribute(#[from] AttrError),
    #[error("IO error: {0:?}")]
    Io(#[from] std::io::Error),
}

/// Contains all the paths of fibex files that should be combined into the model
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug)]
pub struct FibexConfig {
    pub fibex_file_paths: Vec<String>,
}

#[derive(Debug, PartialEq, Hash, Clone, Eq)]
pub struct FrameMetadataIdentification {
    pub context_id: String,
    pub app_id: String,
    pub frame_id: String,
}

/// The model represented by the FIBEX data
#[derive(Debug, PartialEq, Clone)]
pub struct FibexMetadata {
    pub frame_map_with_key: HashMap<FrameMetadataIdentification, FrameMetadata>, // TODO: avoid cloning on .get
    pub frame_map: HashMap<FrameId, FrameMetadata>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FrameMetadata {
    pub short_name: String,
    pub pdus: Vec<PduMetadata>, // TODO keep vector of ids and lookup PduMetadata by id if too expensive
    pub application_id: Option<ApplicationId>,
    pub context_id: Option<ContextId>,
    pub message_type: Option<String>,
    pub message_info: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PduMetadata {
    pub description: Option<String>,
    pub signal_types: Vec<TypeInfo>,
}

pub type FrameId = String;
pub type ContextId = String;
pub type ApplicationId = String;

fn type_info_for_signal_ref(
    signal_ref: String,
    signals: &HashMap<String, String>,
    codings: &HashMap<String, String>,
) -> Option<TypeInfo> {
    fn sint8() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Signed(TypeLength::BitLength8),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn uint8() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Unsigned(TypeLength::BitLength8),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn sint16() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Signed(TypeLength::BitLength16),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn uint16() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Unsigned(TypeLength::BitLength16),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn sint32() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Signed(TypeLength::BitLength32),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn uint32() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Unsigned(TypeLength::BitLength32),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn sint64() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Signed(TypeLength::BitLength64),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn uint64() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Unsigned(TypeLength::BitLength64),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn float32() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Float(FloatWidth::Width32),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn float64() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::Float(FloatWidth::Width64),
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn ascii_str() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::StringType,
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    fn utf8_str() -> TypeInfo {
        TypeInfo {
            kind: TypeInfoKind::StringType,
            coding: StringCoding::UTF8,
            has_variable_info: false,
            has_trace_info: false,
        }
    }

    match signal_ref.as_ref() {
        "S_BOOL" => Some(TypeInfo {
            kind: TypeInfoKind::Bool,
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }),
        "S_SINT8" => Some(sint8()),
        "S_UINT8" => Some(uint8()),
        "S_SINT16" => Some(sint16()),
        "S_UINT16" => Some(uint16()),
        "S_SINT32" => Some(sint32()),
        "S_UINT32" => Some(uint32()),
        "S_SINT64" => Some(sint64()),
        "S_UINT64" => Some(uint64()),
        "S_FLOA16" => {
            warn!("16-bit float not supported");
            None
        }
        "S_FLOA32" => Some(float32()),
        "S_FLOA64" => Some(float64()),
        "S_STRG_ASCII" => Some(ascii_str()),
        "S_STRG_UTF8" => Some(utf8_str()),
        "S_RAWD" | "S_RAW" => Some(TypeInfo {
            kind: TypeInfoKind::Raw,
            coding: StringCoding::ASCII,
            has_variable_info: false,
            has_trace_info: false,
        }),
        s => match signals.get(s).and_then(|s| codings.get(s)) {
            Some(base_type) => match base_type.as_ref() {
                "A_UINT8" => Some(uint8()),
                "A_INT8" | "A_SINT8" => Some(sint8()),
                "A_UINT16" => Some(uint16()),
                "A_INT16" | "A_SINT16" => Some(sint16()),
                "A_UINT32" => Some(uint32()),
                "A_INT32" | "A_SINT32" => Some(sint32()),
                "A_UINT64" => Some(uint64()),
                "A_INT64" | "A_SINT64" => Some(sint64()),
                "A_FLOAT32" => Some(float32()),
                "A_FLOAT64" => Some(float64()),
                "A_ASCIISTRING" => Some(ascii_str()),
                "A_UNICODE2STRING" => Some(utf8_str()),
                s => {
                    warn!(
                        "type_info_for_signal_ref: Signal found but base_type not known:{}",
                        s
                    );
                    None
                }
            },
            None => {
                warn!("type_info_for_signal_ref not supported for {}", s);
                None
            }
        },
    }
}

/// Collects all the data found in the FIBEX files and combines it into a complet model
pub fn gather_fibex_data(fibex: FibexConfig) -> Option<FibexMetadata> {
    if fibex.fibex_file_paths.is_empty() {
        None
    } else {
        let paths: Vec<PathBuf> = fibex
            .fibex_file_paths
            .into_iter()
            .map(PathBuf::from)
            .collect();
        match read_fibexes(paths) {
            Ok(res) => Some(res),
            Err(e) => {
                warn!("error reading fibex {}", e);
                None
            }
        }
    }
}

pub(crate) fn read_fibexes(files: Vec<PathBuf>) -> Result<FibexMetadata, Error> {
    let mut frames = vec![];
    let mut frame_map_with_key: HashMap<FrameMetadataIdentification, FrameMetadata> =
        HashMap::new();
    let mut frame_map: HashMap<FrameId, FrameMetadata> = HashMap::new();
    let mut pdu_by_id = HashMap::new();
    let mut signals_map = HashMap::new();
    let mut codings_map = HashMap::new();
    let mut pdus = vec![];
    for f in files {
        debug!("read_fibexe from {:?}", f);
        let mut reader = Reader::from_file(f)?;
        loop {
            match reader.read_event()? {
                Event::PduStart { id } => {
                    pdus.push((id, read_pdu(&mut reader)?));
                }
                Event::FrameStart { id } => {
                    frames.push((id, read_frame(&mut reader)?));
                }
                Event::Eof => break,
                Event::Signal { id, coding_ref } => {
                    trace!("found signal {} (coding_ref={})", id, coding_ref);
                    signals_map.insert(id, coding_ref);
                }
                Event::Coding { id, base_data_type } => {
                    codings_map.insert(id, base_data_type);
                }
                x => {
                    debug!("read_fibex some other event: {:?}", x);
                }
            }
        }
    }
    for (id, (description, signal_refs)) in pdus {
        match pdu_by_id.entry(id) {
            Entry::Occupied(e) => warn!("duplicate PDU ID {} found in fibexes", e.key()),
            Entry::Vacant(v) => {
                v.insert(PduMetadata {
                    description,
                    signal_types: signal_refs
                        .into_iter()
                        .filter_map(|type_ref| {
                            type_info_for_signal_ref(type_ref, &signals_map, &codings_map)
                        })
                        .collect(),
                });
            }
        }
    }
    for (
        id,
        FrameReadData {
            short_name,
            context_id,
            application_id,
            message_type,
            message_info,
            pdu_refs,
        },
    ) in frames
    {
        let frame = FrameMetadata {
            short_name,
            pdus: pdu_refs
                .into_iter()
                .map(|r| {
                    pdu_by_id
                        .get(&r)
                        .cloned()
                        .ok_or_else(|| Error::FibexStructure(format!("pdu {} not found", &r)))
                })
                .collect::<Result<Vec<_>, Error>>()?,
            application_id,
            context_id,
            message_type,
            message_info,
        };
        if let (Some(context_id), Some(application_id)) =
            (frame.context_id.as_ref(), frame.application_id.as_ref())
        {
            let key = FrameMetadataIdentification {
                context_id: context_id.clone(),
                app_id: application_id.clone(),
                frame_id: id.clone(),
            };

            match frame_map_with_key.entry(key) {
                Entry::Occupied(e) => {
                    let key = e.key();
                    warn!(
                        "duplicate Frame context_id={} application_id={} id={}",
                        key.context_id, key.app_id, key.frame_id
                    )
                }
                Entry::Vacant(entry) => {
                    entry.insert(frame.clone());
                }
            }
        } // else error?
        match frame_map.entry(id) {
            Entry::Occupied(e) => warn!("duplicate Frame id={}", e.key()),
            Entry::Vacant(entry) => {
                entry.insert(frame);
            }
        }
    }
    debug!("parsed fibex data OK");
    Ok(FibexMetadata {
        frame_map_with_key,
        frame_map,
    })
}

fn read_pdu(reader: &mut Reader<BufReader<File>>) -> Result<(Option<String>, Vec<String>), Error> {
    let mut signal_refs = vec![];
    loop {
        match reader.read_event()? {
            Event::SignalInstance {
                signal_ref,
                sequence_number,
                ..
            } => {
                signal_refs.push((sequence_number, signal_ref));
            }
            Event::PduEnd { description, .. } => {
                signal_refs.sort_by_key(|s| s.0);
                return Ok((description, signal_refs.into_iter().map(|v| v.1).collect()));
            }
            _ => {}
        }
    }
}

struct FrameReadData {
    short_name: String,
    context_id: Option<ContextId>,
    application_id: Option<ApplicationId>,
    message_type: Option<String>,
    message_info: Option<String>,
    pdu_refs: Vec<String>,
}

fn read_frame(reader: &mut Reader<BufReader<File>>) -> Result<FrameReadData, Error> {
    let mut pdus = vec![];
    let mut frame_context_id = None;
    let mut frame_application_id = None;
    let mut frame_message_type = None;
    let mut frame_message_info = None;
    loop {
        match reader.read_event()? {
            Event::PduInstance {
                pdu_ref,
                sequence_number,
                ..
            } => {
                pdus.push((sequence_number, pdu_ref));
            }
            Event::ManufacturerExtension {
                context_id,
                application_id,
                message_type,
                message_info,
                ..
            } => {
                frame_context_id = context_id;
                frame_application_id = application_id;
                frame_message_type = message_type;
                frame_message_info = message_info;
            }
            Event::FrameEnd { short_name, .. } => {
                pdus.sort_by_key(|p| p.0);
                return Ok(FrameReadData {
                    short_name,
                    context_id: frame_context_id,
                    application_id: frame_application_id,
                    message_type: frame_message_type,
                    message_info: frame_message_info,
                    pdu_refs: pdus.into_iter().map(|p| p.1).collect(),
                });
            }
            _ => {}
        }
    }
}

const B_SHORT_NAME: &[u8] = b"SHORT-NAME";
const B_ID_REF: &[u8] = b"ID-REF";
const B_ID: &[u8] = b"ID";
const B_XSI_TYPE: &[u8] = b"xsi:type";
const B_PDU: &[u8] = b"PDU";
const B_BYTE_LENGTH: &[u8] = b"BYTE-LENGTH";
const B_PDU_TYPE: &[u8] = b"PDU-TYPE";
const B_DESC: &[u8] = b"DESC";
const B_SIGNAL_INSTANCE: &[u8] = b"SIGNAL-INSTANCE";
const B_SEQUENCE_NUMBER: &[u8] = b"SEQUENCE-NUMBER";
const B_SIGNAL_REF: &[u8] = b"SIGNAL-REF";
const B_FRAME: &[u8] = b"FRAME";
const B_FRAME_TYPE: &[u8] = b"FRAME-TYPE";
const B_PDU_INSTANCE: &[u8] = b"PDU-INSTANCE";
const B_PDU_REF: &[u8] = b"PDU-REF";
const B_MANUFACTURER_EXTENSION: &[u8] = b"MANUFACTURER-EXTENSION";
const B_MESSAGE_TYPE: &[u8] = b"MESSAGE_TYPE";
const B_MESSAGE_INFO: &[u8] = b"MESSAGE_INFO";
const B_APPLICATION_ID: &[u8] = b"APPLICATION_ID";
const B_CONTEXT_ID: &[u8] = b"CONTEXT_ID";
const B_CODING: &[u8] = b"CODING";
const B_SIGNAL: &[u8] = b"SIGNAL";
const B_CODING_REF: &[u8] = b"CODING-REF";
const B_BASE_DATA_TYPE: &[u8] = b"BASE-DATA-TYPE";
const B_CODED_TYPE: &[u8] = b"CODED-TYPE";

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum Event {
    PduStart {
        id: String,
    },
    PduEnd {
        short_name: Option<String>,
        description: Option<String>,
        byte_length: usize,
    },
    SignalInstance {
        id: String,
        sequence_number: usize,
        signal_ref: String,
    },
    FrameStart {
        id: String,
    },
    FrameEnd {
        short_name: String,
        byte_length: usize,
    },
    ManufacturerExtension {
        message_type: Option<String>,
        message_info: Option<String>,
        application_id: Option<String>,
        context_id: Option<String>,
    },
    PduInstance {
        id: String,
        pdu_ref: String,
        sequence_number: usize,
    },
    Signal {
        id: String,
        coding_ref: String,
    },
    Coding {
        id: String,
        base_data_type: String,
    },
    Eof,
}
pub(crate) struct XmlReaderWithContext<B: BufRead> {
    xml_reader: XmlReader<B>,
    file_path: PathBuf,
}
impl<B: BufRead> XmlReaderWithContext<B> {
    pub fn buffer_position(&self) -> usize {
        self.xml_reader.buffer_position()
    }
    pub fn read_event<'a>(&mut self, buf: &'a mut Vec<u8>) -> Result<XmlEvent<'a>, Error> {
        Ok(self.xml_reader.read_event_into(buf)?)
    }
    pub fn read_text(&mut self, buf: &mut Vec<u8>) -> Result<String, Error> {
        match self.xml_reader.read_event_into(buf)? {
            XmlEvent::Text(ref e) => match e.unescape() {
                Ok(text) => Ok(text.into_owned()),
                Err(error) => Err(Error::Xml(error)),
            },
            x => Err(Error::Parse(format!(
                "read_text (unexpected: {:?}) at {}",
                x,
                self.buffer_position()
            ))),
        }
    }
    // Note: Use this only on fatal errors due performance.
    pub fn line_and_column(&self) -> Result<(usize, usize), Error> {
        let s = std::fs::read_to_string(&self.file_path)?;
        let mut line = 1;
        let mut column = 0;
        for c in s.chars().take(self.buffer_position()) {
            if c == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }
        Ok((line, column))
    }
    pub fn read_usize(&mut self) -> Result<usize, Error> {
        self.read_text_buf()?.parse::<usize>().map_err(|e| {
            let (line, column) = self.line_and_column().unwrap_or((0, 0));
            Error::Parse(format!("can't parse usize at {}:{}: {}", line, column, e))
        })
    }
    pub fn read_text_buf(&mut self) -> Result<String, Error> {
        self.read_text(&mut Vec::new())
    }
    pub fn id_ref_attr(&self, e: &BytesStart<'_>, tag: &[u8]) -> Result<String, Error> {
        self.attr_opt(e.attributes(), B_ID_REF)?
            .ok_or_else(|| missing_attr_err(B_ID_REF, tag, self.line_and_column()))
    }
    #[allow(dead_code)]
    pub fn read_bool(&mut self) -> Result<bool, Error> {
        match self.read_text_buf()?.as_ref() {
            "true" => Ok(true),
            "false" => Ok(false),
            v => {
                let (line, column) = self.line_and_column()?;
                Err(Error::Parse(format!(
                    "expected bool value, got {} at {}:{}",
                    v, line, column
                )))
            }
        }
    }
    pub fn attr_opt(&self, attrs: Attributes<'_>, name: &[u8]) -> Result<Option<String>, Error> {
        for attr in attrs {
            let attr = attr?;
            let attr_key = attr.key.as_ref();
            let matches = if attr_key == name {
                true
            } else {
                let name_len = name.len();
                let key_len = attr_key.len();
                if key_len > name_len {
                    // support for namespaced attributes
                    attr_key[key_len - name_len - 1] == b':'
                        && &attr_key[key_len - name_len..] == name
                } else {
                    false
                }
            };
            if matches {
                return Ok(Some(attr.unescape_value()?.into_owned()));
            }
        }
        Ok(None)
    }

    #[allow(dead_code)]
    pub fn xsi_type_attr(&self, e: &BytesStart<'_>, tag: &[u8]) -> Result<String, Error> {
        self.attr(e, B_XSI_TYPE, tag)
    }
    pub fn attr(&self, e: &BytesStart<'_>, name: &[u8], tag: &[u8]) -> Result<String, Error> {
        self.attr_opt(e.attributes(), name)?
            .ok_or_else(|| missing_attr_err(name, tag, self.line_and_column()))
    }
    pub fn id_attr(&self, e: &BytesStart<'_>, tag: &[u8]) -> Result<String, Error> {
        self.attr(e, B_ID, tag)
    }
}

pub(crate) struct Reader<B: BufRead> {
    xml_reader: XmlReaderWithContext<B>,
    buf: Vec<u8>,
    buf2: Vec<u8>,
    short_name: Option<String>,
    description: Option<String>,
    byte_length: Option<usize>,
    #[allow(dead_code)]
    r#type: Option<String>,
    id: Option<String>,
    sequence_number: Option<usize>,
    r#ref: Option<String>,
    application_id: Option<String>,
    context_id: Option<String>,
    message_type: Option<String>,
    message_info: Option<String>,
    base_data_type: Option<String>,
}

impl Reader<BufReader<File>> {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Reader {
            xml_reader: XmlReaderWithContext {
                file_path: path.as_ref().to_owned(),
                xml_reader: XmlReader::from_file(path)?,
            },
            buf: vec![],
            buf2: vec![],
            short_name: None,
            description: None,
            byte_length: None,
            r#type: None,
            id: None,
            sequence_number: None,
            r#ref: None,
            application_id: None,
            context_id: None,
            message_type: None,
            message_info: None,
            base_data_type: None,
        })
    }
}

impl<B: BufRead> Reader<B> {
    #[allow(clippy::cognitive_complexity)]
    pub fn read_event(&mut self) -> Result<Event, Error> {
        loop {
            match self.xml_reader.read_event(&mut self.buf)? {
                XmlEvent::Start(ref e) => match e.local_name().as_ref() {
                    B_PDU => {
                        self.short_name = None;
                        self.byte_length = None;
                        self.r#type = None;
                        self.description = None;
                        return Ok(Event::PduStart {
                            id: self.xml_reader.id_attr(e, B_PDU)?,
                        });
                    }
                    B_SHORT_NAME => {
                        self.short_name = Some(self.xml_reader.read_text(&mut self.buf2)?);
                        self.buf2.clear();
                    }
                    B_BYTE_LENGTH => {
                        self.byte_length = Some(self.xml_reader.read_usize()?);
                    }
                    B_SIGNAL_INSTANCE => {
                        self.id = Some(self.xml_reader.id_attr(e, B_SIGNAL_INSTANCE)?);
                        self.r#ref = None;
                        self.sequence_number = None;
                    }
                    B_SEQUENCE_NUMBER => self.sequence_number = Some(self.xml_reader.read_usize()?),
                    B_SIGNAL_REF => {
                        self.r#ref = Some(self.xml_reader.id_ref_attr(e, B_SIGNAL_REF)?)
                    }
                    B_PDU_TYPE => {
                        self.r#type = Some(self.xml_reader.read_text(&mut self.buf2)?);
                        self.buf2.clear();
                    }
                    B_FRAME_TYPE => {
                        self.r#type = Some(self.xml_reader.read_text(&mut self.buf2)?);
                        self.buf.clear();
                    }
                    B_FRAME => {
                        self.short_name = None;
                        self.byte_length = None;
                        self.r#type = None;
                        return Ok(Event::FrameStart {
                            id: self.xml_reader.id_attr(e, B_PDU)?,
                        });
                    }
                    B_PDU_INSTANCE => {
                        self.id = Some(self.xml_reader.id_attr(e, B_PDU_INSTANCE)?);
                        self.r#ref = None;
                        self.sequence_number = None;
                    }
                    B_PDU_REF => {
                        self.r#ref = Some(self.xml_reader.id_ref_attr(e, B_PDU_REF)?);
                    }
                    B_MANUFACTURER_EXTENSION => {
                        self.application_id = None;
                        self.context_id = None;
                        self.message_info = None;
                        self.message_type = None;
                    }
                    B_APPLICATION_ID => {
                        self.application_id = Some(self.xml_reader.read_text(&mut self.buf2)?);
                    }
                    B_CONTEXT_ID => {
                        self.context_id = Some(self.xml_reader.read_text(&mut self.buf2)?);
                    }
                    B_MESSAGE_INFO => {
                        self.message_info = Some(self.xml_reader.read_text(&mut self.buf2)?);
                    }
                    B_MESSAGE_TYPE => {
                        self.message_type = Some(self.xml_reader.read_text(&mut self.buf2)?);
                    }
                    B_DESC => {
                        self.description = self.xml_reader.read_text(&mut self.buf2).ok();
                    }
                    B_CODING => {
                        self.id = Some(self.xml_reader.id_attr(e, B_CODING)?);
                        self.base_data_type = None;
                    }
                    B_SIGNAL => {
                        self.id = Some(self.xml_reader.id_attr(e, B_SIGNAL)?);
                        self.r#ref = None;
                    }
                    B_CODED_TYPE => {
                        self.base_data_type =
                            self.xml_reader.attr(e, B_BASE_DATA_TYPE, B_CODED_TYPE).ok();
                    }
                    _x => {
                        // trace!("read_event (unknown: {:?})", _x);
                    }
                },
                XmlEvent::Empty(ref e) => match e.local_name().as_ref() {
                    B_SIGNAL_REF => {
                        self.r#ref = Some(self.xml_reader.id_ref_attr(e, B_SIGNAL_REF)?)
                    }
                    B_PDU_REF => self.r#ref = Some(self.xml_reader.id_ref_attr(e, B_PDU_REF)?),
                    B_CODING_REF => {
                        self.r#ref = Some(self.xml_reader.id_ref_attr(e, B_SIGNAL_REF)?);
                    }
                    B_CODED_TYPE => {
                        self.base_data_type =
                            self.xml_reader.attr(e, B_BASE_DATA_TYPE, B_CODED_TYPE).ok();
                    }
                    x => {
                        trace!("XmlEvent::Empty (unknown: {:?})", x);
                    }
                },
                XmlEvent::End(ref e) => match e.local_name().as_ref() {
                    B_PDU => {
                        return Ok(Event::PduEnd {
                            short_name: mem::take(&mut self.short_name),
                            description: mem::take(&mut self.description),
                            byte_length: mem::take(&mut self.byte_length).ok_or_else(|| {
                                missing_tag_err(
                                    B_BYTE_LENGTH,
                                    B_PDU,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                        });
                    }
                    B_SIGNAL_INSTANCE => {
                        return Ok(Event::SignalInstance {
                            id: mem::take(&mut self.id).ok_or_else(|| {
                                missing_attr_err(
                                    B_ID,
                                    B_SIGNAL_INSTANCE,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                            sequence_number: mem::take(&mut self.sequence_number).ok_or_else(
                                || {
                                    missing_tag_err(
                                        B_SEQUENCE_NUMBER,
                                        B_SIGNAL_INSTANCE,
                                        self.xml_reader.line_and_column(),
                                    )
                                },
                            )?,
                            signal_ref: mem::take(&mut self.r#ref).ok_or_else(|| {
                                missing_tag_err(
                                    B_SIGNAL_REF,
                                    B_SIGNAL_INSTANCE,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                        });
                    }
                    B_FRAME => {
                        return Ok(Event::FrameEnd {
                            short_name: mem::take(&mut self.short_name).ok_or_else(|| {
                                missing_tag_err(
                                    B_SHORT_NAME,
                                    B_FRAME,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                            byte_length: mem::take(&mut self.byte_length).ok_or_else(|| {
                                missing_tag_err(
                                    B_BYTE_LENGTH,
                                    B_FRAME,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                        });
                    }
                    B_PDU_INSTANCE => {
                        return Ok(Event::PduInstance {
                            id: mem::take(&mut self.id).ok_or_else(|| {
                                missing_attr_err(
                                    B_ID,
                                    B_PDU_INSTANCE,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                            sequence_number: mem::take(&mut self.sequence_number).ok_or_else(
                                || {
                                    missing_tag_err(
                                        B_SEQUENCE_NUMBER,
                                        B_PDU_INSTANCE,
                                        self.xml_reader.line_and_column(),
                                    )
                                },
                            )?,
                            pdu_ref: mem::take(&mut self.r#ref).ok_or_else(|| {
                                missing_tag_err(
                                    B_PDU_REF,
                                    B_PDU_INSTANCE,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                        });
                    }
                    B_MANUFACTURER_EXTENSION => {
                        return Ok(Event::ManufacturerExtension {
                            application_id: mem::take(&mut self.application_id),
                            context_id: mem::take(&mut self.context_id),
                            message_type: mem::take(&mut self.message_type),
                            message_info: mem::take(&mut self.message_info),
                        });
                    }
                    B_SIGNAL => {
                        return Ok(Event::Signal {
                            id: mem::take(&mut self.id).ok_or_else(|| {
                                missing_attr_err(B_ID, B_SIGNAL, self.xml_reader.line_and_column())
                            })?,
                            coding_ref: mem::take(&mut self.r#ref).ok_or_else(|| {
                                missing_tag_err(
                                    B_CODING_REF,
                                    B_SIGNAL,
                                    self.xml_reader.line_and_column(),
                                )
                            })?,
                        });
                    }
                    B_CODING => {
                        return Ok(Event::Coding {
                            id: mem::take(&mut self.id).ok_or_else(|| {
                                missing_attr_err(B_ID, B_CODING, self.xml_reader.line_and_column())
                            })?,
                            base_data_type: mem::take(&mut self.base_data_type).ok_or_else(
                                || {
                                    missing_attr_err(
                                        B_BASE_DATA_TYPE,
                                        B_CODED_TYPE,
                                        self.xml_reader.line_and_column(),
                                    )
                                },
                            )?,
                        });
                    }
                    _x => {}
                },
                XmlEvent::Eof => return Ok(Event::Eof),
                _x => {
                    // trace!("XmlEvent::* unknown ({:?})", _x);
                }
            }
            self.buf.clear();
            self.buf2.clear();
        }
    }
}

fn missing_tag_err(
    tag: &[u8],
    enclosing_tag: &[u8],
    line_column: Result<(usize, usize), Error>,
) -> Error {
    Error::FibexStructure(format!(
        "required {} tag is missing for {} at {:?}",
        String::from_utf8_lossy(tag),
        String::from_utf8_lossy(enclosing_tag),
        line_column.unwrap_or((0, 0))
    ))
}

fn missing_attr_err(attr: &[u8], tag: &[u8], line_column: Result<(usize, usize), Error>) -> Error {
    Error::FibexStructure(format!(
        "required {} attribute is missing for {} at {:?}",
        String::from_utf8_lossy(attr),
        String::from_utf8_lossy(tag),
        line_column.unwrap_or((0, 0))
    ))
}

/// lookup `FrameMetadata` in the fibex model using the information from the
/// extended header. If no extended header is present, try with just the frame-id.
pub fn extract_metadata<'a>(
    fibex_metadata: &'a FibexMetadata,
    id: u32,
    extended_header: Option<&ExtendedHeader>,
) -> Option<&'a FrameMetadata> {
    let id_text = format!("ID_{}", id);
    match extended_header {
        Some(extended_header) => {
            let frame_identifier = FrameMetadataIdentification {
                context_id: extended_header.context_id.clone(),
                app_id: extended_header.application_id.clone(),
                frame_id: id_text,
            };
            fibex_metadata.frame_map_with_key.get(&frame_identifier)
        }
        None => fibex_metadata.frame_map.get(&id_text),
    }
}
