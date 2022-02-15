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
