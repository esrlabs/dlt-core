# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.20.2]

- Fixed construction of non-verbose DLT arguments for Frames containing multiple PDU's

## [0.20.1] - 2025-05-09

### Changed

- Added robustness for DLT message reader and stream

## [0.20.0] - 2025-02-25

### Added

- API to read DLT messages from a source
- API to read DLT messages from a stream
- API to collect generic DLT statistics

### Changed

- Removed buf_redux dependency
- Cleanup feature names

## [0.19.2] - 2025-02-06

### Changed

- Updated usage documentation

## [0.19.1] - 2025-02-06

### Changed

- Made examples build and run via cargo

## [0.19.0] - 2025-02-06

### Changed

- Made fibex parsing a separate feature

## [0.18.1] - 2025-01-31

### Changed

- Cleanup license headers (GH-issue #13)
- Remove legacy rakefile (GH-issue #29)

## [0.18.0] - 2024-12-18

### Changed

- Add feature "serde-support", which adds to crate's types Serialize/Deserialize

## [0.17.0] - 2024-10-04

### Changed

- breaking change in API: Remove `nom` crate types from the return types of parsing functions.

## [0.16.0] - 2024-08-16

### Added

- Support parsing of DLT network traces

## [0.15.0] - 2024-05-03

### Changed

- Made dlt-statistics a separate feature

## [0.14.4] - 2023-07-21

### Changed

- Updated quick-xml library and dependencies

## [0.14.2] - 2022-11-02

### Added

- Add support merging statistic info

## [0.14.0] - 2022-02-15

### Added

- Add support for collecting statistics for dlt files

## [0.13.1] - 2022-02-10

### Fixed

- fixed payload length validation (thanks to @eifert)

### Added

- example of how to parse huge file
- some benchmark results

## [0.13.0] - 2022-02-10

### Changed

- breaking change in API: Display implementation removed since it is not part of the core functionality

## [0.12.0] - 2022-01-26

### Changed

- Performance improvements

## [0.11.0] - 2021-12-14

### Added

- Formating timestamp based on timezone

## [0.10.1] - 2021-07-12

### Added

- short feature description

## [0.10.0] - 2021-07-09

### Changed

- Formatting DLT non-verbose messages now shows the binary data rather then a `fibex missing`
  literal

## [0.9.3] - 2021-05-03

### Changed

- Removed pcap-parser library dependency by introducing a feature

## [0.9.2] - 2021-04-30

### Added

- More documentation for API

## [0.9.1] - 2021-04-30

### Changed

- move statistics parsing in its own module

---

## DLT-History ported form github.com/esrlabs/chipmunk

## [0.9.0] - 2021-04-14

### Changed

- Use new nom version
  - dlt parser update
  - removed rust compiler warnings
  - enable criterion html report for benchmarking

## [0.8.4] - 2021-04-13

### Added

- Add counting of dlt messages functionality

## [0.8.3] - 2021-Mar-9

### Fixed

- Fix parsing of invalid length DLT messages
  skip to next message when payload length is invalid
  do not report each warning to client

## [0.8.2] - 2021-Feb-23

### Changed

- Rework ip config structure for dlt-net module

## [0.8.1] - 2021-Feb-18

### Changed

- DLT connector: refactoring in scope of TCP connection

## [0.8.0] - 2021-Feb-15

### Added

- TCP support on DLT connector

## [0.7.9] - 2021-Feb-16

### Changed

- refactor usage of fibex data
  - use tokio tasks instead of threads
  - working with new fibex structure

## [0.7.8] - 2021-Feb-11

### Added

- Multiple multicast support for dlt connector (rabasing)

## [0.7.7] - 2021-Feb-10

### Added

- Support multiple multicast points for DLT connector

## [0.7.6] - 2021-Feb-10

### Changed

- Rework dlt socket stream with tokio codecs

## [0.7.5] - 2021-Feb-2

### Changed

- Filter out messages without extended header
  if any context-id or app-id is filtered

## [0.7.4] - 2020-Nov-10

### Added

- Support conversion from pcap to dlt file

## [0.7.3] - 2020-Oct-16

### Changed

- DLT in pcap improvements
  - Do not produce an error if the message
    in a pcap frame does not contain a DLT message
  - correclty return remaining input from parse
  - discover multiple dlt messages in a pcap frame

## [0.7.2] - 2020-Sep-10

### Fixed

- Fix pcap file reading
- pcap file reading was broken in multiple ways. We now
  try to parse as much as possible and discard corrupt
  messages without stopping

### Added

  A new function was added that allows to convert from a pcapng
  file to a dlt file. There is not yet a binding for javascript
  code.

## [0.7.1] - 2020-May-19

### Changed

    Dismiss deprecated failure library in favor of anyhow
    Rework error-handling: Use anyhow and thiserror instead

## [0.7.0] - 2020-04-11

### Added

- support multiple dlt messages in a udp frame

## [0.6.1] - 2020-Mar-16

### Changed

- add support for dlt statistics for live stream

## [0.6.0] - 2020-Mar-10

### Added

- Add support for enums

## [0.5.1] - 2020-Mar-9

### Fixed

- do not bail when fibex contains unsupported signal ids

## [0.5.0] - 2020-Feb-23

### Added

- implement export sections from dlt file

## [0.4.3] - 2020-Feb-12

### Changed

- more standard conform parsing of dlt arguments
  - bool in particular was assuming either 0x0 or 0x1 but can in fact be
  a different uint8
  - accept reserved string encoding (also valid...we used to assume either
  UTF8 or ASCII)

## [0.4.2] - 2020-Jan-18

### Added

- added support for multiple fibex configuration files

## [0.4.1] - 2020-Jan-16

### Added

- added missing storage header

## [0.4.0] - 2020-Jan-10

### Added

- serializing of dlt messages

### Changed

- improved types of dlt entities

## [0.3.4] - 2019-Dec-10

### Fixed

- do not choke on dlt stats for invalid file

## [0.3.3] - 2019-Dec-5

### Changed

- better dlt log message representation
- also add information about if a file contained non-verbose log messages

## [0.3.2] - 2019-Nov-26

### Fixed

- fixed infinit parsing.
  when parsing invalid dlt files we could get stuck
  exclude non running updater unit test
  added tests for cancellation of async dlt processing

## [0.3.1] - 2019-Nov-15

### Fixed

- Fix optional PDU short-name handling

## [0.3.0] - 2019-Nov-12

### Added

- Add fibex support for nonverbose DLT mode

## [0.2.0] - 2019-Oct-30

### Fixed

- try to not choke on bad DLT messages

### Changed

- less verbose error messages for dlt

### Added

- add support for unknown control types

## [0.1.5] - 2019-Jul-29

### Changed

-more robust error handling for invalid DLT files

## [0.1.4] - 2019-Jul-29

### Fixed

- add missing dlt column markers for some cases

## [0.1.3] - 2019-Jul-27

### Added

- provide log level distribution for statistics

## [0.1.2] - 2019-Jul-24

### Changed

- implemented filtering for app_ids, ecu_ids and context_ids

## [0.1.1] - 2019-Jul-23

### Added

- support filtering with components

## [0.1.0] - 2019-Jul-22

### Changed

- use criterion for benchmarks
