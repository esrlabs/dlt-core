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

//! # dlt reading support
use crate::{
    dlt::{HEADER_MIN_LENGTH, STORAGE_HEADER_LENGTH},
    filtering::ProcessedDltFilterConfig,
    parse::{dlt_message, parse_length, DltParseError, ParsedMessage},
};
use std::io::{BufReader, Read};

// The default capacity for the internal buffered reader.
pub(crate) const DEFAULT_BUFFER_CAPACITY: usize = 10 * 1024 * 1024;

// The default length of the maximum message to be parsed.
pub(crate) const DEFAULT_MESSAGE_MAX_LEN: usize =
    STORAGE_HEADER_LENGTH as usize + u16::MAX as usize;

/// Read and parse the next DLT message from the given reader, if any
pub fn read_message<S: Read>(
    reader: &mut DltMessageReader<S>,
    filter_config_opt: Option<&ProcessedDltFilterConfig>,
) -> Result<Option<ParsedMessage>, DltParseError> {
    let with_storage_header = reader.with_storage_header();
    let slice = reader.next_message_slice()?;

    if !slice.is_empty() {
        Ok(Some(
            dlt_message(slice, filter_config_opt, with_storage_header)?.1,
        ))
    } else {
        Ok(None)
    }
}

/// Buffered reader for DLT message slices from a source.
pub struct DltMessageReader<S: Read> {
    source: BufReader<S>,
    with_storage_header: bool,
    buffer: Vec<u8>,
}

impl<S: Read> DltMessageReader<S> {
    /// Create a new reader for the given source.
    pub fn new(source: S, with_storage_header: bool) -> Self {
        DltMessageReader::with_capacity(
            DEFAULT_BUFFER_CAPACITY,
            DEFAULT_MESSAGE_MAX_LEN,
            source,
            with_storage_header,
        )
    }

    /// Create a new reader for the given source and specified capacities.
    pub fn with_capacity(
        buffer_capacity: usize,
        message_max_len: usize,
        source: S,
        with_storage_header: bool,
    ) -> Self {
        debug_assert!(buffer_capacity >= message_max_len);

        DltMessageReader {
            source: BufReader::with_capacity(buffer_capacity, source),
            with_storage_header,
            buffer: vec![0u8; message_max_len],
        }
    }

    /// Read the next message slice from the source,
    /// or return an empty slice if no more message could be read.
    pub fn next_message_slice(&mut self) -> Result<&[u8], DltParseError> {
        let storage_len = if self.with_storage_header {
            STORAGE_HEADER_LENGTH as usize
        } else {
            0
        };
        let header_len = storage_len + HEADER_MIN_LENGTH as usize;
        debug_assert!(header_len <= self.buffer.len());

        if self
            .source
            .read_exact(&mut self.buffer[..header_len])
            .is_err()
        {
            return Ok(&[]);
        }

        let (_, message_len) = parse_length(&self.buffer[storage_len..header_len])?;
        let total_len = storage_len + message_len as usize;
        debug_assert!(total_len <= self.buffer.len());

        self.source
            .read_exact(&mut self.buffer[header_len..total_len])?;

        Ok(&self.buffer[..total_len])
    }

    /// Answer if message slices contain a `StorageHeader´.
    pub fn with_storage_header(&self) -> bool {
        self.with_storage_header
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dlt::Message,
        proptest_strategies::{messages_strat, messages_with_storage_header_strat},
        tests::{DLT_MESSAGE, DLT_MESSAGE_WITH_STORAGE_HEADER},
    };
    use proptest::prelude::*;

    #[test]
    fn test_message_reader() {
        let messages_with_storage = [
            (DLT_MESSAGE, false),
            (DLT_MESSAGE_WITH_STORAGE_HEADER, true),
        ];

        for message_with_storage in &messages_with_storage {
            let bytes = message_with_storage.0;
            let with_storage_header = message_with_storage.1;

            let mut reader = DltMessageReader::new(bytes, with_storage_header);
            assert_eq!(with_storage_header, reader.with_storage_header());

            let slice = reader.next_message_slice().expect("message");
            assert_eq!(bytes, slice);

            assert!(reader.next_message_slice().expect("message").is_empty());
        }
    }

    #[test]
    fn test_read_message() {
        let messages_with_storage = [
            (DLT_MESSAGE, false),
            (DLT_MESSAGE_WITH_STORAGE_HEADER, true),
        ];

        for message_with_storage in &messages_with_storage {
            let bytes = message_with_storage.0;
            let with_storage_header = message_with_storage.1;

            let mut reader = DltMessageReader::new(bytes, with_storage_header);

            if let Some(ParsedMessage::Item(message)) =
                read_message(&mut reader, None).expect("message")
            {
                assert_eq!(bytes, message.as_bytes());
            }

            assert_eq!(None, read_message(&mut reader, None).expect("message"))
        }
    }

    proptest! {
        #[test]
        fn test_read_messages_proptest(messages in messages_strat(10)) {
            test_read_messages(messages, false);
        }
        #[test]
        fn test_read_messages_with_storage_header_proptest(messages in messages_with_storage_header_strat(10)) {
            test_read_messages(messages, true);
        }
    }

    fn test_read_messages(messages: Vec<Message>, with_storage_header: bool) {
        let mut bytes = vec![];
        for message in &messages {
            bytes.extend(message.as_bytes());
        }

        let mut reader = DltMessageReader::new(bytes.as_slice(), with_storage_header);
        let mut parsed = 0usize;

        loop {
            match read_message(&mut reader, None).expect("read") {
                Some(ParsedMessage::Item(message)) => {
                    assert_eq!(messages.get(parsed).unwrap().as_bytes(), message.as_bytes());
                    parsed += 1;
                }
                None => {
                    break;
                }
                _ => {
                    panic!("unexpected item");
                }
            };
        }

        assert_eq!(messages.len(), parsed);
    }
}
