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

//! # Provide API to parse dlt-messages from streams
use crate::{
    dlt::{HEADER_MAX_LENGTH, STORAGE_HEADER_LENGTH},
    filtering::ProcessedDltFilterConfig,
    parse::{dlt_message, dlt_standard_header, DltParseError, ParsedMessage},
};
use futures::{AsyncRead, AsyncReadExt};

/// Parse a DLT-message from a stream.
///
/// Note: This is an adapter for [`parse::dlt_message`]
pub async fn dlt_stream<S: AsyncRead + Unpin>(
    stream: &mut S,
    filter_config_opt: Option<&ProcessedDltFilterConfig>,
    with_storage_header: bool,
) -> Result<ParsedMessage, DltParseError> {
    let storage_len = if with_storage_header {
        STORAGE_HEADER_LENGTH as usize
    } else {
        0
    };

    let header_len = storage_len + HEADER_MAX_LENGTH as usize;
    let mut header_buf = vec![0u8; header_len];
    stream.read_exact(&mut header_buf).await?;

    let (_, header) = dlt_standard_header(&header_buf[storage_len..])?;

    let message_len = storage_len + header.overall_length() as usize;
    let mut message_buf = vec![0u8; message_len];
    message_buf[..header_len].copy_from_slice(&header_buf);

    stream.read_exact(&mut message_buf[header_len..]).await?;
    let (rest, message) = dlt_message(&message_buf, filter_config_opt, with_storage_header)?;

    if !rest.is_empty() {
        return Err(DltParseError::Unrecoverable(format!(
            "Incomplete parse ({} bytes remaining)!",
            rest.len()
        )));
    }
    Ok(message)
}
