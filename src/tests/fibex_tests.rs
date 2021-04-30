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
    use crate::fibex::read_fibexes;
    use std::path::PathBuf;
    #[test]
    fn test_fibex_parsing() {
        let fibex = read_fibexes(vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/dlt-messages.xml")
        ])
        .expect("can't parse fibex");
        println!("{:?}", fibex);
    }
}
