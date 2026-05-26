## ADDED Requirements

### Requirement: Parse offset definitions from text file
The system SHALL read a plain-text file containing FSUIPC offset definitions, one per line.

#### Scenario: Parse fixed-size offsets
- **WHEN** the input file contains `0x02BC,i32`, `0x0D0C,u16`, and `0x3365,u8`
- **THEN** the system SHALL parse three offsets with addresses 0x02BC (i32, 4 bytes), 0x0D0C (u16, 2 bytes), and 0x3365 (u8, 1 byte)

#### Scenario: Parse variable-size types
- **WHEN** the input file contains `0x3160,string,24` and `0x0238,bytes,10`
- **THEN** the system SHALL parse two offsets: address 0x3160 as a 24-byte string and 0x0238 as 10 raw bytes

#### Scenario: Skip blank lines and comments
- **WHEN** the input file contains blank lines and lines starting with `#`
- **THEN** those lines SHALL be ignored

#### Scenario: Reject malformed line
- **WHEN** a line cannot be parsed as `<offset>,<type>[,<size>]`
- **THEN** the system SHALL report an error with the line number and content

### Requirement: Register offsets with FSUIPCConnection
The system SHALL create `FSUIPC.Offset<T>` objects for each parsed offset definition and register them with the FSUIPC client library.

#### Scenario: Register typed offsets
- **WHEN** offsets are parsed from the input file
- **THEN** the system SHALL create typed `Offset<T>` objects (e.g., `Offset<int>`, `Offset<ushort>`, `Offset<byte>`, `Offset<double>`, `Offset<byte[]>`)

#### Scenario: Re-register on reload
- **WHEN** the input file is reloaded
- **THEN** the system SHALL disconnect old offsets and create new ones from the updated file
