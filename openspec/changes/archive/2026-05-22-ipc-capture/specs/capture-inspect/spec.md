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
- `reqID` as hex
- `dwOffset` as hex and decimal
- Operation type: `READ` or `WRITE` based on high bit of nBytes
- Data size in bytes
- Sentinel status: `✓` if valid, `✗` with byte position if invalid

#### Scenario: Valid record displayed
- **WHEN** a record has a valid sentinel
- **THEN** the tool SHALL show `#1  reqID=0x0001  offset=0x3304  READ  8B  ✓`

#### Scenario: Bad sentinel displayed
- **WHEN** a record has an invalid sentinel
- **THEN** the tool SHALL show `#2  ── BAD SENTINEL @ 0x0C ──`

### Requirement: Gap display after bad sentinel

When a bad sentinel is encountered and the scan finds the next valid record, the tool SHALL display the gap distance and the position of the next record.

#### Scenario: Gap between bad sentinel and next record
- **WHEN** a bad sentinel is at byte 12 and the next valid record is found at byte 35
- **THEN** the tool SHALL show `#2  ── BAD SENTINEL @ 0x0C ──  (scan +23 → next at 0x23)`

### Requirement: End-of-data termination

When a zero reqID is encountered and no further sentinel is found within the scan range, the tool SHALL display a termination indicator and stop.

#### Scenario: Clean termination
- **WHEN** the parser hits a zero reqID with no subsequent sentinel
- **THEN** the tool SHALL show `── END OF DATA ──`

### Requirement: Non-zero exit on errors

The tool SHALL exit with a non-zero status if any capture file contained corrupted records.

#### Scenario: Corrupted file detection
- **WHEN** any record in any file has a bad sentinel
- **THEN** the tool SHALL exit with code 1
- **WHEN** all records in all files have valid sentinels
- **THEN** the tool SHALL exit with code 0
