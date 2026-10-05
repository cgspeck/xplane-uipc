## ADDED Requirements

### Requirement: CLI reads capture files

The capture-inspect tool SHALL accept a list of `.bin` file paths as command-line arguments and display parsed contents.

#### Scenario: Inspect single file
- **WHEN** invoked as `capture-inspect foo.bin`
- **THEN** the tool SHALL parse and display the records in `foo.bin`

#### Scenario: Inspect multiple files
- **WHEN** invoked as `capture-inspect foo.bin bar.bin`
- **THEN** the tool SHALL parse and display records in each file, separated by file headers

### Requirement: Record-level display

For each record in a capture file, the tool SHALL display:
- Record number (1-based within file)
- Byte offset of the record header in the file, as hex
- Record kind: `READ32`, `READ64` or `WRITE`, from `dwId`
- `dwOffset` as hex
- Data size in bytes
- For reads, `pDest` as hex. For writes, the payload as hex bytes (truncated to 16 bytes)

#### Scenario: Read record displayed
- **WHEN** a file starts with a Read32 record for `0x3304`, 4 bytes, `pDest = 0x010AFFF8`
- **THEN** the tool SHALL show a line like `#1    @0x0000  READ32  offset=0x3304  4B  pDest=0x010afff8`

#### Scenario: Write record displayed
- **WHEN** a file starts with a Write record for `0x8001` with 13 bytes
- **THEN** the tool SHALL show a line like `#1    @0x0000  WRITE   offset=0x8001  13B  36 50 45 54 …`

### Requirement: End-of-data termination

When a zero `dwId` is encountered, the tool SHALL display a termination indicator with the record count and stop. When parsing stops because the data is malformed, the tool SHALL display the reason and the byte offset where it stopped, followed by the record count.

#### Scenario: Clean termination
- **WHEN** the parser hits a zero `dwId`
- **THEN** the tool SHALL show `── END OF DATA ── (N records)`

#### Scenario: Malformed termination
- **WHEN** the parser hits an unknown `dwId` at byte `0x1D`
- **THEN** the tool SHALL show `── MALFORMED: unknown dwId 0xfc000000 at 0x001d ── (N records)`

### Requirement: Non-zero exit on errors

The tool SHALL exit with a non-zero status if any capture file was malformed.

#### Scenario: Corrupted file detection
- **WHEN** any file stops parsing because it is malformed
- **THEN** the tool SHALL exit with code 1
- **WHEN** all files parse to a terminator
- **THEN** the tool SHALL exit with code 0
