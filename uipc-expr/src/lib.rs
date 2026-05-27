use std::collections::HashMap;

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
    IntDiv,
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
    Tern,
}

impl Expr {
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

    pub fn token_strs(&self) -> Vec<String> {
        self.tokens.iter().map(|t| t.to_string()).collect()
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Num(n) => write!(f, "{}", n),
            Token::Var(v) => write!(f, "${}", v),
            Token::Op(op) => write!(f, "{}", op),
        }
    }
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::IntDiv => "\\",
            Op::Mod => "%",
            Op::Pow => "^",
            Op::And => "&",
            Op::Or => "|",
            Op::Eq => "==",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Gt => ">",
            Op::Le => "<=",
            Op::Ge => ">=",
            Op::Abs => "abs",
            Op::Round => "round",
            Op::Tern => "?",
        };
        write!(f, "{}", s)
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let strs: Vec<String> = self.token_strs();
        write!(f, "{}", strs.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, vars: &[(&str, f64)]) -> f64 {
        let map: HashMap<String, f64> = vars.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        Expr::parse(src).unwrap().eval(&map)
    }

    // --- Arithmetic ---

    #[test]
    fn test_add() {
        assert_eq!(eval("3 4 +", &[]), 7.0);
    }

    #[test]
    fn test_sub() {
        assert_eq!(eval("10 3 -", &[]), 7.0);
    }

    #[test]
    fn test_mul() {
        assert_eq!(eval("6 7 *", &[]), 42.0);
        assert_eq!(eval("$IAS 128 *", &[("IAS", 250.0)]), 32000.0);
    }

    #[test]
    fn test_div() {
        assert!((eval("10 3 /", &[]) - 3.3333333333333335).abs() < 1e-12);
    }

    #[test]
    fn test_div_by_near_zero() {
        assert_eq!(eval("5 0 /", &[]), 0.0);
        assert_eq!(eval("5 1e-350 /", &[]), 0.0);
    }

    #[test]
    fn test_intdiv() {
        assert_eq!(eval("123 10 \\", &[("a", 123.0)]), 12.0);
        assert_eq!(eval("20 6 \\", &[]), 3.0);
    }

    #[test]
    fn test_intdiv_by_zero() {
        assert_eq!(eval("5 0 \\", &[]), 0.0);
    }

    #[test]
    fn test_mod() {
        assert_eq!(eval("10 3 %", &[]), 1.0);
        assert_eq!(eval("20 6 %", &[]), 2.0);
    }

    #[test]
    fn test_mod_by_zero() {
        assert_eq!(eval("5 0 %", &[]), 0.0);
    }

    #[test]
    fn test_pow() {
        assert_eq!(eval("2 3 ^", &[]), 8.0);
        assert_eq!(eval("4 0.5 ^", &[]), 2.0);
    }

    #[test]
    fn test_multi_step() {
        assert_eq!(eval("$x 1 + 2 *", &[("x", 3.0)]), 8.0);
    }

    // --- Comparisons ---

    #[test]
    fn test_eq_true() {
        assert_eq!(eval("3 3 ==", &[]), 1.0);
    }

    #[test]
    fn test_eq_false() {
        assert_eq!(eval("3 4 ==", &[]), 0.0);
    }

    #[test]
    fn test_ne_true() {
        assert_eq!(eval("3 4 !=", &[]), 1.0);
    }

    #[test]
    fn test_ne_false() {
        assert_eq!(eval("3 3 !=", &[]), 0.0);
    }

    #[test]
    fn test_lt_true() {
        assert_eq!(eval("3 4 <", &[]), 1.0);
    }

    #[test]
    fn test_lt_false() {
        assert_eq!(eval("4 3 <", &[]), 0.0);
    }

    #[test]
    fn test_gt_true() {
        assert_eq!(eval("4 3 >", &[]), 1.0);
    }

    #[test]
    fn test_gt_false() {
        assert_eq!(eval("3 4 >", &[]), 0.0);
    }

    #[test]
    fn test_le_true() {
        assert_eq!(eval("3 4 <=", &[]), 1.0);
        assert_eq!(eval("4 4 <=", &[]), 1.0);
    }

    #[test]
    fn test_le_false() {
        assert_eq!(eval("4 3 <=", &[]), 0.0);
    }

    #[test]
    fn test_ge_true() {
        assert_eq!(eval("4 3 >=", &[]), 1.0);
        assert_eq!(eval("3 3 >=", &[]), 1.0);
    }

    #[test]
    fn test_ge_false() {
        assert_eq!(eval("3 4 >=", &[]), 0.0);
    }

    // --- Bitwise ---

    #[test]
    fn test_bitwise_and() {
        assert_eq!(eval("7 2 &", &[("a", 7.0)]), 2.0);
        assert_eq!(eval("12 6 &", &[]), 4.0);
    }

    #[test]
    fn test_bitwise_or() {
        assert_eq!(eval("1 2 |", &[]), 3.0);
        assert_eq!(eval("4 2 |", &[]), 6.0);
    }

    // --- Unary ---

    #[test]
    fn test_abs_positive() {
        assert_eq!(eval("5 abs", &[]), 5.0);
    }

    #[test]
    fn test_abs_negative() {
        assert_eq!(eval("-5 abs", &[]), 5.0);
    }

    #[test]
    fn test_abs_zero() {
        assert_eq!(eval("0 abs", &[]), 0.0);
    }

    #[test]
    fn test_round_down() {
        assert_eq!(eval("3.2 round", &[]), 3.0);
    }

    #[test]
    fn test_round_up() {
        assert_eq!(eval("3.7 round", &[]), 4.0);
    }

    #[test]
    fn test_round_negative() {
        assert_eq!(eval("-3.7 round", &[]), -4.0);
    }

    // --- Ternary ---

    #[test]
    fn test_ternary_true_branch() {
        assert_eq!(eval("1 10 20 ?", &[]), 10.0);
    }

    #[test]
    fn test_ternary_false_branch() {
        assert_eq!(eval("0 10 20 ?", &[]), 20.0);
    }

    #[test]
    fn test_ternary_with_var() {
        assert_eq!(eval("$a 0 1 ?", &[("a", 0.0)]), 1.0);
        assert_eq!(eval("$a 5 1 ?", &[("a", 3.0)]), 5.0);
    }

    // --- Constants ---

    #[test]
    fn test_pi() {
        assert_eq!(eval("PI", &[]), std::f64::consts::PI);
    }

    // --- Variables ---

    #[test]
    fn test_var_present() {
        assert_eq!(eval("$x", &[("x", 42.0)]), 42.0);
    }

    #[test]
    fn test_var_missing_defaults_to_zero() {
        assert_eq!(eval("$undefined", &[]), 0.0);
    }

    #[test]
    fn test_var_in_arithmetic() {
        assert_eq!(eval("$x 2 +", &[("x", 5.0)]), 7.0);
    }

    // --- Parse errors ---

    #[test]
    fn test_parse_unknown_token() {
        assert!(Expr::parse("foo").is_err());
    }

    #[test]
    fn test_parse_unknown_token_message() {
        assert_eq!(Expr::parse("foo").unwrap_err(), "unknown token 'foo'");
    }

    // --- Edge cases ---

    #[test]
    fn test_empty_expr() {
        assert_eq!(eval("", &[]), 0.0);
    }

    #[test]
    fn test_single_number() {
        assert_eq!(eval("42", &[]), 42.0);
    }

    #[test]
    fn test_single_var() {
        assert_eq!(eval("$x", &[("x", 10.0)]), 10.0);
    }

    #[test]
    fn test_stack_underflow_binary_op() {
        assert_eq!(eval("5 +", &[]), 5.0);
    }

    #[test]
    fn test_stack_underflow_unary() {
        assert_eq!(eval("abs", &[]), 0.0);
    }

    #[test]
    fn test_stack_underflow_ternary() {
        assert_eq!(eval("?", &[]), 0.0);
    }

    // --- vars() method ---

    #[test]
    fn test_vars_returns_variable_names() {
        let e = Expr::parse("$x $y + $z *").unwrap();
        let mut v = e.vars();
        v.sort();
        assert_eq!(v, vec!["x", "y", "z"]);
    }

    #[test]
    fn test_vars_empty_when_no_vars() {
        let e = Expr::parse("3 4 +").unwrap();
        assert!(e.vars().is_empty());
    }

    #[test]
    fn test_vars_deduplicates() {
        let e = Expr::parse("$x $x +").unwrap();
        assert_eq!(e.vars(), vec!["x", "x"]);
    }

    // --- token_strs() method ---

    #[test]
    fn test_token_strs() {
        let e = Expr::parse("$x 10 +").unwrap();
        assert_eq!(e.token_strs(), vec!["$x", "10", "+"]);
    }

    #[test]
    fn test_token_strs_pi() {
        let e = Expr::parse("PI").unwrap();
        let ts = e.token_strs();
        assert_eq!(ts.len(), 1);
        // PI token displays as its numeric value
        assert!(ts[0].contains("3.14"));
    }

    // --- Display ---

    #[test]
    fn test_display_expr() {
        let e = Expr::parse("$x 1 + 2 *").unwrap();
        assert_eq!(e.to_string(), "$x 1 + 2 *");
    }

    #[test]
    fn test_display_expr_roundtrip() {
        let src = "$IAS 128 *";
        let e = Expr::parse(src).unwrap();
        assert_eq!(e.to_string(), src);
    }
}
