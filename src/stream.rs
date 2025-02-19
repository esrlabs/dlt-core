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

//! # dlt streaming support
use crate::{
    dlt::{HEADER_MIN_LENGTH, STORAGE_HEADER_LENGTH},
    filtering::ProcessedDltFilterConfig,
    parse::{dlt_message, parse_length, DltParseError, ParsedMessage},
    read::{DEFAULT_BUFFER_CAPACITY, DEFAULT_MESSAGE_MAX_LEN},
};
use futures::{io::BufReader, AsyncRead, AsyncReadExt};

/// Async read and parse the next DLT message from the given reader, if any.
///
/// # Cancel safety
/// This function is not cancel safe due to internal buffering.
pub async fn read_message<S: AsyncRead + Unpin>(
    reader: &mut DltStreamReader<S>,
    filter_config_opt: Option<&ProcessedDltFilterConfig>,
) -> Result<Option<ParsedMessage>, DltParseError> {
    let with_storage_header = reader.with_storage_header();
    let slice = reader.next_message_slice().await?;

    if !slice.is_empty() {
        Ok(Some(
            dlt_message(slice, filter_config_opt, with_storage_header)?.1,
        ))
    } else {
        Ok(None)
    }
}

/// Buffered async reader for DLT message slices from a source.
pub struct DltStreamReader<S: AsyncRead + Unpin> {
    source: BufReader<S>,
    with_storage_header: bool,
    buffer: Vec<u8>,
}

impl<S: AsyncRead + Unpin> DltStreamReader<S> {
    /// Create a new reader for the given source.
    pub fn new(source: S, with_storage_header: bool) -> Self {
        DltStreamReader::with_capacity(
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

        DltStreamReader {
            source: BufReader::with_capacity(buffer_capacity, source),
            with_storage_header,
            buffer: vec![0u8; message_max_len],
        }
    }

    /// Async read the next message slice from the source,
    /// or return an empty slice if no more message could be read.
    ///
    /// # Cancel safety
    /// This function is not cancel safe due to internal buffering.
    pub async fn next_message_slice(&mut self) -> Result<&[u8], DltParseError> {
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
            .await
            .is_err()
        {
            return Ok(&[]);
        }

        let (_, message_len) = parse_length(&self.buffer[storage_len..header_len])?;
        let total_len = storage_len + message_len as usize;
        debug_assert!(total_len <= self.buffer.len());

        self.source
            .read_exact(&mut self.buffer[header_len..total_len])
            .await?;

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
    use futures::{stream, TryStreamExt};
    use proptest::prelude::*;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_message_reader() {
        let messages_with_storage = [
            (DLT_MESSAGE, false),
            (DLT_MESSAGE_WITH_STORAGE_HEADER, true),
        ];

        for message_with_storage in &messages_with_storage {
            let bytes = message_with_storage.0;
            let with_storage_header = message_with_storage.1;

            let stream = stream::iter([Ok(bytes)]);
            let mut input = stream.into_async_read();
            let mut reader = DltStreamReader::new(&mut input, with_storage_header);
            assert_eq!(with_storage_header, reader.with_storage_header());

            let slice = reader.next_message_slice().await.expect("message");
            assert_eq!(bytes, slice);

            assert!(reader
                .next_message_slice()
                .await
                .expect("message")
                .is_empty());
        }
    }

    #[tokio::test]
    async fn test_read_message() {
        let messages_with_storage = [
            (DLT_MESSAGE, false),
            (DLT_MESSAGE_WITH_STORAGE_HEADER, true),
        ];

        for message_with_storage in &messages_with_storage {
            let bytes = message_with_storage.0;
            let with_storage_header = message_with_storage.1;

            let stream = stream::iter([Ok(bytes)]);
            let mut input = stream.into_async_read();
            let mut reader = DltStreamReader::new(&mut input, with_storage_header);

            if let Some(ParsedMessage::Item(message)) =
                read_message(&mut reader, None).await.expect("message")
            {
                assert_eq!(bytes, message.as_bytes());
            }

            assert_eq!(
                None,
                read_message(&mut reader, None).await.expect("message")
            )
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
            bytes.push(Ok(message.as_bytes()));
        }

        let stream = stream::iter(bytes);
        let mut input = stream.into_async_read();
        let mut reader = DltStreamReader::new(&mut input, with_storage_header);
        let mut parsed = 0usize;

        Runtime::new().unwrap().block_on(async {
            loop {
                match read_message(&mut reader, None).await.expect("read") {
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
        });

        assert_eq!(messages.len(), parsed);
    }
}
