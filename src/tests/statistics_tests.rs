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
#[cfg(test)]
mod tests {
    use crate::{
        dlt::LogLevel,
        statistics::common::{LevelDistribution, StatisticInfo},
    };

    fn get_stat_entities() -> Vec<(String, LevelDistribution)> {
        vec![
            (
                String::from("ID_0"),
                LevelDistribution::new(Some(LogLevel::Debug)),
            ),
            (
                String::from("ID_1"),
                LevelDistribution::new(Some(LogLevel::Error)),
            ),
            (
                String::from("ID_2"),
                LevelDistribution::new(Some(LogLevel::Verbose)),
            ),
            (
                String::from("ID_3"),
                LevelDistribution::new(Some(LogLevel::Warn)),
            ),
        ]
    }

    #[test]
    fn test_merging() {
        let mut stat_a = StatisticInfo {
            app_ids: get_stat_entities(),
            context_ids: get_stat_entities(),
            ecu_ids: get_stat_entities(),
            contained_non_verbose: false,
        };
        let stat_b = StatisticInfo {
            app_ids: get_stat_entities(),
            context_ids: get_stat_entities(),
            ecu_ids: get_stat_entities(),
            contained_non_verbose: true,
        };
        assert_eq!(stat_a.app_ids[0].1.log_debug, 1);
        assert_eq!(stat_a.app_ids[1].1.log_error, 1);
        assert_eq!(stat_a.app_ids[2].1.log_verbose, 1);
        assert_eq!(stat_a.app_ids[3].1.log_warning, 1);
        assert_eq!(stat_b.app_ids[0].1.log_debug, 1);
        assert_eq!(stat_b.app_ids[1].1.log_error, 1);
        assert_eq!(stat_b.app_ids[2].1.log_verbose, 1);
        assert_eq!(stat_b.app_ids[3].1.log_warning, 1);
        stat_a.merge(stat_b);
        assert_eq!(stat_a.app_ids[0].0, String::from("ID_0"));
        assert_eq!(stat_a.app_ids[1].0, String::from("ID_1"));
        assert_eq!(stat_a.app_ids[2].0, String::from("ID_2"));
        assert_eq!(stat_a.app_ids[3].0, String::from("ID_3"));
        assert_eq!(stat_a.app_ids[0].1.log_debug, 2);
        assert_eq!(stat_a.app_ids[1].1.log_error, 2);
        assert_eq!(stat_a.app_ids[2].1.log_verbose, 2);
        assert_eq!(stat_a.app_ids[3].1.log_warning, 2);
        assert_eq!(stat_a.context_ids[0].0, String::from("ID_0"));
        assert_eq!(stat_a.context_ids[1].0, String::from("ID_1"));
        assert_eq!(stat_a.context_ids[2].0, String::from("ID_2"));
        assert_eq!(stat_a.context_ids[3].0, String::from("ID_3"));
        assert_eq!(stat_a.context_ids[0].1.log_debug, 2);
        assert_eq!(stat_a.context_ids[1].1.log_error, 2);
        assert_eq!(stat_a.context_ids[2].1.log_verbose, 2);
        assert_eq!(stat_a.context_ids[3].1.log_warning, 2);
        assert_eq!(stat_a.ecu_ids[0].0, String::from("ID_0"));
        assert_eq!(stat_a.ecu_ids[1].0, String::from("ID_1"));
        assert_eq!(stat_a.ecu_ids[2].0, String::from("ID_2"));
        assert_eq!(stat_a.ecu_ids[3].0, String::from("ID_3"));
        assert_eq!(stat_a.ecu_ids[0].1.log_debug, 2);
        assert_eq!(stat_a.ecu_ids[1].1.log_error, 2);
        assert_eq!(stat_a.ecu_ids[2].1.log_verbose, 2);
        assert_eq!(stat_a.ecu_ids[3].1.log_warning, 2);
        assert!(stat_a.contained_non_verbose);
    }
}
