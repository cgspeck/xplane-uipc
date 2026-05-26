## Context

`OffsetParser.Parse()` strips full-line comments (lines starting with `#`) and blank lines, but leaves `#` characters on non-comment lines intact. When a line like `0x02BC,i32      # comment` is parsed, the type field includes everything after the second comma — `"i32      # comment"` — which doesn't match any known type.

## Goals / Non-Goals

**Goals:**
- Inline `#` comments work on offset definition lines
- The sample file parses without errors

**Non-Goals:**
- No change to the comment/blank-line stripping that already works
- No new CLI features

## Decisions

**1. Strip inline comments before splitting fields**

The cleanest fix: after trimming the line and before skipping blank/comment lines, strip everything from the first `#` onward. This way `#` inside values (not that any exist) are handled, and the existing field-splitting logic is untouched.

```csharp
var commentIdx = line.IndexOf('#');
if (commentIdx >= 0)
    line = line[..commentIdx].TrimEnd();
```

This goes right after `line.Trim()` and before the blank/comment-line check — so `#`-only lines still get caught by the existing empty-line skip.

**2. Existing inline-commented lines in sample file**

The sample file already has `#` inline comments on many lines — those will start working immediately.

## Risks / Trade-offs

- **[Edge case]** A `#` character inside a string or bytes value would be truncated. → Mitigation: offset definitions can't contain `#` in their type or size fields, so this is safe for the supported grammar.
