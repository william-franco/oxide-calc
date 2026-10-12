//! Expression engine: tokenizer, recursive-descent parser and evaluator. No UI dependencies.

mod error;
mod eval;
mod format;
mod lexer;
mod parser;

pub use error::CalcError;
pub use eval::{AngleMode, evaluate};
pub use format::format_number;

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(input: &str) -> Result<f64, CalcError> {
        evaluate(input, AngleMode::Degrees)
    }

    fn rad(input: &str) -> Result<f64, CalcError> {
        evaluate(input, AngleMode::Radians)
    }

    fn approx(input: &str, expected: f64) {
        let value = deg(input).unwrap_or_else(|e| panic!("{input}: {e}"));
        assert!((value - expected).abs() < 1e-9, "{input} = {value}, expected {expected}");
    }

    #[test]
    fn arithmetic_and_precedence() {
        approx("1+2*3", 7.0);
        approx("(1+2)*3", 9.0);
        approx("10-4-3", 3.0);
        approx("100/10/5", 2.0);
        approx("7÷2", 3.5);
        approx("2×3−1", 5.0);
        approx("0.1+0.2", 0.3);
        approx("1,5+1", 2.5);
    }

    #[test]
    fn unary_and_power() {
        approx("-3+5", 2.0);
        approx("−−4", 4.0);
        approx("-2^2", -4.0);
        approx("(-2)^2", 4.0);
        approx("2^3^2", 512.0);
        approx("2^-1", 0.5);
        approx("(-8)^(1/3)", -2.0);
    }

    #[test]
    fn implicit_multiplication() {
        approx("2π", 2.0 * std::f64::consts::PI);
        approx("3(4)", 12.0);
        approx("(1+1)(2+2)", 8.0);
        approx("2e", 2.0 * std::f64::consts::E);
        approx("6÷2(1+2)", 9.0);
    }

    #[test]
    fn percent() {
        approx("50%", 0.5);
        approx("200+10%", 220.0);
        approx("200-10%", 180.0);
        approx("200×10%", 20.0);
    }

    #[test]
    fn trigonometry_degrees_and_radians() {
        approx("sin(30)", 0.5);
        approx("cos(60)", 0.5);
        approx("tan(45)", 1.0);
        approx("sin(180)", 0.0);
        approx("asin(1)", 90.0);
        approx("atan(1)", 45.0);
        let value = rad("sin(π/2)").unwrap();
        assert!((value - 1.0).abs() < 1e-12);
        assert_eq!(rad("sin(π)"), Ok(0.0));
        let value = rad("acos(-1)").unwrap();
        assert!((value - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn logarithms_roots_and_constants() {
        approx("ln(e)", 1.0);
        approx("log(1000)", 3.0);
        approx("√(16)", 4.0);
        approx("sqrt(2)^2", 2.0);
        approx("∛(27)", 3.0);
        approx("∛(-27)", -3.0);
        approx("pi", std::f64::consts::PI);
    }

    #[test]
    fn factorial() {
        approx("0!", 1.0);
        approx("5!", 120.0);
        approx("3!!", 720.0);
        approx("2×3!", 12.0);
        assert!(deg("170!").is_ok());
    }

    #[test]
    fn auto_closes_parentheses() {
        approx("(1+2", 3.0);
        approx("sin(30", 0.5);
        approx("2×(3+(4", 14.0);
    }

    #[test]
    fn scientific_notation_round_trip() {
        approx("1.5e3", 1500.0);
        approx("2e−3", 0.002);
        approx("1e2+1", 101.0);
    }

    #[test]
    fn errors() {
        assert_eq!(deg(""), Err(CalcError::Empty));
        assert_eq!(deg("1/0"), Err(CalcError::DivisionByZero));
        assert_eq!(deg("5÷(2−2)"), Err(CalcError::DivisionByZero));
        assert_eq!(deg("0^-1"), Err(CalcError::DivisionByZero));
        assert_eq!(deg("10^400"), Err(CalcError::Overflow));
        assert_eq!(deg("1e308*10"), Err(CalcError::Overflow));
        assert_eq!(deg("√(-1)"), Err(CalcError::Domain));
        assert_eq!(deg("ln(0)"), Err(CalcError::Domain));
        assert_eq!(deg("log(-5)"), Err(CalcError::Domain));
        assert_eq!(deg("asin(2)"), Err(CalcError::Domain));
        assert_eq!(deg("tan(90)"), Err(CalcError::Domain));
        assert_eq!(deg("tan(-270)"), Err(CalcError::Domain));
        assert_eq!(deg("(-8)^0.5"), Err(CalcError::Domain));
        assert_eq!(deg("171!"), Err(CalcError::FactorialRange));
        assert_eq!(deg("2.5!"), Err(CalcError::FactorialRange));
        assert_eq!(deg("(-1)!"), Err(CalcError::FactorialRange));
        assert_eq!(deg("1+"), Err(CalcError::InvalidExpression));
        assert_eq!(deg("*2"), Err(CalcError::InvalidExpression));
        assert_eq!(deg("1..2"), Err(CalcError::InvalidExpression));
        assert_eq!(deg("2)"), Err(CalcError::InvalidExpression));
        assert_eq!(deg("abc"), Err(CalcError::InvalidExpression));
        assert_eq!(deg("1 $ 2"), Err(CalcError::InvalidExpression));
        assert_eq!(deg("()"), Err(CalcError::InvalidExpression));
    }

    #[test]
    fn deep_nesting_does_not_overflow_the_stack() {
        let input = "(".repeat(10_000) + "1";
        assert_eq!(deg(&input), Err(CalcError::InvalidExpression));
        let input = "-".repeat(10_000) + "1";
        assert_eq!(deg(&input), Err(CalcError::InvalidExpression));
    }

    #[test]
    fn formatting() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(-2.5), "-2.5");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
        assert_eq!(format_number(1.0 / 3.0), "0.333333333333");
        assert_eq!(format_number(123_456_789_012.0), "123456789012");
        assert_eq!(format_number(1e15), "1e15");
        assert_eq!(format_number(2.5e-12), "2.5e-12");
        assert_eq!(format_number(f64::NAN), "—");
    }

    #[test]
    fn formatted_results_are_valid_input() {
        for value in [1e15, -2.5e-12, 1.0 / 3.0, 123.456] {
            let text = format_number(value);
            let parsed = deg(&text).unwrap();
            assert!((parsed - value).abs() <= value.abs() * 1e-11, "{text}");
        }
    }
}
