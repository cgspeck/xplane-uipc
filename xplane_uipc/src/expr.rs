//! Minimal RPN (Reverse Polish Notation) expression evaluator.
//!
//! Mirrors the expression language used by XPUIPC's CFG files so that
//! the same scaling formulas can be expressed directly in the TOML mapping.
//!
//! Supported tokens (whitespace-separated):
//!   Numbers   – integer or floating point literals (may be negative: -1.5)
//!   PI        – 3.14159…
//!   $name     – read the value of the named operand slot (populated at eval time)
//!   Operators – + - * / \ % ^ abs round
//!               & |  (bitwise, after truncating to i64)
//!               == != < > <= >=  (return 1.0 / 0.0)
//!               ?  (ternary: cond then else – pops three, returns then if cond≠0)
//!
//! Example:  "$IAS 128 *"  →  IAS * 128

use std::collections::HashMap;

/// A compiled (tokenised) expression, ready for repeated evaluation.
#[derive(Debug, Clone)]
pub struct Expr {
    tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Var(String),
    Op(Op),
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv, // backslash
    Mod,
    Pow,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Abs,
    Round,
    Tern, // ?
}

impl Expr {
    /// Parse an RPN expression string into a compiled `Expr`.
    pub fn parse(src: &str) -> Result<Self, String> {
        let mut tokens = Vec::new();
        for raw in src.split_whitespace() {
            let tok = match raw {
                "+" => Token::Op(Op::Add),
                "-" => Token::Op(Op::Sub),
                "*" => Token::Op(Op::Mul),
                "/" => Token::Op(Op::Div),
                "\\" => Token::Op(Op::IntDiv),
                "%" => Token::Op(Op::Mod),
                "^" => Token::Op(Op::Pow),
                "&" => Token::Op(Op::And),
                "|" => Token::Op(Op::Or),
                "==" => Token::Op(Op::Eq),
                "!=" => Token::Op(Op::Ne),
                "<" => Token::Op(Op::Lt),
                ">" => Token::Op(Op::Gt),
                "<=" => Token::Op(Op::Le),
                ">=" => Token::Op(Op::Ge),
                "abs" => Token::Op(Op::Abs),
                "round" => Token::Op(Op::Round),
                "?" => Token::Op(Op::Tern),
                "PI" => Token::Num(std::f64::consts::PI),
                s if s.starts_with('$') => Token::Var(s[1..].to_string()),
                s => s
                    .parse::<f64>()
                    .map(Token::Num)
                    .map_err(|_| format!("unknown token '{}'", s))?,
            };
            tokens.push(tok);
        }
        Ok(Expr { tokens })
    }

    /// Evaluate with a map of variable name → value.
    /// Returns the top of the stack, or 0.0 if the expression is empty.
    pub fn eval(&self, vars: &HashMap<String, f64>) -> f64 {
        let mut stack: Vec<f64> = Vec::with_capacity(16);

        for tok in &self.tokens {
            match tok {
                Token::Num(n) => stack.push(*n),
                Token::Var(name) => {
                    stack.push(*vars.get(name).unwrap_or(&0.0));
                }
                Token::Op(op) => match op {
                    Op::Abs => {
                        if let Some(a) = stack.pop() {
                            stack.push(a.abs());
                        }
                    }
                    Op::Round => {
                        if let Some(a) = stack.pop() {
                            stack.push(a.round());
                        }
                    }
                    Op::Tern => {
                        if stack.len() >= 3 {
                            let els = stack.pop().unwrap();
                            let then = stack.pop().unwrap();
                            let cond = stack.pop().unwrap();
                            stack.push(if cond != 0.0 { then } else { els });
                        }
                    }
                    _ => {
                        if stack.len() >= 2 {
                            let b = stack.pop().unwrap();
                            let a = stack.pop().unwrap();
                            let result = match op {
                                Op::Add => a + b,
                                Op::Sub => a - b,
                                Op::Mul => a * b,
                                Op::Div => {
                                    if b.abs() < 1e-300 {
                                        0.0
                                    } else {
                                        a / b
                                    }
                                }
                                Op::IntDiv => {
                                    if b.abs() < 1e-300 {
                                        0.0
                                    } else {
                                        ((a as i64) / (b as i64)) as f64
                                    }
                                }
                                Op::Mod => {
                                    if b.abs() < 1e-300 {
                                        0.0
                                    } else {
                                        ((a as i64) % (b as i64)) as f64
                                    }
                                }
                                Op::Pow => a.powf(b),
                                Op::And => ((a as i64) & (b as i64)) as f64,
                                Op::Or => ((a as i64) | (b as i64)) as f64,
                                Op::Eq => {
                                    if a == b {
                                        1.0
                                    } else {
                                        0.0
                                    }
                                }
                                Op::Ne => {
                                    if a != b {
                                        1.0
                                    } else {
                                        0.0
                                    }
                                }
                                Op::Lt => {
                                    if a < b {
                                        1.0
                                    } else {
                                        0.0
                                    }
                                }
                                Op::Gt => {
                                    if a > b {
                                        1.0
                                    } else {
                                        0.0
                                    }
                                }
                                Op::Le => {
                                    if a <= b {
                                        1.0
                                    } else {
                                        0.0
                                    }
                                }
                                Op::Ge => {
                                    if a >= b {
                                        1.0
                                    } else {
                                        0.0
                                    }
                                }
                                Op::Abs | Op::Round | Op::Tern => unreachable!(),
                            };
                            stack.push(result);
                        }
                    }
                },
            }
        }

        stack.pop().unwrap_or(0.0)
    }

    /// Return all variable names referenced in this expression.
    pub fn vars(&self) -> Vec<String> {
        self.tokens
            .iter()
            .filter_map(|t| {
                if let Token::Var(n) = t {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, vars: &[(&str, f64)]) -> f64 {
        let map: HashMap<String, f64> = vars.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        Expr::parse(src).unwrap().eval(&map)
    }

    #[test]
    fn test_simple_scale() {
        assert_eq!(eval("$IAS 128 *", &[("IAS", 250.0)]), 32000.0);
    }
    #[test]
    fn test_multi_step() {
        assert_eq!(eval("$x 1 + 2 *", &[("x", 3.0)]), 8.0);
    }
    #[test]
    fn test_ternary() {
        assert_eq!(eval("$a 0 1 ?", &[("a", 0.0)]), 1.0);
    }
    #[test]
    fn test_ternary_true() {
        assert_eq!(eval("$a 5 1 ?", &[("a", 3.0)]), 5.0);
    }
    #[test]
    fn test_bitwise() {
        assert_eq!(eval("$a 2 &", &[("a", 7.0)]), 2.0);
    }
    #[test]
    fn test_intdiv() {
        assert_eq!(eval("$a 10 \\", &[("a", 123.0)]), 12.0);
    }
}
