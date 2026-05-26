## 1. Fix inline comment parsing

- [x] 1.1 Add `#` stripping logic in `OffsetParser.Parse()` before field splitting
- [x] 1.2 Verify `sample-offsets.txt` parses without errors
- [x] 1.3 Add parser unit test for inline comments

## 2. Verify

- [x] 2.1 Run existing tests — no regressions
- [x] 2.2 Build and confirm 0 warnings
