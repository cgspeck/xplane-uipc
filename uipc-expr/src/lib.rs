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
    IMod,
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
    Floor,
    Ceil,
    Neg,
    Sqrt,
    Not,
    Sin,
    Cos,
    Min,
    Max,
    Atan2,
    Dup,
    Swap,
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
                "imod" => Token::Op(Op::IMod),
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
                "floor" => Token::Op(Op::Floor),
                "ceil" => Token::Op(Op::Ceil),
                "neg" => Token::Op(Op::Neg),
                "sqrt" => Token::Op(Op::Sqrt),
                "not" => Token::Op(Op::Not),
                "sin" => Token::Op(Op::Sin),
                "cos" => Token::Op(Op::Cos),
                "min" => Token::Op(Op::Min),
                "max" => Token::Op(Op::Max),
                "atan2" => Token::Op(Op::Atan2),
                "dup" => Token::Op(Op::Dup),
                "swap" => Token::Op(Op::Swap),
                "?" => Token::Op(Op::Tern),
                "PI" => Token::Num(std::f64::consts::PI),
                "E" => Token::Num(std::f64::consts::E),
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
                    Op::Floor => {
                        if let Some(a) = stack.pop() {
                            stack.push(a.floor());
                        }
                    }
                    Op::Ceil => {
                        if let Some(a) = stack.pop() {
                            stack.push(a.ceil());
                        }
                    }
                    Op::Neg => {
                        if let Some(a) = stack.pop() {
                            stack.push(-a);
                        }
                    }
                    Op::Sqrt => {
                        if let Some(a) = stack.pop() {
                            let r = a.sqrt();
                            stack.push(if r.is_nan() { 0.0 } else { r });
                        }
                    }
                    Op::Not => {
                        if let Some(a) = stack.pop() {
                            stack.push(if a == 0.0 { 1.0 } else { 0.0 });
                        }
                    }
                    Op::Sin => {
                        if let Some(a) = stack.pop() {
                            stack.push(a.sin());
                        }
                    }
                    Op::Cos => {
                        if let Some(a) = stack.pop() {
                            stack.push(a.cos());
                        }
                    }
                    Op::Dup => {
                        if let Some(&a) = stack.last() {
                            stack.push(a);
                        }
                    }
                    Op::Swap => {
                        let len = stack.len();
                        if len >= 2 {
                            stack.swap(len - 1, len - 2);
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
                                        a % b
                                    }
                                }
                                Op::IMod => {
                                    if b.abs() < 1e-300 {
                                        0.0
                                    } else {
                                        ((a as i64) % (b as i64)) as f64
                                    }
                                }
                                Op::Pow => a.powf(b),
                                Op::Min => a.min(b),
                                Op::Max => a.max(b),
                                Op::Atan2 => a.atan2(b),
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
                                Op::Abs
                                | Op::Round
                                | Op::Floor
                                | Op::Ceil
                                | Op::Neg
                                | Op::Sqrt
                                | Op::Not
                                | Op::Sin
                                | Op::Cos
                                | Op::Dup
                                | Op::Swap
                                | Op::Tern => {
                                    unreachable!()
                                }
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
            Op::IMod => "imod",
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
            Op::Floor => "floor",
            Op::Ceil => "ceil",
            Op::Neg => "neg",
            Op::Sqrt => "sqrt",
            Op::Not => "not",
            Op::Sin => "sin",
            Op::Cos => "cos",
            Op::Min => "min",
            Op::Max => "max",
            Op::Atan2 => "atan2",
            Op::Dup => "dup",
            Op::Swap => "swap",
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
        assert_eq!(eval("304.75 1 %", &[]), 0.75);
    }

    #[test]
    fn test_mod_negative() {
        assert!((eval("-5.3 1 %", &[]) - -0.3).abs() < 1e-10);
    }

    #[test]
    fn test_mod_by_zero() {
        assert_eq!(eval("5.5 0 %", &[]), 0.0);
    }

    #[test]
    fn test_imod() {
        assert_eq!(eval("304.75 3 imod", &[]), 1.0);
        assert_eq!(eval("10 3 imod", &[]), 1.0);
    }

    #[test]
    fn test_imod_by_zero() {
        assert_eq!(eval("5 0 imod", &[]), 0.0);
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

    // --- Stack manipulation ---

    #[test]
    fn test_dup() {
        assert_eq!(eval("5 dup *", &[]), 25.0);
    }

    #[test]
    fn test_dup_stack_underflow() {
        assert_eq!(eval("dup", &[]), 0.0);
    }

    #[test]
    fn test_swap() {
        assert_eq!(eval("3 5 swap -", &[]), 2.0);
    }

    #[test]
    fn test_swap_stack_underflow() {
        // Only one element — swap is a no-op, value stays
        assert_eq!(eval("5 swap", &[]), 5.0);
    }

    // --- Floor / Ceil ---

    #[test]
    fn test_floor() {
        assert_eq!(eval("3.7 floor", &[]), 3.0);
    }

    #[test]
    fn test_floor_negative() {
        assert_eq!(eval("-3.2 floor", &[]), -4.0);
    }

    #[test]
    fn test_ceil() {
        assert_eq!(eval("3.2 ceil", &[]), 4.0);
    }

    #[test]
    fn test_ceil_negative() {
        assert_eq!(eval("-3.7 ceil", &[]), -3.0);
    }

    // --- Min / Max ---

    #[test]
    fn test_min() {
        assert_eq!(eval("3 5 min", &[]), 3.0);
    }

    #[test]
    fn test_max() {
        assert_eq!(eval("3 5 max", &[]), 5.0);
    }

    #[test]
    fn test_min_equal() {
        assert_eq!(eval("4 4 min", &[]), 4.0);
    }

    // --- Neg ---

    #[test]
    fn test_neg() {
        assert_eq!(eval("5 neg", &[]), -5.0);
    }

    #[test]
    fn test_neg_negative() {
        assert_eq!(eval("-3 neg", &[]), 3.0);
    }

    // --- Sqrt ---

    #[test]
    fn test_sqrt() {
        assert_eq!(eval("9 sqrt", &[]), 3.0);
    }

    #[test]
    fn test_sqrt_negative() {
        assert_eq!(eval("-4 sqrt", &[]), 0.0);
    }

    // --- Not ---

    #[test]
    fn test_not_zero() {
        assert_eq!(eval("0 not", &[]), 1.0);
    }

    #[test]
    fn test_not_nonzero() {
        assert_eq!(eval("5 not", &[]), 0.0);
    }

    #[test]
    fn test_not_negative() {
        assert_eq!(eval("-1 not", &[]), 0.0);
    }

    // --- Trig ---

    #[test]
    fn test_sin_zero() {
        assert_eq!(eval("0 sin", &[]), 0.0);
    }

    #[test]
    fn test_sin_pi_half() {
        assert!((eval("PI 2 / sin", &[]) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_cos_zero() {
        assert_eq!(eval("0 cos", &[]), 1.0);
    }

    #[test]
    fn test_cos_pi() {
        assert!((eval("PI cos", &[]) - (-1.0)).abs() < 1e-12);
    }

    #[test]
    fn test_atan2_basic() {
        assert!((eval("1 1 atan2", &[]) - std::f64::consts::FRAC_PI_4).abs() < 1e-12);
    }

    #[test]
    fn test_atan2_zero() {
        assert_eq!(eval("0 1 atan2", &[]), 0.0);
    }

    // --- E constant ---

    #[test]
    fn test_e() {
        assert!((eval("E", &[]) - std::f64::consts::E).abs() < 1e-12);
    }

    // --- Display roundtrip for new ops ---

    #[test]
    fn test_display_new_ops_roundtrip() {
        for op in [
            "5 dup",
            "3 5 swap",
            "3.7 floor",
            "3.2 ceil",
            "3 5 min",
            "3 5 max",
            "5 neg",
            "9 sqrt",
            "0 not",
            "0 sin",
            "0 cos",
            "1 1 atan2",
        ] {
            let e = Expr::parse(op).unwrap();
            let reparsed = Expr::parse(&e.to_string()).unwrap();
            assert_eq!(
                e.to_string(),
                reparsed.to_string(),
                "roundtrip failed for: {}",
                op
            );
        }
    }

    // --- Integration: UTC offset with midnight crossover ---

    #[test]
    fn test_utc_offset_midnight_crossover() {
        let expr_src =
            "$zh 60 * $zm + $lh 60 * $lm + - dup dup 720 > 1440 0 ? swap -720 < 1440 0 ? swap - +";

        // Normal positive offset (UTC+5:30): zulu 10:00, local 15:30 → -330
        assert_eq!(
            eval(
                expr_src,
                &[("zh", 10.0), ("zm", 0.0), ("lh", 15.0), ("lm", 30.0)]
            ),
            -330.0
        );

        // Normal negative offset (UTC-5): zulu 15:00, local 10:00 → 300
        assert_eq!(
            eval(
                expr_src,
                &[("zh", 15.0), ("zm", 0.0), ("lh", 10.0), ("lm", 0.0)]
            ),
            300.0
        );

        // Midnight crossover (UTC+10): zulu 22:00, local 08:00 next day → -600
        assert_eq!(
            eval(
                expr_src,
                &[("zh", 22.0), ("zm", 0.0), ("lh", 8.0), ("lm", 0.0)]
            ),
            -600.0
        );

        // Midnight crossover negative (UTC-10): zulu 02:00, local 16:00 prev day → 600
        assert_eq!(
            eval(
                expr_src,
                &[("zh", 2.0), ("zm", 0.0), ("lh", 16.0), ("lm", 0.0)]
            ),
            600.0
        );
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
    fn test_token_strs_imod() {
        let src = "$x 3 imod";
        let e = Expr::parse(src).unwrap();
        assert_eq!(e.to_string(), src);
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
