use dlt_core::{
    parse::DltParseError,
    read::DltMessageReader,
    statistics::{collect_statistics, Statistic, StatisticCollector},
};
use std::{env, fs, fs::File, path::PathBuf, time::Instant};

pub struct MessageCounter {
    count: usize,
}

impl StatisticCollector for MessageCounter {
    fn collect_statistic(&mut self, _statistic: Statistic) -> Result<(), DltParseError> {
        self.count += 1;
        Ok(())
    }
}

fn main() {
    // collect input file details
    let dlt_file_path = PathBuf::from(&env::args().nth(1).expect("no filename given"));
    let dlt_file = File::open(&dlt_file_path).expect("open input file");
    let dlt_file_size = fs::metadata(&dlt_file_path).expect("file size error").len();
    // now scan all file content
    let mut dlt_reader = DltMessageReader::new(dlt_file, true);
    let mut dlt_collector = MessageCounter { count: 0 };
    let start = Instant::now();
    collect_statistics(&mut dlt_reader, &mut dlt_collector).expect("collect dlt statistics");
    // print some stats
    let duration_in_s = start.elapsed().as_millis() as f64 / 1000.0;
    let file_size_in_mb = dlt_file_size as f64 / 1024.0 / 1024.0;
    let amount_per_second: f64 = file_size_in_mb / duration_in_s;
    println!(
        "parsing {} messages took {:.3}s! ({:.3} MB/s)",
        dlt_collector.count, duration_in_s, amount_per_second
    );
}
