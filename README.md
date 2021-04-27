# Autosar DLT Support

A library that support efficient parsing & writing log-messages encoded as `Diagnositic` `Log` and `Trace` messages.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
dlt_core = "1.0"
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
