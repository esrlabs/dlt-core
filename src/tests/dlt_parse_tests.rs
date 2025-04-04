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
#[cfg(test)]
mod tests {
    use crate::{
        dlt::*,
        parse::{
            dlt_argument, dlt_consume_msg, dlt_extended_header, dlt_message, dlt_standard_header,
            dlt_storage_header, dlt_type_info, dlt_zero_terminated_string,
            forward_to_next_storage_header, parse_ecu_id, DltParseError, ParsedMessage,
            DLT_PATTERN,
        },
        proptest_strategies::*,
    };
    use core::num::NonZeroUsize;
    use nom::IResult;
    use proptest::prelude::*;
    use std::io::Write;

    use byteorder::{BigEndian, LittleEndian};
    use bytes::BytesMut;
    use pretty_assertions::assert_eq;
    use std::sync::Once;
    extern crate log;

    static INIT: Once = Once::new();

    fn init_logging() {
        INIT.call_once(|| {
            if std::env::var("RUST_LOG").is_err() {
                std::env::set_var("RUST_LOG", "error,dlt_core::dlt_parse=error");
            }
            env_logger::init();
        });
    }

    #[test]
    fn test_skip_to_next_storage_header_later_in_input() {
        let input_1: Vec<u8> = concatenate_arrays(&[0xa, 0xb, 0xc], DLT_PATTERN);
        assert_eq!(
            Some((3, DLT_PATTERN)),
            forward_to_next_storage_header(&input_1)
        );
        let input_2: Vec<u8> = concatenate_arrays(&[0xa, 0xb, 0xc, 0xd], DLT_PATTERN);
        assert_eq!(
            Some((4, DLT_PATTERN)),
            forward_to_next_storage_header(&input_2)
        );
    }
    #[test]
    fn test_skip_to_next_storage_header_immediately_in_input() {
        let input_1 = &DLT_PATTERN;
        let res = forward_to_next_storage_header(input_1);
        assert_eq!(Some((0, DLT_PATTERN)), res);
    }
    #[test]
    fn test_skip_to_next_storage_header_no_more_pattern_match() {
        let input_1 = &[0x1, 0x2, 0x3, 0x4, 0x1, 0x2, 0x3, 0x4, 0x1, 0x2, 0x3];
        let res = forward_to_next_storage_header(input_1);
        assert_eq!(None, res);
    }
    fn concatenate_arrays<T: Clone>(x: &[T], y: &[T]) -> Vec<T> {
        x.iter().chain(y).cloned().collect()
    }

    #[test]
    fn test_dlt_roundtrip_msg() {
        init_logging();
        let raw1 = crate::tests::DLT_MESSAGE_WITH_STORAGE_HEADER;
        match dlt_message(&raw1[..], None, true) {
            Ok((_rest, ParsedMessage::Item(msg))) => {
                let msg_bytes = msg.as_bytes();
                assert_eq!(raw1, msg_bytes);
            }
            _ => panic!("could not parse message"),
        }
    }

    #[test]
    fn test_dlt_bool_msg() {
        init_logging();
        #[rustfmt::skip]
        let raw1: Vec<u8> = vec![
            // --------------- storage header
            /* DLT + 0x01 */ 0x44, 0x4C, 0x54, 0x01,
            /* timestamp sec */ 0x2B, 0x2C, 0xC9, 0x4D,
            /* timestamp us */ 0x7A, 0xE8, 0x01, 0x00,
            /* ecu id "ECU" */ 0x45, 0x43, 0x55, 0x00,
            // --------------- header
            /* header-type       0b0010 0001 */ 0x21,
            /* extended header        | |||^ */
            /* MSBF: 0  little endian | ||^  */
            /* WEID: 0  no ecu id     | |^   */
            /* WSID: 0  sess id       | ^    */
            /* WTMS: 0 no timestamp   ^      */
            /* version nummber 1   ^^^       */
            /* message counter */ 0x0A,
            /* length = 0 */ 0x00, 0x13,
            // --------------- extended header
            0x41, // MSIN 0b0100 0001 => verbose, MST log, ApplicationTraceType::State
            0x01, // arg count
            0x4C, 0x4F, 0x47, 0x00, // app id LOG
            0x54, 0x45, 0x53, 0x32, // context id TES2
            // --------------- payload
            /* type info 0b0001 0000 => type bool */ 0x10, 0x00, 0x00, 0x00,
             0x6F,
        ];
        match dlt_message(&raw1[..], None, true) {
            Ok((_rest, ParsedMessage::Item(msg))) => {
                let msg_bytes = msg.as_bytes();
                assert_eq!(raw1, msg_bytes);
            }
            _ => panic!("could not parse message"),
        }
        // println!("msg bytes: {:02X?}", msg_bytes);
    }

    #[test]
    fn test_dlt_network_trace_msg() {
        init_logging();
        #[rustfmt::skip]
        let raw1: Vec<u8> = vec![
            // --------------- storage header
            /* DLT + 0x01 */ 0x44, 0x4c, 0x54, 0x01, 
            /* timestamp sec */ 0xbc, 0xa6, 0xd4, 0x65, 
            /* timestamp us */ 0x27, 0x13, 0x07, 0x00, 
            /* ecu id "ECU" */ 0x49, 0x44, 0x43, 0x45,
            // --------------- header
            /* header-type       0b0011 1101 */ 0x3d,
            /* extended header        | |||^ */
            /* MSBF: 0  little endian | ||^  */
            /* WEID: 0  no ecu id     | |^   */
            /* WSID: 0  sess id       | ^    */
            /* WTMS: 0 no timestamp   ^      */
            /* version nummber 1   ^^^       */
            /* message counter */ 0x40, 
            /* length = 0 */ 0x00, 0x48, 
            0x49, 0x44, 0x43, 0x45, // ecu-id
            0x00, 0x00, 0x02, 0x4f, // session-id
            0x01, 0xba, 0x71, 0xb6, // timestamp
            // --------------- extended header
            0x15, // MSIN 0b0001 0101 => IPC/NW-Trace
            0x02, // arg count
            0x4e, 0x32, 0x53, 0x49, // app-id
            0x54, 0x43, 0x00, 0x00, // ctx-id
            /* === arg 1 (some/ip info) === */
            /* type info */ 0x00, 0x04, 0x00, 0x00, 0x0a, 0x00, // type rawd + len(16bit)
            0x00, 0x00, 0x00, 0x00, // ipv4
            0x00, 0x00, // port
            0xff, // protocol
            0x01, // direction
            0x00, 0x01, // instance-id (1|2|4 byte)
            /* === arg 2 (some/ip message) === */
            /* type info */ 0x00, 0x04, 0x00, 0x00, 0x18, 0x00, // type rawd + len(16bit)
            0xf9, 0x61, 0x85, 0x1d, 0x00, 0x00, 0x00, 0x10, 
            0x00, 0x00, 0x28, 0x15, 0x01, 0x01, 0x02, 0x00,
            0x01, 0xfe, 0xfc, 0xff, 0xff, 0xff, 0xff, 0xff,
        ];
        match dlt_message(&raw1[..], None, true) {
            Ok((_rest, ParsedMessage::Item(msg))) => {
                let ext_hdr = msg.extended_header.as_ref().unwrap();
                assert!(ext_hdr.verbose);
                assert_eq!(2, ext_hdr.argument_count);
                if let PayloadContent::NetworkTrace(slices) = &msg.payload {
                    assert_eq!(2, slices.len());
                    let slice = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x01, 0x00, 0x01];
                    assert_eq!(slice, slices[0]);
                    let slice = vec![
                        0xf9, 0x61, 0x85, 0x1d, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x28, 0x15,
                        0x01, 0x01, 0x02, 0x00, 0x01, 0xfe, 0xfc, 0xff, 0xff, 0xff, 0xff, 0xff,
                    ];
                    assert_eq!(slice, slices[1]);
                } else {
                    panic!("unexpected payload type");
                }
                let msg_bytes = msg.as_bytes();
                assert_eq!(raw1, msg_bytes);
            }
            _ => panic!("could not parse message"),
        }
        // println!("msg bytes: {:02X?}", msg_bytes);
    }

    #[test]
    fn test_dlt_message_parsing() {
        init_logging();
        #[rustfmt::skip]
        let mut raw1: Vec<u8> = vec![
            0x44, 0x4C, 0x54, 0x01, // storage header
            0x56, 0xA2, 0x91, 0x5C, 0x9C, 0x91, 0x0B, 0x00, 0x45, 0x43, 0x55, 0x31, // header
            0x3D, // header type 0b11 1101
            0x40, 0x00, 0xA2, 0x45, 0x43, 0x55, 0x31, // ecu id
            0x00, 0x00, 0x01, 0x7F, // session id
            0x00, 0x5B, 0xF7, 0x16, // timestamp
            // extended header
            0x51, // MSIN 0b101 0001 => verbose, MST log,
            0x06, // arg count
            0x56, 0x53, 0x6F, 0x6D, // app id VSom
            0x76, 0x73, 0x73, 0x64, // context id vssd
            // arguments
            0x00, 0x82, 0x00, 0x00, // type info 0b1000001000000000
            0x3A, 0x00,
            0x5B, 0x33, 0x38, 0x33, 0x3A, 0x20, 0x53, 0x65,
            0x72, 0x76, 0x69, 0x63, 0x65, 0x44, 0x69, 0x73, 0x63, 0x6F, 0x76, 0x65, 0x72, 0x79,
            0x55, 0x64, 0x70, 0x45, 0x6E, 0x64, 0x70, 0x6F, 0x69, 0x6E, 0x74, 0x28, 0x31, 0x36,
            0x30, 0x2E, 0x34, 0x38, 0x2E, 0x31, 0x39, 0x39, 0x2E, 0x31, 0x30, 0x32, 0x3A, 0x35,
            0x30, 0x31, 0x35, 0x32, 0x29, 0x5D, 0x20, 0x00,
            0x00, 0x82, 0x00, 0x00, // type info 0b1000001000000000
            0x0F, 0x00, // length
            0x50, 0x72, 0x6F, 0x63, 0x65, 0x73, 0x73, 0x4D, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65,
            0x00, // "ProcessMessage"
            0x00, 0x82, 0x00, 0x00, // type info 0b1000001000000000
            0x02, 0x00, // length
            0x3A, 0x00, // ":"
            0x23, 0x00, 0x00, 0x00, // type info 0b10000000001000010
            0x0D, 0x01, 0x00, 0x00, 0x00, 0x82, 0x00, 0x00, 0x03, 0x00, 0x3A, 0x20, 0x00, 0x00,
            0x82, 0x00, 0x00, 0x14, 0x00, 0x31, 0x36, 0x30, 0x2E, 0x34, 0x38, 0x2E, 0x31, 0x39,
            0x39, 0x2E, 0x31, 0x36, 0x2C, 0x33, 0x30, 0x35, 0x30, 0x31, 0x00,
        ];
        let raw2: Vec<u8> = vec![
            0x44, 0x4C, 0x54, 0x01, 0x56, 0xA2, 0x91, 0x5C, 0x9C, 0x91, 0x0B, 0x00, 0x45, 0x43,
            0x55, 0x31, 0x3D, 0x41, 0x00, 0xA9, 0x45, 0x43, 0x55, 0x31, 0x00, 0x00, 0x01, 0x7F,
            0x00, 0x5B, 0xF7, 0x16, 0x51, 0x09, 0x56, 0x53, 0x6F, 0x6D, 0x76, 0x73, 0x73, 0x64,
            0x00, 0x82, 0x00, 0x00, 0x3A, 0x00, 0x5B, 0x33, 0x38, 0x33, 0x3A, 0x20, 0x53, 0x65,
            0x72, 0x76, 0x69, 0x63, 0x65, 0x44, 0x69, 0x73, 0x63, 0x6F, 0x76, 0x65, 0x72, 0x79,
            0x55, 0x64, 0x70, 0x45, 0x6E, 0x64, 0x70, 0x6F, 0x69, 0x6E, 0x74, 0x28, 0x31, 0x36,
            0x30, 0x2E, 0x34, 0x38, 0x2E, 0x31, 0x39, 0x39, 0x2E, 0x31, 0x30, 0x32, 0x3A, 0x35,
            0x30, 0x31, 0x35, 0x32, 0x29, 0x5D, 0x20, 0x00, 0x00, 0x82, 0x00, 0x00, 0x0F, 0x00,
            0x50, 0x72, 0x6F, 0x63, 0x65, 0x73, 0x73, 0x4D, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65,
            0x00, 0x00, 0x82, 0x00, 0x00, 0x02, 0x00, 0x3A, 0x00, 0x23, 0x00, 0x00, 0x00, 0x24,
            0x01, 0x00, 0x00, 0x00, 0x82, 0x00, 0x00, 0x06, 0x00, 0x3A, 0x20, 0x28, 0x30, 0x78,
            0x00, 0x42, 0x00, 0x01, 0x00, 0x36, 0x15, 0x00, 0x82, 0x00, 0x00, 0x04, 0x00, 0x2C,
            0x30, 0x78, 0x00, 0x42, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x82, 0x00, 0x00, 0x02,
            0x00, 0x29, 0x00,
        ];
        #[rustfmt::skip]
        let raw3: Vec<u8> = vec![
            // storage header
            /* DLT + 0x01 */ 0x44, 0x4C, 0x54, 0x01,
            /* timestamp sec = 0x5DB3B890 =  October 26, 2019 3:08:00 AM*/ 0x90, 0xB8, 0xB3, 0x5D,
            /* timestamp ms */ 0x00, 0x00, 0x00, 0x00,
            /* ecu id "ECU" */ 0x45, 0x43, 0x55, 0x00,
            /* header-type       0b0101 0011 */ 0x53,
            /* extended header        | |||^ */
            /* MSBF: 1  big endian    | ||^  */
            /* WEID: 0  no ecu id     | |^   */
            /* WSID: 0  no sess id    | ^    */
            /* WTMS: 1  timestamp     ^      */
            /* version nummber 2   ^^^       */
            /* message counter */ 0x44,
            /* length = 23 */ 0x00, 0x17,
            /* timestamp (ecu/session id missing) 139698662.4 = ~39h */ 0x53, 0x44, 0x53, 0x00,
            /* extended header */ 0x02, 0x00, 0x00, 0x1D, 0x00, 0x5B, 0x50, 0x6F, 0x6C, 0x6C,
            // message-info MSIN = 0x02 = 0b0000 0010
            // 0 = non-verbose              |||| |||^
            // 001 (MSTP) 0x1 Dlt AppTrace  |||| ^^^
            // 0000 (Type Info) = 0x0       ^^^^
            // number of arguments NOAR = 0x00
            // application id = 001D005B = "   ["
            // context id = 506F6C6C = "Poll"
            // ========================================
            // payload
            0x10, 0x00, 0x00, 0x00, 0x6F,
        ];
        raw1.extend_from_slice(&raw2);
        let result_1_2 = match dlt_message(&raw1, None, true) {
            Ok((rest1, _msg)) => dlt_message(rest1, None, true),
            Err(e) => panic!("Parse should succeed: {}", e),
        };
        let res2: Result<(&[u8], ParsedMessage), DltParseError> = dlt_message(&raw2, None, true);
        assert!(res2.is_ok());
        assert_eq!(result_1_2, res2);
        let res3: Result<(&[u8], ParsedMessage), DltParseError> = dlt_message(&raw3, None, true);
        assert!(res3.is_ok());

        for end in 1..raw3.len() - 1 {
            let res3_incomplete = dlt_message(&raw3[..end], None, true);

            assert!(
                matches!(res3_incomplete, Err(DltParseError::IncompleteParse { .. })),
                "Ending before {} did not yield Incomplete",
                end
            );
        }
    }

    static VALID_ECU_ID_FORMAT: &str = "[0-9a-zA-Z]{4}";
    proptest! {
        #[test]
        fn parse_ecu_id_doesnt_crash(s in "\\PC*") {
            let _ = parse_ecu_id(s.as_bytes());
        }
        #[test]
        fn parses_all_valid_ecu_ids(s in VALID_ECU_ID_FORMAT) {
            parse_ecu_id(s.as_bytes()).unwrap();
        }
    }

    #[test]
    fn test_ecu_id_parser() {
        let expected: IResult<&[u8], &str, DltParseError> = Ok((&[], "ecu1"));
        assert_eq!(expected, parse_ecu_id(b"ecu1"));
        assert_eq!(
            Err(nom::Err::Incomplete(nom::Needed::Size(
                NonZeroUsize::new(1).expect("1 is positive")
            ))),
            parse_ecu_id(b"ecu")
        );
    }
    #[test]
    fn test_parse_example_type_info() {
        init_logging();
        let type_info_bytes = vec![0x42, 0x00, 0x01, 0x00];
        trace!("{:02X?}", type_info_bytes);
        let res: TypeInfo = dlt_type_info::<LittleEndian>(&type_info_bytes).unwrap().1;
        let type_info_as_bytes = res.as_bytes::<LittleEndian>();
        assert_eq!(type_info_bytes, type_info_as_bytes);
    }

    proptest! {
        #[test]
        fn test_dlt_all_storage_header(header_to_expect: StorageHeader) {
            trace!("header_to_expect: {:?}", header_to_expect);
            let mut header_bytes = header_to_expect.as_bytes();
            trace!("header bytes: {:02X?}", header_bytes);
            header_bytes.extend(b"----");
            let res: IResult<&[u8], Option<(StorageHeader, u64)>, DltParseError> = dlt_storage_header(&header_bytes);
            if let Ok((_, Some((v, 0)))) = &res {
                trace!("parsed header: {:?}", v)
            }
            let expected: IResult<&[u8], Option<(StorageHeader, u64)>, DltParseError> =
                Ok((b"----", Some((header_to_expect, 0))));
            assert_eq!(expected, res);
        }
        #[test]
        fn test_count_messages(messages in messages_with_storage_header_strat(10)) {
            let mut content = vec![];
            let generated_msg_cnt = messages.len();
            for msg in messages {
                let msg_bytes = msg.as_bytes();
                content.extend(msg_bytes);
            }
            let mut was_message = true;
            let mut msg_cnt = 0usize;
            let mut remaining: &[u8] = &content;
            while was_message {
                if let Ok((rest, consumed))  = dlt_consume_msg(remaining) {
                    was_message = consumed.is_some();
                    remaining = rest;
                    if was_message {
                        msg_cnt += 1;
                    }
                } else {
                    break;
                }
            }
            assert_eq!(msg_cnt, generated_msg_cnt);
        }

        #[test]
        fn test_dlt_standard_header(header_to_expect in header_strategy(4, Endianness::Big)) {
            init_logging();
            let mut header_bytes = header_to_expect.as_bytes();
            header_bytes.extend(b"----");
            let res: IResult<&[u8], StandardHeader, DltParseError> = dlt_standard_header(&header_bytes);
            let expected: IResult<&[u8], StandardHeader, DltParseError> = Ok((b"----", header_to_expect));
            assert_eq!(expected, res);
        }
        #[test]
        fn test_extended_header(header_to_expect: ExtendedHeader) {
            let mut header_bytes = header_to_expect.as_bytes();
            header_bytes.extend(b"----");
            let res: IResult<&[u8], ExtendedHeader, DltParseError> = dlt_extended_header(&header_bytes );
            let expected: IResult<&[u8], ExtendedHeader, DltParseError> = Ok((b"----", header_to_expect));
            assert_eq!(expected, res);
        }
        #[test]
        fn test_parse_type_info(type_info: TypeInfo) {
            let mut type_info_bytes = type_info.as_bytes::<BigEndian>();
            trace!("{:02X?}", type_info_bytes);
            type_info_bytes.extend(b"----");
            let res: IResult<&[u8], TypeInfo, DltParseError> = dlt_type_info::<BigEndian>(&type_info_bytes);
            let expected: IResult<&[u8], TypeInfo, DltParseError> = Ok((b"----", type_info));
            assert_eq!(expected, res);
        }

        #[test]
        fn test_parse_any_argument(argument in argument_strategy()) {
            let mut argument_bytes = argument.as_bytes::<BigEndian>();
            argument_bytes.extend(b"----");
            let res: IResult<&[u8], Argument, DltParseError> = dlt_argument::<BigEndian>(&argument_bytes);
            let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
            assert_eq!(expected, res);
        }
        #[test]
        fn test_argument_to_bytes_to_argument(arg in argument_strategy(), endianness in any::<Endianness>()) {
            init_logging();
            let mut arg_bytes = if endianness == Endianness::Big {
                arg.as_bytes::<BigEndian>()
            } else {
                arg.as_bytes::<LittleEndian>()
            };
            arg_bytes.extend(b"----");
            let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", arg));
            if endianness == Endianness::Big {
                assert_eq!(expected, dlt_argument::<BigEndian>(&arg_bytes));
            } else {
                assert_eq!(expected, dlt_argument::<LittleEndian>(&arg_bytes));
            };
        }
        #[test]
        fn test_message_to_bytes_to_message(msg in message_strat()) {
            init_logging();
            // println!("msg: {:?}", serde_json::to_string(&msg));
            let mut msg_bytes = msg.as_bytes();
            // println!("msg bytes: {:02X?}", msg_bytes);
            msg_bytes.extend(b"----");
            // dump_to_file(&msg_bytes)?;
            let expected: Result<(&[u8], ParsedMessage), DltParseError>  =
                Ok((b"----", ParsedMessage::Item(msg)));
            assert_eq!(expected, dlt_message(&msg_bytes, None, false));
        }
    }

    fn dump_to_file(msg_bytes: &[u8]) -> std::io::Result<()> {
        let home_dir = dirs::home_dir().expect("we need to have access to home-dir");
        let file_path = home_dir.join("testmsg.bin");
        let mut file = std::fs::File::create(file_path)?;
        file.write_all(msg_bytes)
    }

    #[test]
    fn test_parse_msg() {
        init_logging();
        let payload = PayloadContent::Verbose(vec![Argument {
            type_info: TypeInfo {
                kind: TypeInfoKind::Unsigned(TypeLength::BitLength32),
                coding: StringCoding::UTF8,
                has_variable_info: true,
                has_trace_info: false,
            },
            name: Some("UcbfX".to_string()),
            unit: Some("seconds".to_string()),
            fixed_point: None,
            value: Value::U32(2_063_359_909),
        }]);
        let msg_conf = MessageConfig {
            version: 0,
            endianness: Endianness::Big,
            counter: 21,
            ecu_id: Some("AA".to_string()),
            session_id: None,
            timestamp: None,
            payload,
            extended_header_info: Some(ExtendedHeaderConfig {
                message_type: MessageType::Log(LogLevel::Warn),
                app_id: "o".to_string(),
                context_id: "hK".to_string(),
            }),
        };
        let msg = Message::new(msg_conf, None);
        let mut msg_bytes = msg.as_bytes();
        dump_to_file(&msg_bytes).expect("could not dump bytes");
        println!("--> test_parse_msg: msg_bytes: {:02X?}", msg_bytes);

        msg_bytes.extend(b"----");
        let res = dlt_message(&msg_bytes, None, false);
        let expected: Result<(&[u8], ParsedMessage), DltParseError> =
            Ok((b"----", ParsedMessage::Item(msg)));
        assert_eq!(expected, res);
    }

    #[test]
    fn test_parse_offending_argument() {
        let type_info = TypeInfo {
            kind: TypeInfoKind::SignedFixedPoint(FloatWidth::Width64),
            coding: StringCoding::UTF8,
            has_variable_info: true,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: Some("a".to_string()),
            unit: Some("a".to_string()),
            fixed_point: Some(FixedPoint {
                quantization: 1.0,
                offset: FixedPointValue::I64(1),
            }),
            value: Value::I64(-1_246_093_129_526_187_791),
        };

        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }
    #[test]
    fn test2_parse_offending_argument() {
        let argument = Argument {
            type_info: TypeInfo {
                kind: TypeInfoKind::SignedFixedPoint(FloatWidth::Width32),
                coding: StringCoding::UTF8,
                has_variable_info: true,
                has_trace_info: false,
            },
            name: Some("a".to_string()),
            unit: Some("A".to_string()),
            fixed_point: Some(FixedPoint {
                quantization: 0.1,
                offset: FixedPointValue::I32(0),
            }),
            value: Value::I32(1_319_631_541),
        };

        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }
    #[test]
    fn test_parse_bool_argument() {
        {
            let type_info = TypeInfo {
                kind: TypeInfoKind::Bool,
                coding: StringCoding::UTF8,
                has_variable_info: false,
                has_trace_info: false,
            };
            let argument = Argument {
                type_info,
                name: None,
                unit: None,
                fixed_point: None,
                value: Value::Bool(0x1),
            };
            println!("argument: {:?}", argument);
            let mut argument_bytes = argument.as_bytes::<BigEndian>();
            println!("argument bytes: {:02X?}", argument_bytes);
            argument_bytes.extend(b"4321");
            let res: IResult<&[u8], Argument, DltParseError> =
                dlt_argument::<BigEndian>(&argument_bytes);
            let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"4321", argument));
            assert_eq!(expected, res);
        }
        // now with variable info
        {
            let type_info = TypeInfo {
                kind: TypeInfoKind::Bool,
                coding: StringCoding::UTF8,
                has_variable_info: true,
                has_trace_info: false,
            };
            let argument = Argument {
                type_info,
                name: Some("abc".to_string()),
                unit: None,
                fixed_point: None,
                value: Value::Bool(0x1),
            };
            println!("argument: {:?}", argument);
            let mut argument_bytes = argument.as_bytes::<BigEndian>();
            println!("argument bytes: {:02X?}", argument_bytes);
            argument_bytes.extend(b"----");
            let res: IResult<&[u8], Argument, DltParseError> =
                dlt_argument::<BigEndian>(&argument_bytes);
            let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
            assert_eq!(expected, res);
        }
    }
    #[test]
    fn test_parse_unsigned_argument() {
        {
            let type_info = TypeInfo {
                kind: TypeInfoKind::Unsigned(TypeLength::BitLength32),
                coding: StringCoding::UTF8,
                has_variable_info: false,
                has_trace_info: false,
            };
            let argument = Argument {
                type_info,
                name: None,
                unit: None,
                fixed_point: None,
                value: Value::U32(0x123),
            };
            let mut argument_bytes = argument.as_bytes::<BigEndian>();
            trace!("argument bytes: {:02X?}", argument_bytes);
            argument_bytes.extend(b"----");
            let res: IResult<&[u8], Argument, DltParseError> =
                dlt_argument::<BigEndian>(&argument_bytes);
            let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
            assert_eq!(expected, res);
        }
        // now with variable info
        {
            let type_info = TypeInfo {
                kind: TypeInfoKind::Unsigned(TypeLength::BitLength32),
                coding: StringCoding::UTF8,
                has_variable_info: true,
                has_trace_info: false,
            };
            let argument = Argument {
                type_info,
                name: Some("speed".to_string()),
                unit: Some("mph".to_string()),
                fixed_point: None,
                value: Value::U32(0x123),
            };
            let mut argument_bytes = argument.as_bytes::<BigEndian>();
            trace!("argument bytes: {:02X?}", argument_bytes);
            argument_bytes.extend(b"----");
            let res: IResult<&[u8], Argument, DltParseError> =
                dlt_argument::<BigEndian>(&argument_bytes);
            let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
            assert_eq!(expected, res);
        }
    }
    #[test]
    fn test_problem_arg() {
        let argument = Argument {
            type_info: TypeInfo {
                kind: TypeInfoKind::Unsigned(TypeLength::BitLength32),
                coding: StringCoding::UTF8,
                has_variable_info: true,
                has_trace_info: false,
            },
            name: Some("UcbfX".to_string()),
            unit: Some("seconds".to_string()),
            fixed_point: None,
            value: Value::U32(2_063_359_909),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }
    #[test]
    fn test_parse_signed_argument() {
        let type_info = TypeInfo {
            kind: TypeInfoKind::Signed(TypeLength::BitLength16),
            coding: StringCoding::UTF8,
            has_variable_info: false,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: None,
            unit: None,
            fixed_point: None,
            value: Value::I16(-23),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
        // now with variable info
        let type_info = TypeInfo {
            kind: TypeInfoKind::Signed(TypeLength::BitLength32),
            coding: StringCoding::UTF8,
            has_variable_info: true,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: Some("temperature".to_string()),
            unit: Some("celcius".to_string()),
            fixed_point: None,
            value: Value::I32(-23),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }
    #[test]
    fn test_parse_float_argument() {
        let type_info = TypeInfo {
            kind: TypeInfoKind::Float(FloatWidth::Width32),
            coding: StringCoding::UTF8,
            has_variable_info: false,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: None,
            unit: None,
            fixed_point: None,
            value: Value::F32(123.98f32),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
        // now with variable info
        let type_info = TypeInfo {
            kind: TypeInfoKind::Float(FloatWidth::Width64),
            coding: StringCoding::UTF8,
            has_variable_info: true,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: Some("temperature".to_string()),
            unit: Some("celcius".to_string()),
            fixed_point: None,
            value: Value::F64(28.3),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }
    #[test]
    fn test_parse_raw_argument() {
        let type_info = TypeInfo {
            kind: TypeInfoKind::Raw,
            coding: StringCoding::UTF8,
            has_variable_info: false,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: None,
            unit: None,
            fixed_point: None,
            value: Value::Raw(vec![0xD, 0xE, 0xA, 0xD]),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
        // now with variable info
        let type_info = TypeInfo {
            kind: TypeInfoKind::Raw,
            coding: StringCoding::UTF8,
            has_variable_info: true,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: Some("payload".to_string()),
            unit: None,
            fixed_point: None,
            value: Value::Raw(vec![0xD, 0xE, 0xA, 0xD]),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes raw: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }

    #[test]
    fn test_parse_string_argument() {
        let type_info = TypeInfo {
            kind: TypeInfoKind::StringType,
            coding: StringCoding::UTF8,
            has_variable_info: false,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: None,
            unit: None,
            fixed_point: None,
            value: Value::StringVal("foo".to_string()),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }

    #[test]
    fn test_parse_fixed_point_argument() {
        let type_info = TypeInfo {
            kind: TypeInfoKind::SignedFixedPoint(FloatWidth::Width32),
            coding: StringCoding::UTF8,
            has_variable_info: true,
            has_trace_info: false,
        };
        let argument = Argument {
            type_info,
            name: Some("speed".to_string()),
            unit: Some("mph".to_string()),
            value: Value::I32(-44),
            fixed_point: Some(FixedPoint {
                quantization: 1.5,
                offset: FixedPointValue::I32(-200),
            }),
        };
        let mut argument_bytes = argument.as_bytes::<BigEndian>();
        trace!("argument bytes: {:02X?}", argument_bytes);
        argument_bytes.extend(b"----");
        let res: IResult<&[u8], Argument, DltParseError> =
            dlt_argument::<BigEndian>(&argument_bytes);
        let expected: IResult<&[u8], Argument, DltParseError> = Ok((b"----", argument));
        assert_eq!(expected, res);
    }

    #[test]
    fn test_dlt_zero_terminated_string_exact() {
        let mut buf = BytesMut::with_capacity(4);
        buf.extend_from_slice(b"id42");
        let res = dlt_zero_terminated_string(&buf, 4);
        let expected: Result<(&[u8], &str), DltParseError> = Ok((&[], "id42"));
        assert_eq!(expected, res);
    }
    #[test]
    fn test_dlt_zero_terminated_string_more_data() {
        let mut buf = BytesMut::with_capacity(6);
        buf.extend_from_slice(b"id42++");
        let res = dlt_zero_terminated_string(&buf, 4);
        let expected: Result<(&[u8], &str), DltParseError> = Ok((b"++", "id42"));
        assert_eq!(expected, res);
    }
    #[test]
    fn test_dlt_zero_terminated_string_less_data() {
        let mut buf = BytesMut::with_capacity(4);
        buf.extend_from_slice(b"id\0");
        assert!(matches!(
            dlt_zero_terminated_string(&buf, 4),
            Err(DltParseError::IncompleteParse { .. })
        ));
        buf.clear();
        buf.extend_from_slice(b"id\0\0");
        let expected: Result<(&[u8], &str), DltParseError> = Ok((b"", "id"));
        assert_eq!(expected, dlt_zero_terminated_string(&buf, 4));
    }
    #[test]
    fn test_dlt_zero_terminated_string_early_terminated() {
        let mut buf = BytesMut::with_capacity(4);
        buf.extend_from_slice(b"id4\0somethingelse");
        let res = dlt_zero_terminated_string(&buf, 4);
        trace!("res : {:?}", res);
        let expected: Result<(&[u8], &str), DltParseError> = Ok((b"somethingelse", "id4"));
        assert_eq!(expected, res);
    }
    #[test]
    fn test_dlt_zero_terminated_string_non_utf8() {
        let mut buf = BytesMut::with_capacity(4);
        let broken = vec![0x41, 0, 146, 150];
        buf.extend_from_slice(&broken);
        let res = dlt_zero_terminated_string(&buf, 4);
        let expected: Result<(&[u8], &str), DltParseError> = Ok((b"", "A"));
        assert_eq!(expected, res);
    }
}
