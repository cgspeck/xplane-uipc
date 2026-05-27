# uipc-expr

A stack-based RPN (Reverse Polish Notation) expression parser and evaluator for the X-Plane UIPC ecosystem.

## Usage

```rust
use std::collections::HashMap;
use uipc_expr::Expr;

let expr = Expr::parse("$IAS 128 *").unwrap();
let mut vars = HashMap::new();
vars.insert("IAS".to_string(), 250.0);
let result = expr.eval(&vars);
assert_eq!(result, 32000.0);
```

## Expression Syntax

Tokens are separated by whitespace.

### Literals

| Token | Description |
|---|---|
| `42` | Any f64 number |
| `PI` | π constant |
| `$name` | Variable reference (fetched from the vars map; missing vars default to 0.0) |

### Operators

#### Arithmetic

| Op | Example | Result |
|---|---|---|
| `+` | `3 4 +` | 7.0 |
| `-` | `10 3 -` | 7.0 |
| `*` | `6 7 *` | 42.0 |
| `/` | `10 3 /` | 3.333... (division by near-zero returns 0.0) |
| `\` | `123 10 \` | 12.0 (integer division; near-zero returns 0.0) |
| `%` | `10 3 %` | 1.0 (modulo on i64 cast; near-zero returns 0.0) |
| `^` | `2 3 ^` | 8.0 (power) |

#### Comparison

All comparisons return `1.0` (true) or `0.0` (false).

| Op | Example | Result |
|---|---|---|
| `==` | `3 3 ==` | 1.0 |
| `!=` | `3 4 !=` | 1.0 |
| `<` | `3 4 <` | 1.0 |
| `>` | `4 3 >` | 1.0 |
| `<=` | `3 4 <=` | 1.0 |
| `>=` | `4 3 >=` | 1.0 |

#### Bitwise

Operands are cast to `i64`, the operation is applied, then the result is cast back to `f64`.

| Op | Example | Result |
|---|---|---|
| `&` | `7 2 &` | 2.0 |
| `\|` | `1 2 \|` | 3.0 |

#### Unary

| Op | Example | Result |
|---|---|---|
| `abs` | `-5 abs` | 5.0 |
| `round` | `3.7 round` | 4.0 |

#### Ternary

| Op | Stack order | Description |
|---|---|---|
| `?` | `cond then else ?` | If `cond != 0.0`, pushes `then`; otherwise pushes `else` |

Example: `$enable 42 0 ?` — returns 42 when `$enable` is non-zero, 0 otherwise.

## API

- `Expr::parse(src)` — Parse a string into an `Expr`.
- `expr.eval(&vars)` — Evaluate the expression with the given variable map.
- `expr.vars()` — Return all variable names referenced in the expression.
- `expr.token_strs()` — Return the tokens as a vector of strings.
- `expr.to_string()` — Reconstruct the expression string from tokens.

## Safety

- Division by near-zero (< 1e-300) silently returns `0.0`.
- Missing variables silently default to `0.0`.
- Stack underflow for operators is silently ignored.
- Empty expressions evaluate to `0.0`.

## License

GNU LGPLv3.
