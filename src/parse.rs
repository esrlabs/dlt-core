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

//! # dlt parsing support
use crate::{
    dlt::{
        calculate_all_headers_length, float_width_to_type_length, ApplicationTraceType, Argument,
        ControlType, DltTimeStamp, Endianness, ExtendedHeader, FixedPoint, FixedPointValue,
        FloatWidth, LogLevel, Message, MessageType, NetworkTraceType, PayloadContent,
        StandardHeader, StorageHeader, TypeInfo, TypeInfoKind, TypeLength, Value, BIG_ENDIAN_FLAG,
        STORAGE_HEADER_LENGTH, VERBOSE_FLAG, WITH_ECU_ID_FLAG, WITH_EXTENDED_HEADER_FLAG,
        WITH_SESSION_ID_FLAG, WITH_TIMESTAMP_FLAG,
    },
    filtering,
};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use nom::{
    bytes::streaming::{tag, take, take_while_m_n},
    combinator::map,
    error::{ErrorKind, ParseError},
    multi::count,
    number::streaming::{
        be_f32, be_f64, be_i128, be_i16, be_i32, be_i64, be_i8, be_u128, be_u16, be_u32, be_u64,
        be_u8, le_f32, le_f64, le_i128, le_i16, le_i32, le_i64, le_u128, le_u16, le_u32, le_u64,
    },
    sequence::tuple,
    Err::Error,
    IResult,
};
use std::convert::TryFrom;
use thiserror::Error;

/// DLT pattern at the start of a storage header
pub const DLT_PATTERN: &[u8] = &[0x44, 0x4C, 0x54, 0x01];

pub(crate) fn parse_ecu_id(input: &[u8]) -> IResult<&[u8], &str, DltParseError> {
    dlt_zero_terminated_string_intern(input, 4)
}

impl ParseError<&[u8]> for DltParseError {
    fn from_error_kind(input: &[u8], kind: ErrorKind) -> Self {
        DltParseError::ParsingHickup(format!(
            "Nom error: {:?} ({} bytes left)",
            kind,
            input.len()
        ))
    }

    fn append(_: &[u8], _: ErrorKind, other: Self) -> Self {
        other
    }
}

/// Errors that can happen during parsing
#[derive(Error, Debug, PartialEq)]
pub enum DltParseError {
    #[error("parsing stopped, cannot continue: {0}")]
    Unrecoverable(String),
    #[error("parsing error, try to continue: {0}")]
    ParsingHickup(String),
    #[error("parsing could not complete: {:?}", needed)]
    IncompleteParse {
        needed: Option<std::num::NonZeroUsize>,
    },
}

impl From<std::io::Error> for DltParseError {
    fn from(err: std::io::Error) -> DltParseError {
        DltParseError::Unrecoverable(format!("{}", err))
    }
}

impl From<nom::Err<DltParseError>> for DltParseError {
    fn from(ne: nom::Err<DltParseError>) -> Self {
        nom_to_dlt_parse_error(ne, "")
    }
}

impl From<nom::Err<(&[u8], nom::error::ErrorKind)>> for DltParseError {
    fn from(err: nom::Err<(&[u8], nom::error::ErrorKind)>) -> DltParseError {
        match err {
            nom::Err::Incomplete(n) => {
                let needed = match n {
                    nom::Needed::Size(s) => Some(s),
                    nom::Needed::Unknown => None,
                };
                DltParseError::IncompleteParse { needed }
            }
            nom::Err::Error((input, kind)) => DltParseError::ParsingHickup(format!(
                "{:?} ({} bytes left in input)",
                kind,
                input.len()
            )),
            nom::Err::Failure((input, kind)) => DltParseError::Unrecoverable(format!(
                "{:?} ({} bytes left in input)",
                kind,
                input.len()
            )),
        }
    }
}

/// Skips ahead in input array up to the next storage header
///
/// Returns the number of dropped bytes along with the remaining slice.
/// If no next storage header can be found, `None` is returned.
///
/// Note: will not skip anything if the input already begins with a storage header.
///
/// # Arguments
///
/// * `input` - A slice of bytes that contain dlt messages including storage headers
///
pub fn forward_to_next_storage_header(input: &[u8]) -> Option<(u64, &[u8])> {
    use memchr::memmem;
    let finder = memmem::Finder::new(DLT_PATTERN);
    finder.find(input).map(|to_drop| {
        if to_drop > 0 {
            trace!("Need to drop {} bytes to get to next message", to_drop);
        }
        (to_drop as u64, &input[to_drop..])
    })
}

/// parse the next DLT storage header
/// this function will move along the content until it finds a storage header
/// the amount of bytes we had to move forwared is the second part of the return value
pub(crate) fn dlt_storage_header(
    input: &[u8],
) -> IResult<&[u8], Option<(StorageHeader, u64)>, DltParseError> {
    if input.len() < STORAGE_HEADER_LENGTH as usize {
        return Err(nom::Err::Incomplete(nom::Needed::Unknown));
    }
    match forward_to_next_storage_header(input) {
        Some((consumed, rest)) => {
            let (input, (_, _, seconds, microseconds)) =
                tuple((tag("DLT"), tag(&[0x01]), le_u32, le_u32))(rest)?;

            let (after_string, ecu_id) = dlt_zero_terminated_string_intern(input, 4)?;
            Ok((
                after_string,
                Some((
                    StorageHeader {
                        timestamp: DltTimeStamp {
                            seconds,
                            microseconds,
                        },
                        ecu_id: ecu_id.to_string(),
                    },
                    consumed,
                )),
            ))
        }
        None => {
            warn!("Did not find another storage header in file");
            Ok((&[], None))
        }
    }
}

fn maybe_parse_ecu_id(a: bool) -> impl Fn(&[u8]) -> IResult<&[u8], Option<&str>, DltParseError> {
    fn parse_ecu_id_to_option(input: &[u8]) -> IResult<&[u8], Option<&str>, DltParseError> {
        let (rest, ecu_id) = parse_ecu_id(input)?;
        Ok((rest, Some(ecu_id)))
    }
    #[allow(clippy::unnecessary_wraps)]
    fn parse_nothing_str(input: &[u8]) -> IResult<&[u8], Option<&str>, DltParseError> {
        Ok((input, None))
    }
    if a {
        parse_ecu_id_to_option
    } else {
        parse_nothing_str
    }
}

fn maybe_parse_u32(a: bool) -> impl Fn(&[u8]) -> IResult<&[u8], Option<u32>, DltParseError> {
    fn parse_u32_to_option(input: &[u8]) -> IResult<&[u8], Option<u32>, DltParseError> {
        map(be_u32, Some)(input)
    }
    #[allow(clippy::unnecessary_wraps)]
    fn parse_nothing_u32(input: &[u8]) -> IResult<&[u8], Option<u32>, DltParseError> {
        Ok((input, None))
    }
    if a {
        parse_u32_to_option
    } else {
        parse_nothing_u32
    }
}

fn add_context(ne: nom::Err<DltParseError>, desc: String) -> nom::Err<DltParseError> {
    match ne {
        nom::Err::Incomplete(n) => nom::Err::Incomplete(n),
        nom::Err::Error(e) => {
            nom::Err::Error(DltParseError::ParsingHickup(format!("{}: {}", desc, e)))
        }
        nom::Err::Failure(e) => {
            nom::Err::Error(DltParseError::Unrecoverable(format!("{}: {}", desc, e)))
        }
    }
}

fn nom_to_dlt_parse_error(ne: nom::Err<DltParseError>, desc: &str) -> DltParseError {
    match ne {
        nom::Err::Incomplete(nom::Needed::Size(needed)) => DltParseError::IncompleteParse {
            needed: Some(needed),
        },
        nom::Err::Incomplete(nom::Needed::Unknown) => {
            DltParseError::IncompleteParse { needed: None }
        }
        nom::Err::Error(e) => DltParseError::ParsingHickup(format!("{}: {}", desc, e)),
        nom::Err::Failure(e) => DltParseError::Unrecoverable(format!("{}: {}", desc, e)),
    }
}

/// The standard header is part of every DLT message
/// all big endian format [PRS_Dlt_00091]
pub(crate) fn dlt_standard_header(input: &[u8]) -> IResult<&[u8], StandardHeader, DltParseError> {
    let (input, header_type_byte) = be_u8(input)?;
    let has_ecu_id = (header_type_byte & WITH_ECU_ID_FLAG) != 0;
    let has_session_id = (header_type_byte & WITH_SESSION_ID_FLAG) != 0;
    let has_timestamp = (header_type_byte & WITH_TIMESTAMP_FLAG) != 0;
    let (input, (message_counter, overall_length, ecu_id, session_id, timestamp)) = tuple((
        be_u8,
        be_u16,
        maybe_parse_ecu_id(has_ecu_id),
        maybe_parse_u32(has_session_id),
        maybe_parse_u32(has_timestamp),
    ))(input)?;

    let has_extended_header = (header_type_byte & WITH_EXTENDED_HEADER_FLAG) != 0;
    let all_headers_length = calculate_all_headers_length(header_type_byte);
    if all_headers_length > overall_length {
        return Err(Error(DltParseError::ParsingHickup(
            "Header indecates wrong message length".to_string(),
        )));
    }
    let payload_length = overall_length - all_headers_length;

    Ok((
        input,
        StandardHeader::new(
            (header_type_byte >> 5) & 0b111,
            if (header_type_byte & BIG_ENDIAN_FLAG) != 0 {
                Endianness::Big
            } else {
                Endianness::Little
            },
            message_counter,
            has_extended_header,
            payload_length,
            ecu_id.map(|r| r.to_string()),
            session_id,
            timestamp,
        ),
    ))
}

pub(crate) fn dlt_extended_header(input: &[u8]) -> IResult<&[u8], ExtendedHeader, DltParseError> {
    let (i, (message_info, argument_count, app_id, context_id)) =
        tuple((be_u8, be_u8, parse_ecu_id, parse_ecu_id))(input)?;

    let verbose = (message_info & VERBOSE_FLAG) != 0;
    match MessageType::try_from(message_info) {
        Ok(message_type) => {
            match message_type {
                MessageType::Unknown(n) => {
                    warn!("unknown message type {:?}", n);
                }
                MessageType::Log(LogLevel::Invalid(n)) => {
                    warn!("unknown log level {}", n);
                }
                MessageType::Control(ControlType::Unknown(n)) => {
                    warn!("unknown control type {}", n);
                }
                MessageType::ApplicationTrace(ApplicationTraceType::Invalid(n)) => {
                    warn!("invalid application-trace type {}", n);
                }
                MessageType::NetworkTrace(NetworkTraceType::Invalid) => {
                    warn!("invalid application-trace type 0");
                }
                _ => (),
            };
            Ok((
                i,
                ExtendedHeader {
                    verbose,
                    argument_count,
                    message_type,
                    application_id: app_id.to_string(),
                    context_id: context_id.to_string(),
                },
            ))
        }
        Err(e) => {
            let msg = format!("invalid message type: {}", e);
            Err(Error(DltParseError::ParsingHickup(msg)))
        }
    }
}

#[inline]
fn is_not_null(chr: u8) -> bool {
    chr != 0x0
}

/// Extracts the string in a byte sequence up to the `\0` termination character
///
/// In various places within the DLT message, there can be strings that are
/// terminated with a `\0`.
pub fn dlt_zero_terminated_string(s: &[u8], size: usize) -> Result<(&[u8], &str), DltParseError> {
    dlt_zero_terminated_string_intern(s, size).map_err(DltParseError::from)
}

/// Extracts the string in a byte sequence up to the `\0` termination character
///
/// In various places within the DLT message, there can be strings that are
/// terminated with a `\0`.
fn dlt_zero_terminated_string_intern(s: &[u8], size: usize) -> IResult<&[u8], &str, DltParseError> {
    let (rest_with_null, content_without_null) = take_while_m_n(0, size, is_not_null)(s)?;
    let res_str = match nom::lib::std::str::from_utf8(content_without_null) {
        Ok(content) => content,
        Err(e) => {
            let (valid, _) = content_without_null.split_at(e.valid_up_to());
            unsafe { nom::lib::std::str::from_utf8_unchecked(valid) }
        }
    };
    let missing = size - content_without_null.len();
    let (rest, _) = take(missing)(rest_with_null)?;
    Ok((rest, res_str))
}

fn dlt_variable_name<T: NomByteOrder>(input: &[u8]) -> IResult<&[u8], String, DltParseError> {
    let (i, size) = T::parse_u16(input)?;
    let (i2, name) = dlt_zero_terminated_string_intern(i, size as usize)?;
    Ok((i2, name.to_string()))
}

pub(crate) trait NomByteOrder: Clone + Copy + Eq + Ord + PartialEq + PartialOrd {
    fn parse_u16(i: &[u8]) -> IResult<&[u8], u16, DltParseError>;
    fn parse_i16(i: &[u8]) -> IResult<&[u8], i16, DltParseError>;
    fn parse_u32(i: &[u8]) -> IResult<&[u8], u32, DltParseError>;
    fn parse_i32(i: &[u8]) -> IResult<&[u8], i32, DltParseError>;
    fn parse_f32(i: &[u8]) -> IResult<&[u8], f32, DltParseError>;
    fn parse_u64(i: &[u8]) -> IResult<&[u8], u64, DltParseError>;
    fn parse_i64(i: &[u8]) -> IResult<&[u8], i64, DltParseError>;
    fn parse_f64(i: &[u8]) -> IResult<&[u8], f64, DltParseError>;
    fn parse_u128(i: &[u8]) -> IResult<&[u8], u128, DltParseError>;
    fn parse_i128(i: &[u8]) -> IResult<&[u8], i128, DltParseError>;
    fn to_string(input: &[u8], width: usize) -> String;
}

macro_rules! impl_nombyteorder{($($fn_trait:ident $fn_nom:ident $tp:ident ,)*) => {
    $(
        #[inline]
        fn $fn_trait(i: &[u8]) -> IResult<&[u8], $tp, DltParseError> {
            $fn_nom(i)
        }
    )*
}}

impl NomByteOrder for BigEndian {
    impl_nombyteorder!(
        parse_u16 be_u16 u16,
        parse_i16 be_i16 i16,
        parse_u32 be_u32 u32,
        parse_i32 be_i32 i32,
        parse_f32 be_f32 f32,
        parse_u64 be_u64 u64,
        parse_i64 be_i64 i64,
        parse_f64 be_f64 f64,
        parse_u128 be_u128 u128,
        parse_i128 be_i128 i128,
    );
    fn to_string(input: &[u8], width: usize) -> String {
        let v = input
            .iter()
            .take(width)
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join("");
        format!("0x{}", v)
    }
}

#[allow(clippy::type_complexity)]
fn dlt_variable_name_and_unit<T: NomByteOrder>(
    type_info: &TypeInfo,
) -> fn(&[u8]) -> IResult<&[u8], (Option<String>, Option<String>), DltParseError> {
    if type_info.has_variable_info {
        |input: &[u8]| -> IResult<&[u8], (Option<String>, Option<String>), DltParseError> {
            let (i2, name_size_unit_size) = tuple((T::parse_u16, T::parse_u16))(input)?;
            dbg_parsed("namesize, unitsize", input, i2, &name_size_unit_size);
            let (i3, name) = dlt_zero_terminated_string_intern(i2, name_size_unit_size.0 as usize)?;
            dbg_parsed("name", i2, i3, &name);
            let (rest, unit) =
                dlt_zero_terminated_string_intern(i3, name_size_unit_size.1 as usize)?;
            dbg_parsed("unit", i3, rest, &unit);
            Ok((rest, (Some(name.to_string()), Some(unit.to_string()))))
        }
    } else {
        |input| Ok((input, (None, None)))
    }
}

impl NomByteOrder for LittleEndian {
    impl_nombyteorder!(
        parse_u16 le_u16 u16,
        parse_i16 le_i16 i16,
        parse_u32 le_u32 u32,
        parse_i32 le_i32 i32,
        parse_f32 le_f32 f32,
        parse_u64 le_u64 u64,
        parse_i64 le_i64 i64,
        parse_f64 le_f64 f64,
        parse_u128 le_u128 u128,
        parse_i128 le_i128 i128,
    );
    fn to_string(input: &[u8], width: usize) -> String {
        let v = input
            .iter()
            .take(width)
            .rev()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join("");
        format!("0x{}", v)
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn dlt_uint<T: NomByteOrder>(
    width: TypeLength,
) -> fn(&[u8]) -> IResult<&[u8], Value, DltParseError> {
    match width {
        TypeLength::BitLength8 => |i| map(be_u8, Value::U8)(i),
        TypeLength::BitLength16 => |i| map(T::parse_u16, Value::U16)(i),
        TypeLength::BitLength32 => |i| map(T::parse_u32, Value::U32)(i),
        TypeLength::BitLength64 => |i| map(T::parse_u64, Value::U64)(i),
        TypeLength::BitLength128 => |i| map(T::parse_u128, Value::U128)(i),
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn dlt_sint<T: NomByteOrder>(
    width: TypeLength,
) -> fn(&[u8]) -> IResult<&[u8], Value, DltParseError> {
    match width {
        TypeLength::BitLength8 => |i| map(be_i8, Value::I8)(i),
        TypeLength::BitLength16 => |i| map(T::parse_i16, Value::I16)(i),
        TypeLength::BitLength32 => |i| map(T::parse_i32, Value::I32)(i),
        TypeLength::BitLength64 => |i| map(T::parse_i64, Value::I64)(i),
        TypeLength::BitLength128 => |i| map(T::parse_i128, Value::I128)(i),
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn dlt_fint<T: NomByteOrder>(
    width: FloatWidth,
) -> fn(&[u8]) -> IResult<&[u8], Value, DltParseError> {
    match width {
        FloatWidth::Width32 => |i| map(T::parse_f32, Value::F32)(i),
        FloatWidth::Width64 => |i| map(T::parse_f64, Value::F64)(i),
    }
}

pub(crate) fn dlt_type_info<T: NomByteOrder>(
    input: &[u8],
) -> IResult<&[u8], TypeInfo, DltParseError> {
    let (i, info) = T::parse_u32(input)?;
    match TypeInfo::try_from(info) {
        Ok(type_info) => {
            trace!(
                "type_info parsed input: {:02X?} => {:#b}",
                &input[..4],
                info
            );
            Ok((i, type_info))
        }
        Err(_) => {
            let err_msg = format!("dlt_type_info failed to parse {}", T::to_string(input, 4));
            Err(nom::Err::Error(DltParseError::ParsingHickup(err_msg)))
        }
    }
}

pub(crate) fn dlt_fixed_point<T: NomByteOrder>(
    input: &[u8],
    width: FloatWidth,
) -> IResult<&[u8], FixedPoint, DltParseError> {
    let (i, quantization) = T::parse_f32(input)?;
    if width == FloatWidth::Width32 {
        let (rest, offset) = T::parse_i32(i)?;
        Ok((
            rest,
            FixedPoint {
                quantization,
                offset: FixedPointValue::I32(offset),
            },
        ))
    } else if width == FloatWidth::Width64 {
        let (rest, offset) = T::parse_i64(i)?;
        Ok((
            rest,
            FixedPoint {
                quantization,
                offset: FixedPointValue::I64(offset),
            },
        ))
    } else {
        let err_msg = "error in dlt_fixed_point".to_string();
        Err(nom::Err::Error(DltParseError::ParsingHickup(err_msg)))
    }
}

pub(crate) fn dlt_argument<T: NomByteOrder>(
    input: &[u8],
) -> IResult<&[u8], Argument, DltParseError> {
    let (i, type_info) = dlt_type_info::<T>(input)?;
    dbg_parsed("type info", input, i, &type_info);
    match type_info.kind {
        TypeInfoKind::Signed(width) => {
            let (before_val, name_unit) = dlt_variable_name_and_unit::<T>(&type_info)(i)?;
            dbg_parsed("name and unit", i, before_val, &name_unit);
            let (rest, value) = dlt_sint::<T>(width)(before_val)?;
            dbg_parsed("sint", before_val, rest, &value);
            Ok((
                rest,
                Argument {
                    name: name_unit.0,
                    unit: name_unit.1,
                    value,
                    fixed_point: None,
                    type_info,
                },
            ))
        }
        TypeInfoKind::SignedFixedPoint(width) => {
            let (before_val, name_unit) = dlt_variable_name_and_unit::<T>(&type_info)(i)?;
            dbg_parsed("name and unit", i, before_val, &name_unit);
            let (r, fp) = dlt_fixed_point::<T>(before_val, width)?;
            let (after_fixed_point, fixed_point) = (r, Some(fp));
            dbg_parsed("fixed_point", before_val, after_fixed_point, &fixed_point);
            let (rest, value) =
                dlt_sint::<T>(float_width_to_type_length(width))(after_fixed_point)?;
            Ok((
                rest,
                Argument {
                    name: name_unit.0,
                    unit: name_unit.1,
                    value,
                    fixed_point,
                    type_info,
                },
            ))
        }
        TypeInfoKind::Unsigned(width) => {
            let (before_val, (name, unit)) = dlt_variable_name_and_unit::<T>(&type_info)(i)?;
            let (rest, value) = dlt_uint::<T>(width)(before_val)?;
            dbg_parsed("unsigned", before_val, rest, &value);
            Ok((
                rest,
                Argument {
                    name,
                    unit,
                    value,
                    fixed_point: None,
                    type_info,
                },
            ))
        }
        TypeInfoKind::UnsignedFixedPoint(width) => {
            let (before_val, (name, unit)) = dlt_variable_name_and_unit::<T>(&type_info)(i)?;
            let (after_fixed_point, fixed_point) = {
                let (r, fp) = dlt_fixed_point::<T>(before_val, width)?;
                (r, Some(fp))
            };
            let (rest, value) =
                dlt_uint::<T>(float_width_to_type_length(width))(after_fixed_point)?;
            Ok((
                rest,
                Argument {
                    type_info,
                    name,
                    unit,
                    fixed_point,
                    value,
                },
            ))
        }
        TypeInfoKind::Float(width) => {
            let (rest, ((name, unit), value)) = tuple((
                dlt_variable_name_and_unit::<T>(&type_info),
                dlt_fint::<T>(width),
            ))(i)?;
            Ok((
                rest,
                Argument {
                    name,
                    unit,
                    value,
                    fixed_point: None,
                    type_info,
                },
            ))
        }
        TypeInfoKind::Raw => {
            let (i2, raw_byte_cnt) = T::parse_u16(i)?;
            let (i3, name) = if type_info.has_variable_info {
                map(dlt_variable_name::<T>, Some)(i2)?
            } else {
                (i2, None)
            };
            let (rest, value) = map(take(raw_byte_cnt), |s: &[u8]| Value::Raw(s.to_vec()))(i3)?;
            Ok((
                rest,
                Argument {
                    name,
                    unit: None,
                    value,
                    fixed_point: None,
                    type_info,
                },
            ))
        }
        TypeInfoKind::Bool => {
            let (after_var_name, name) = if type_info.has_variable_info {
                map(dlt_variable_name::<T>, Some)(i)?
            } else {
                (i, None)
            };
            dbg_parsed("var name", i, after_var_name, &name);
            let (rest, bool_value) = be_u8(after_var_name)?;
            dbg_parsed("bool value", after_var_name, rest, &bool_value);
            Ok((
                rest,
                Argument {
                    type_info,
                    name,
                    unit: None,
                    fixed_point: None,
                    value: Value::Bool(bool_value),
                },
            ))
        }
        TypeInfoKind::StringType => {
            let (i2, size) = T::parse_u16(i)?;
            let (i3, name) = if type_info.has_variable_info {
                map(dlt_variable_name::<T>, Some)(i2)?
            } else {
                (i2, None)
            };
            let (rest, value) = dlt_zero_terminated_string_intern(i3, size as usize)?;
            dbg_parsed("StringType", i3, rest, &value);
            Ok((
                rest,
                Argument {
                    name,
                    unit: None,
                    fixed_point: None,
                    value: Value::StringVal(value.to_string()),
                    type_info,
                },
            ))
        }
    }
}

#[allow(dead_code)]
struct DltArgumentParser {
    current_index: Option<usize>,
}

fn dlt_payload<T: NomByteOrder>(
    input: &[u8],
    verbose: bool,
    payload_length: u16,
    arg_cnt: u8,
    msg_type: Option<MessageType>,
) -> IResult<&[u8], PayloadContent, DltParseError> {
    if verbose {
        match count(dlt_argument::<T>, arg_cnt as usize)(input) {
            Ok((rest, arguments)) => {
                if let Some(MessageType::NetworkTrace(_)) = msg_type {
                    let slices = arguments
                        .iter()
                        .filter_map(|i| match &i.value {
                            Value::Raw(bytes) => Some(bytes.clone()),
                            _ => None,
                        })
                        .collect();
                    Ok((rest, PayloadContent::NetworkTrace(slices)))
                } else {
                    Ok((rest, PayloadContent::Verbose(arguments)))
                }
            }
            Err(e) => Err(add_context(
                e,
                format!("Problem parsing {} arguments", arg_cnt),
            )),
        }
    } else if let Some(MessageType::Control(_)) = msg_type {
        if payload_length < 1 {
            return Err(nom::Err::Failure(DltParseError::ParsingHickup(format!(
                "error, payload too short {}",
                payload_length
            ))));
        }
        match tuple((nom::number::complete::be_u8, take(payload_length - 1)))(input) {
            Ok((rest, (control_msg_id, payload))) => Ok((
                rest,
                PayloadContent::ControlMsg(
                    ControlType::from_value(control_msg_id),
                    payload.to_vec(),
                ),
            )),
            Err(e) => Err(e),
        }
    } else {
        if payload_length < 4 {
            return Err(nom::Err::Failure(DltParseError::ParsingHickup(format!(
                "error, payload too short {}",
                payload_length
            ))));
        }
        match tuple((T::parse_u32, take(payload_length - 4)))(input) {
            Ok((rest, (message_id, payload))) => Ok((
                rest,
                PayloadContent::NonVerbose(message_id, payload.to_vec()),
            )),
            Err(e) => Err(e),
        }
    }
}

#[inline]
fn dbg_parsed<T: std::fmt::Debug>(_name: &str, _before: &[u8], _after: &[u8], _value: &T) {
    // #[cfg(feature = "debug")]
    {
        let input_len = _before.len();
        let now_len = _after.len();
        let parsed_len = input_len - now_len;
        if parsed_len == 0 {
            trace!("{}: not parsed", _name);
        } else {
            trace!(
                "parsed {} ({} bytes: {:02X?}) => {:?}",
                _name,
                parsed_len,
                &_before[0..parsed_len],
                _value
            );
        }
    }
}

/// Used when producing messages in a stream, indicates if messages
/// where filtered or could not be parsed
#[derive(Debug, PartialEq)]
pub enum ParsedMessage {
    /// Regular message, could be parsed
    Item(Message),
    /// message was filtered out due to filter conditions (Log-Level etc.)
    FilteredOut(usize),
    /// Parsed message was invalid, no parse possible
    Invalid,
}

/// Parse a DLT-message from some binary input data.
///
/// A DLT message looks like this: `<STANDARD-HEADER><EXTENDED-HEADER><PAYLOAD>`
///
/// if stored, an additional header is placed BEFORE all of this `<storage-header><...>`
///
/// example: `444C5401 262CC94D D8A20C00 45435500 3500001F 45435500 3F88623A 16014150 5000434F 4E001100 00000472 656D6F`
/// --------------------------------------------
/// `<STORAGE-HEADER>: 444C5401 262CC94D D8A20C00 45435500`
///     444C5401 = DLT + 0x01 (DLT Pattern)
///  timestamp_sec: 262CC94D = 0x4DC92C26
///  timestamp_us: D8A20C00 = 0x000CA2D8
///  ecu-id: 45435500 = b"ECU\0"
///
/// 3500001F 45435500 3F88623A 16014150 5000434F 4E001100 00000472 656D6F (31 byte)
/// --------------------------------------------
/// `<HEADER>: 35 00 001F 45435500 3F88623A`
///   header type = 0x35 = 0b0011 0101
///       UEH: 1 - > using extended header
///       MSBF: 0 - > little endian
///       WEID: 1 - > with ecu id
///       WSID: 0 - > no session id
///       WTMS: 1 - > with timestamp
///   message counter = 0x00 = 0
///   length = 001F = 31
///   ecu-id = 45435500 = "ECU "
///   timestamp = 3F88623A = 106590265.0 ms since ECU startup (~30 h)
/// --------------------------------------------
/// `<EXTENDED HEADER>: 16014150 5000434F 4E00`
///   message-info MSIN = 0x16 = 0b0001 0110
///   0 -> non-verbose
///   011 (MSTP Message Type) = 0x3 = Dlt Control Message
///   0001 (MTIN Message Type Info) = 0x1 = Request Control Message
///   number of arguments NOAR = 0x01
///   application id = 41505000 = "APP "
///   context id = 434F4E00 = "CON "
/// --------------------------------------------
/// payload: 1100 00000472 656D6F
///   0x11 == SetDefaultLogLevel
///     00 == new log level (block all messages)
///
pub fn dlt_message<'a>(
    input: &'a [u8],
    filter_config_opt: Option<&filtering::ProcessedDltFilterConfig>,
    with_storage_header: bool,
) -> Result<(&'a [u8], ParsedMessage), DltParseError> {
    dlt_message_intern(input, filter_config_opt, with_storage_header).map_err(DltParseError::from)
}

fn dlt_message_intern<'a>(
    input: &'a [u8],
    filter_config_opt: Option<&filtering::ProcessedDltFilterConfig>,
    with_storage_header: bool,
) -> IResult<&'a [u8], ParsedMessage, DltParseError> {
    let (after_storage_header, storage_header_shifted): (&[u8], Option<(StorageHeader, u64)>) =
        if with_storage_header {
            dlt_storage_header(input)?
        } else {
            (input, None)
        };
    if let Some((storage_header, shifted)) = &storage_header_shifted {
        dbg_parsed(
            "storage header",
            &input[(*shifted as usize)..],
            after_storage_header,
            &storage_header,
        )
    };
    let (after_storage_and_normal_header, header) = dlt_standard_header(after_storage_header)?;
    dbg_parsed(
        "normal header",
        after_storage_header,
        after_storage_and_normal_header,
        &header,
    );

    let payload_length_res = validated_payload_length(&header, after_storage_header.len());

    let mut verbose: bool = false;
    let mut msg_type: Option<MessageType> = None;
    let mut arg_count = 0;
    let (after_headers, extended_header) = if header.has_extended_header {
        let (rest, ext_header) = dlt_extended_header(after_storage_and_normal_header)?;
        verbose = ext_header.verbose;
        arg_count = ext_header.argument_count;
        msg_type = Some(ext_header.message_type.clone());
        dbg_parsed(
            "extended header",
            after_storage_and_normal_header,
            rest,
            &ext_header,
        );
        (rest, Some(ext_header))
    } else {
        (after_storage_and_normal_header, None)
    };
    let payload_length = match payload_length_res {
        Ok(length) => length,
        Err(DltParseError::IncompleteParse { needed }) => {
            return Err(nom::Err::Incomplete(
                needed.map_or(nom::Needed::Unknown, nom::Needed::Size),
            ))
        }
        Err(e) => {
            warn!("No validated payload length: {}", e);
            return Ok((after_storage_and_normal_header, ParsedMessage::Invalid));
        }
    };
    if filtered_out(
        extended_header.as_ref(),
        filter_config_opt,
        header.ecu_id.as_ref(),
    ) {
        let (after_message, _) = take(payload_length)(after_headers)?;
        return Ok((
            after_message,
            ParsedMessage::FilteredOut(payload_length as usize),
        ));
    }
    let (i, payload) = if header.endianness == Endianness::Big {
        dlt_payload::<BigEndian>(after_headers, verbose, payload_length, arg_count, msg_type)?
    } else {
        dlt_payload::<LittleEndian>(after_headers, verbose, payload_length, arg_count, msg_type)?
    };
    dbg_parsed("payload", after_headers, i, &payload);
    Ok((
        i,
        ParsedMessage::Item(Message {
            storage_header: storage_header_shifted.map(|shs| shs.0),
            header,
            extended_header,
            payload,
        }),
    ))
}

fn filtered_out(
    extended_header: Option<&ExtendedHeader>,
    filter_config_opt: Option<&filtering::ProcessedDltFilterConfig>,
    ecu_id: Option<&String>,
) -> bool {
    if let Some(filter_config) = filter_config_opt {
        if let Some(h) = &extended_header {
            if let Some(min_filter_level) = filter_config.min_log_level {
                if h.skip_with_level(min_filter_level) {
                    // trace!("no need to parse further, skip payload (skipped level)");
                    return true;
                }
            }
            if let Some(only_these_components) = &filter_config.app_ids {
                if !only_these_components.contains(&h.application_id) {
                    // trace!("no need to parse further, skip payload (skipped app id)");
                    return true;
                }
            }
            if let Some(only_these_context_ids) = &filter_config.context_ids {
                if !only_these_context_ids.contains(&h.context_id) {
                    // trace!("no need to parse further, skip payload (skipped context id)");
                    return true;
                }
            }
            if let Some(only_these_ecu_ids) = &filter_config.ecu_ids {
                if let Some(ecu_id) = ecu_id {
                    if !only_these_ecu_ids.contains(ecu_id) {
                        // trace!("no need to parse further, skip payload (skipped ecu id)");
                        return true;
                    }
                }
            }
        } else {
            // filter out some messages when we do not have an extended header
            if let Some(app_id_set) = &filter_config.app_ids {
                if filter_config.app_id_count > app_id_set.len() as i64 {
                    // some app id was filtered, ignore this entry
                    return true;
                }
            }
            if let Some(context_id_set) = &filter_config.context_ids {
                if filter_config.context_id_count > context_id_set.len() as i64 {
                    // some context id was filtered, ignore this entry
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn validated_payload_length(
    header: &StandardHeader,
    remaining_bytes: usize,
) -> Result<u16, DltParseError> {
    let message_length = header.overall_length();
    let headers_length = calculate_all_headers_length(header.header_type_byte());
    if message_length < headers_length {
        return Err(DltParseError::ParsingHickup(
            "Parsed message-length is less then the length of all headers".to_string(),
        ));
    }

    if message_length as usize > remaining_bytes {
        return Err(DltParseError::IncompleteParse {
            needed: std::num::NonZeroUsize::new(message_length as usize - remaining_bytes),
        });
    }
    let payload_length = message_length - headers_length;
    Ok(payload_length)
}

#[allow(clippy::useless_conversion)]
/// Remove the storage header from the input if present
pub fn skip_storage_header(input: &[u8]) -> Result<(&[u8], u64), DltParseError> {
    let (i, (_, _, _)): (&[u8], _) = tuple((tag("DLT"), tag(&[0x01]), take(12usize)))(input)
        .map_err(nom::Err::<DltParseError>::from)?;
    if input.len() - i.len() == STORAGE_HEADER_LENGTH as usize {
        Ok((i, STORAGE_HEADER_LENGTH))
    } else {
        Err(DltParseError::ParsingHickup(
            "did not match DLT pattern".into(),
        ))
    }
}

#[allow(clippy::useless_conversion)]
/// Skip one dlt message in the input stream in an efficient way
/// pre: message to be parsed contains a storage header
pub fn dlt_consume_msg(input: &[u8]) -> Result<(&[u8], Option<u64>), DltParseError> {
    if input.is_empty() {
        return Ok((input, None));
    }
    let (after_storage_header, skipped_bytes) = skip_storage_header(input)?;
    let (_, header) = dlt_standard_header(after_storage_header)?;
    let overall_length_without_storage_header = header.overall_length();
    let (after_message, _) = take(overall_length_without_storage_header)(after_storage_header)
        .map_err(nom::Err::<DltParseError>::from)?;
    let consumed = skipped_bytes + overall_length_without_storage_header as u64;
    Ok((after_message, Some(consumed)))
}

/// if the type-info for the payload arguments is provided, this
/// function parses and creates the individual arguments from payload data
/// and returns the rest of data not yet consumed.
pub fn construct_arguments<'b>(
    endianness: Endianness,
    pdu_signal_types: &[TypeInfo],
    data: &'b [u8],
) -> Result<(&'b [u8], Vec<Argument>), DltParseError> {
    let mut offset = 0;
    let mut arguments = vec![];
    for signal_type in pdu_signal_types {
        let argument = {
            let (value, fixed_point): (Value, Option<FixedPoint>) = {
                let mut fixed_point = None;
                match signal_type.kind {
                    TypeInfoKind::StringType | TypeInfoKind::Raw => {
                        if data.len() < offset + 2 {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        let length = if endianness == Endianness::Big {
                            BigEndian::read_u16(&data[offset..offset + 2]) as usize
                        } else {
                            LittleEndian::read_u16(&data[offset..offset + 2]) as usize
                        };
                        offset += 2;
                        if data.len() < offset + length {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        let v = if signal_type.kind == TypeInfoKind::StringType {
                            Value::StringVal(
                                String::from_utf8(data[offset..offset + length].to_vec()).map_err(
                                    |e| {
                                        DltParseError::ParsingHickup(format!(
                                            "Could not build string: {}",
                                            e
                                        ))
                                    },
                                )?,
                            )
                        } else {
                            Value::Raw(Vec::from(&data[offset..offset + length]))
                        };
                        offset += length;
                        Ok((v, fixed_point))
                    }
                    TypeInfoKind::Bool => {
                        offset += 1;
                        if data.len() < offset {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        Ok((Value::Bool(data[offset - 1]), fixed_point))
                    }
                    TypeInfoKind::Float(width) => {
                        let length = width as usize / 8;
                        if data.len() < offset + length {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        let v = if endianness == Endianness::Big {
                            dlt_fint::<BigEndian>(width)(&data[offset..offset + length])
                        } else {
                            dlt_fint::<LittleEndian>(width)(&data[offset..offset + length])
                        }
                        .map_err(|e| {
                            DltParseError::ParsingHickup(format!("Could not read fint: {}", e))
                        })?
                        .1;
                        offset += length;
                        Ok((v, fixed_point)) as Result<(Value, Option<FixedPoint>), DltParseError>
                    }
                    TypeInfoKind::Signed(length) => {
                        let byte_length = length as usize / 8;
                        if data.len() < offset + byte_length {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        let value_offset = &data[offset..];
                        let (_, v) = if endianness == Endianness::Big {
                            dlt_sint::<BigEndian>(length)(value_offset)
                        } else {
                            dlt_sint::<LittleEndian>(length)(value_offset)
                        }
                        .map_err(|e| {
                            DltParseError::ParsingHickup(format!("Could not read sint: {}", e))
                        })?;
                        offset += byte_length;
                        Ok((v, fixed_point))
                    }
                    TypeInfoKind::SignedFixedPoint(length) => {
                        let byte_length = length as usize / 8;
                        if data.len() < offset + byte_length {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        let (value_offset, fp) = if endianness == Endianness::Big {
                            dlt_fixed_point::<BigEndian>(
                                &data[offset..offset + byte_length],
                                length,
                            )
                        } else {
                            dlt_fixed_point::<LittleEndian>(
                                &data[offset..offset + byte_length],
                                length,
                            )
                        }
                        .map_err(|e| {
                            DltParseError::ParsingHickup(format!(
                                "Could not read fixed point: {}",
                                e
                            ))
                        })?;
                        fixed_point = Some(fp);
                        let (_, v) = if endianness == Endianness::Big {
                            dlt_sint::<BigEndian>(float_width_to_type_length(length))(value_offset)
                        } else {
                            dlt_sint::<LittleEndian>(float_width_to_type_length(length))(
                                value_offset,
                            )
                        }
                        .map_err(|e| {
                            DltParseError::ParsingHickup(format!("Could not read sint: {}", e))
                        })?;
                        offset += byte_length;
                        Ok((v, fixed_point))
                    }
                    TypeInfoKind::Unsigned(length) => {
                        let byte_length = length as usize / 8;
                        if data.len() < offset + byte_length {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        let value_offset = &data[offset..];
                        let (_, v) = if endianness == Endianness::Big {
                            dlt_uint::<BigEndian>(length)(value_offset)
                        } else {
                            dlt_uint::<LittleEndian>(length)(value_offset)
                        }
                        .map_err(|e| {
                            DltParseError::ParsingHickup(format!("Could not read uint: {}", e))
                        })?;
                        offset += byte_length;
                        Ok((v, fixed_point))
                    }
                    TypeInfoKind::UnsignedFixedPoint(length) => {
                        let byte_length = length as usize / 8;
                        if data.len() < offset + byte_length {
                            return Err(DltParseError::ParsingHickup(
                                "Data not long enough".to_owned(),
                            ));
                        }
                        let value_offset = {
                            let (r, fp) = if endianness == Endianness::Big {
                                dlt_fixed_point::<BigEndian>(
                                    &data[offset..offset + byte_length],
                                    length,
                                )
                            } else {
                                dlt_fixed_point::<LittleEndian>(
                                    &data[offset..offset + byte_length],
                                    length,
                                )
                            }
                            .map_err(|e| {
                                DltParseError::ParsingHickup(format!(
                                    "Could not read fixed point: {}",
                                    e
                                ))
                            })?;
                            fixed_point = Some(fp);
                            r
                        };
                        let (_, v) = if endianness == Endianness::Big {
                            dlt_uint::<BigEndian>(float_width_to_type_length(length))(value_offset)
                        } else {
                            dlt_uint::<LittleEndian>(float_width_to_type_length(length))(
                                value_offset,
                            )
                        }
                        .map_err(|e| {
                            DltParseError::ParsingHickup(format!("Could not read float: {}", e))
                        })?;
                        offset += byte_length;
                        Ok((v, fixed_point))
                    }
                }
            }?;

            Argument {
                type_info: signal_type.clone(),
                name: None,
                unit: None,
                fixed_point,
                value,
            }
        };
        arguments.push(argument);
    }
    Ok((&data[offset..], arguments))
}

/// Parse the DLT message length from a slice containing a DLT standard header.
pub(crate) fn parse_length(input: &[u8]) -> IResult<&[u8], u16, DltParseError> {
    let (rest, (_, length)) = tuple((take(2usize), be_u16))(input)?;
    Ok((rest, length))
}
