use dlt_core::stream::{read_message, DltStreamReader};
use std::{env, fs, path::PathBuf, time::Instant};
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;

#[tokio::main]
async fn main() {
    // collect input file details
    let dlt_file_path = PathBuf::from(&env::args().nth(1).expect("no filename given"));
    let dlt_file = File::open(&dlt_file_path).await.expect("open input file");
    let dlt_file_size = fs::metadata(&dlt_file_path).expect("file size error").len();
    // now parse all file content
    let mut dlt_reader = DltStreamReader::new(dlt_file.compat(), true);
    let mut message_count = 0usize;
    let start = Instant::now();
    while let Some(_message) = read_message(&mut dlt_reader, None)
        .await
        .expect("read dlt message")
    {
        message_count += 1;
    }
    // print some stats
    let duration_in_s = start.elapsed().as_millis() as f64 / 1000.0;
    let file_size_in_mb = dlt_file_size as f64 / 1024.0 / 1024.0;
    let amount_per_second: f64 = file_size_in_mb / duration_in_s;
    println!(
        "parsing {} messages took {:.3}s! ({:.3} MB/s)",
        message_count, duration_in_s, amount_per_second
    );
}
