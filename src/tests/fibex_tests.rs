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
#[cfg(test)]
mod tests {
    use crate::{
        dlt::{StringCoding::*, TypeInfo, TypeInfoKind::*, TypeLength::*},
        fibex::*,
    };
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_fibex_parsing() {
        let fibex = read_fibexes(vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/dlt-messages.xml")
        ])
        .expect("can't parse fibex");

        assert_eq!(
            fibex,
            FibexMetadata {
                frame_map_with_key: HashMap::from([
                    (
                        FrameMetadataIdentification {
                            context_id: "CTX1".to_string(),
                            app_id: "DR".to_string(),
                            frame_id: "ID_65".to_string()
                        },
                        FrameMetadata {
                            short_name: "timeing: ".to_string(),
                            pdus: [
                                PduMetadata {
                                    description: Some("timeing: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: Some("type: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("contextId: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("eventId: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("ts: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength64),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("threadId: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Signed(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                }
                            ]
                            .to_vec(),
                            application_id: Some("DR".to_string()),
                            context_id: Some("CTX1".to_string()),
                            message_type: Some("DLT_TYPE_LOG".to_string()),
                            message_info: Some("DLT_LOG_WARN".to_string())
                        }
                    ),
                    (
                        FrameMetadataIdentification {
                            context_id: "CTX1".to_string(),
                            app_id: "DR".to_string(),
                            frame_id: "ID_64".to_string()
                        },
                        FrameMetadata {
                            short_name: "direction".to_string(),
                            pdus: [
                                PduMetadata {
                                    description: Some("direction".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: Some("speed: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Signed(BitLength64),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("heading: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Signed(BitLength64),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                }
                            ]
                            .to_vec(),
                            application_id: Some("DR".to_string()),
                            context_id: Some("CTX1".to_string()),
                            message_type: Some("DLT_TYPE_LOG".to_string()),
                            message_info: Some("DLT_LOG_WARN".to_string())
                        }
                    )
                ]),
                frame_map: HashMap::from([
                    (
                        "ID_64".to_string(),
                        FrameMetadata {
                            short_name: "direction".to_string(),
                            pdus: [
                                PduMetadata {
                                    description: Some("direction".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: Some("speed: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Signed(BitLength64),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("heading: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Signed(BitLength64),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                }
                            ]
                            .to_vec(),
                            application_id: Some("DR".to_string()),
                            context_id: Some("CTX1".to_string()),
                            message_type: Some("DLT_TYPE_LOG".to_string()),
                            message_info: Some("DLT_LOG_WARN".to_string())
                        }
                    ),
                    (
                        "ID_65".to_string(),
                        FrameMetadata {
                            short_name: "timeing: ".to_string(),
                            pdus: [
                                PduMetadata {
                                    description: Some("timeing: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: Some("type: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("contextId: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("eventId: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("ts: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Unsigned(BitLength64),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                },
                                PduMetadata {
                                    description: Some("threadId: ".to_string()),
                                    signal_types: [].to_vec()
                                },
                                PduMetadata {
                                    description: None,
                                    signal_types: [TypeInfo {
                                        kind: Signed(BitLength32),
                                        coding: ASCII,
                                        has_variable_info: false,
                                        has_trace_info: false
                                    }]
                                    .to_vec()
                                }
                            ]
                            .to_vec(),
                            application_id: Some("DR".to_string()),
                            context_id: Some("CTX1".to_string()),
                            message_type: Some("DLT_TYPE_LOG".to_string()),
                            message_info: Some("DLT_LOG_WARN".to_string())
                        }
                    )
                ])
            }
        );
    }

    #[test]
    fn test_fibex_robustness() {
        let fibex = read_fibexes(vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/robustness.xml")
        ])
        .expect("can't parse fibex");

        println!("{:?}", fibex);
    }
}
