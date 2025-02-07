[![LICENSE](https://img.shields.io/github/license/esrlabs/dlt-core?color=blue)](LICENSE.txt)
[![](https://github.com/esrlabs/dlt-core/workflows/CI/badge.svg)](https://github.com/esrlabs/dlt-core/actions)

# Autosar DLT Support

A library that support efficient parsing & writing log-messages encoded as `Diagnositic` `Log` and `Trace` messages.

## Features / Functionality

* compliant with the official Autosar DLT specification
* efficiently parse binary DLT content
* serialize DLT messages
* support for non-verbose messages via FIBEX file information

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
dlt_core = "0.18"
```

This is an example of how to parse a message and serialize it back to a byte array.

```rust
use dlt_core::dlt_parse::{dlt_message, ParsedMessage};

fn main() {
    let raw1: Vec<u8> = vec![
        // --------------- storage header
        /* DLT + 0x01 */ 0x44, 0x4C, 0x54, 0x01,
        /* timestamp sec */ 0x2B, 0x2C, 0xC9, 0x4D, /* timestamp us */ 0x7A, 0xE8, 0x01,
        0x00, /* ecu id "ECU" */ 0x45, 0x43, 0x55, 0x00,
        // --------------- header
        /* header-type       0b0010 0001 */ 0x21,
        /* extended header        | |||^ */
        /* MSBF: 0  little endian | ||^  */
        /* WEID: 0  no ecu id     | |^   */
        /* WSID: 0  sess id       | ^    */
        /* WTMS: 0 no timestamp   ^      */
        /* version nummber 1   ^^^       */
        /* message counter */
        0x0A, /* length = 0 */ 0x00, 0x13,
        // --------------- extended header
        0x41, // MSIN 0b0100 0001 => verbose, MST log, ApplicationTraceType::State
        0x01, // arg count
        0x4C, 0x4F, 0x47, 0x00, // app id LOG
        0x54, 0x45, 0x53, 0x32, // context id TES2
        // --------------- payload
        /* type info 0b0001 0000 => type bool */ 0x10,
        0x00, 0x00, 0x00, 0x6F,
    ];
    match dlt_message(&raw1[..], None, true) {
        Ok((_rest, ParsedMessage::Item(msg))) => {
            let msg_bytes = msg.as_bytes();
            assert_eq!(raw1, msg_bytes);
        }
        _ => panic!("could not parse message"),
    }
}
```

## Parser in action

The parser is quite fast. Parsing a 4.8 GByte DLT file that contains over 3.5 mio messages took ~12 seconds (~409 MB/sec)

The following example can be run with `cargo run --example file_parser --release sample.dlt`

<!-- example start -->
```rust
use buf_redux::{policy::MinBuffered, BufReader};
use dlt_core::parse::{dlt_message, DltParseError};
use std::{env, fs, fs::File, io::BufRead, path::PathBuf, time::Instant};

const BIN_READER_CAPACITY: usize = 10 * 1024 * 1024;
const BIN_MIN_BUFFER_SPACE: usize = 10 * 1024;

fn main() {
    // collect input file details
    let dlt_file_path = PathBuf::from(&env::args().nth(1).expect("No filename given"));
    let dlt_file = File::open(&dlt_file_path).expect("could not open file");
    let source_file_size = fs::metadata(&dlt_file_path).expect("file size error").len();
    // create a reader that maintains a minimum amount of bytes in it's buffer
    let mut reader = BufReader::with_capacity(BIN_READER_CAPACITY, dlt_file)
        .set_policy(MinBuffered(BIN_MIN_BUFFER_SPACE));
    // now parse all file content
    let mut parsed = 0usize;
    let start = Instant::now();
    loop {
        let consumed: usize = match reader.fill_buf() {
            Ok(content) => {
                if content.is_empty() {
                    println!("empty content after {} parsed messages", parsed);
                    break;
                }
                let available = content.len();

                match dlt_message(content, None, true) {
                    Ok((rest, _maybe_msg)) => {
                        let consumed = available - rest.len();
                        parsed += 1;
                        consumed
                    }
                    Err(DltParseError::IncompleteParse { needed }) => {
                        println!("parse incomplete, needed: {:?}", needed);
                        return;
                    }
                    Err(DltParseError::ParsingHickup(reason)) => {
                        println!("parse error: {}", reason);
                        4 //skip 4 bytes
                    }
                    Err(DltParseError::Unrecoverable(cause)) => {
                        println!("unrecoverable parse failure: {}", cause);
                        return;
                    }
                }
            }
            Err(e) => {
                println!("Error reading: {}", e);
                return;
            }
        };
        reader.consume(consumed);
    }

    // print some stats
    let duration_in_s = start.elapsed().as_millis() as f64 / 1000.0;
    let file_size_in_mb = source_file_size as f64 / 1024.0 / 1024.0;
    let amount_per_second: f64 = file_size_in_mb / duration_in_s;
    println!(
        "parsing {} messages took {:.3}s! ({:.3} MB/s)",
        parsed, duration_in_s, amount_per_second
    );
}

```
<!-- example end -->

```

empty content after 33554430 parsed messages
parsing 33554430 messages took 12.117s! (408.651 MB/s)

```

Below is the revised and improved English version of the documentation:

## Crate's Features

* **`statistics`**: Enables the `statistics` module, which scans the source data and provides a summary of its contents. This gives you an overview of the number of messages and their content.

* **`fibex_parser`**: Enables the `fibex` module, which allows to parse configurations for non-verbose messages from a fibex model.

* **`debug_parser`**: Adds additional log output for debugging purposes.

* **`serde-support`**: Adds `Serialize` and `Deserialize` implementations (via `serde`) to all public types. This feature is useful if you need to encode or decode these types for transmission or storage.

## Example users

### Fast DLT Log Viewing with chipmunk

[**chipmunk**](https://github.com/esrlabs/chipmunk) is a cross-platform log viewer that integrates **dlt-core** for lightning-fast parsing and display of DLT log files. With chipmunk, you can:

- Instantly search and filter log entries  
- Highlight and save specific patterns  
- Efficiently handle large log files without sacrificing performance
- Inspect (export and view) DLT attachments  

If you’re looking for a user-friendly way to work with large DLT logs, give [**chipmunk**](https://github.com/esrlabs/chipmunk) a try!
