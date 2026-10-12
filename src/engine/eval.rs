use super::error::CalcError;
use super::lexer::{Func, tokenize};
use super::parser::{BinOp, Expr, parse};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AngleMode {
    #[default]
    Degrees,
    Radians,
}

const MAX_FACTORIAL: f64 = 170.0;
const SNAP_EPSILON: f64 = 1e-14;

pub fn evaluate(input: &str, angle: AngleMode) -> Result<f64, CalcError> {
    let tokens = tokenize(input)?;
    let ast = parse(&tokens)?;
    let value = eval(&ast, angle)?;
    // Avoid displaying "-0".
    Ok(if value == 0.0 { 0.0 } else { value })
}

fn eval(expr: &Expr, angle: AngleMode) -> Result<f64, CalcError> {
    let value = match expr {
        Expr::Num(value) => *value,
        Expr::Neg(inner) => -eval(inner, angle)?,
        Expr::Percent(inner) => eval(inner, angle)? / 100.0,
        Expr::Factorial(inner) => factorial(eval(inner, angle)?)?,
        Expr::Call(func, arg) => call(*func, eval(arg, angle)?, angle)?,
        // Calculator-style percentage: `a + b%` means `a + a·b/100`.
        Expr::Binary(op @ (BinOp::Add | BinOp::Sub), lhs, rhs) if matches!(**rhs, Expr::Percent(_)) => {
            let base = eval(lhs, angle)?;
            let delta = base * eval(rhs, angle)?;
            if *op == BinOp::Add { base + delta } else { base - delta }
        }
        Expr::Binary(op, lhs, rhs) => {
            let a = eval(lhs, angle)?;
            let b = eval(rhs, angle)?;
            match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => {
                    if b == 0.0 {
                        return Err(CalcError::DivisionByZero);
                    }
                    a / b
                }
                BinOp::Pow => power(a, b)?,
            }
        }
    };
    check(value)
}

fn check(value: f64) -> Result<f64, CalcError> {
    if value.is_nan() {
        Err(CalcError::Domain)
    } else if value.is_infinite() {
        Err(CalcError::Overflow)
    } else {
        Ok(value)
    }
}

fn power(base: f64, exponent: f64) -> Result<f64, CalcError> {
    if base == 0.0 && exponent < 0.0 {
        return Err(CalcError::DivisionByZero);
    }
    // Real odd roots of negative numbers, e.g. (-8)^(1/3) = -2.
    if base < 0.0 && exponent.fract() != 0.0 {
        let inverse = 1.0 / exponent;
        if (inverse - inverse.round()).abs() < 1e-9 && (inverse.round() as i64) % 2 != 0 {
            return check(-(-base).powf(exponent));
        }
        return Err(CalcError::Domain);
    }
    check(base.powf(exponent))
}

fn factorial(n: f64) -> Result<f64, CalcError> {
    if n.fract() != 0.0 || !(0.0..=MAX_FACTORIAL).contains(&n) {
        return Err(CalcError::FactorialRange);
    }
    Ok((2..=n as u32).fold(1.0, |acc, k| acc * f64::from(k)))
}

fn call(func: Func, x: f64, angle: AngleMode) -> Result<f64, CalcError> {
    let to_rad = |v: f64| match angle {
        AngleMode::Degrees => v.to_radians(),
        AngleMode::Radians => v,
    };
    let from_rad = |v: f64| match angle {
        AngleMode::Degrees => v.to_degrees(),
        AngleMode::Radians => v,
    };

    let value = match func {
        Func::Sin => snap(to_rad(x).sin()),
        Func::Cos => snap(to_rad(x).cos()),
        Func::Tan => {
            if angle == AngleMode::Degrees && (x - 90.0).rem_euclid(180.0) == 0.0 {
                return Err(CalcError::Domain);
            }
            let rad = to_rad(x);
            if snap(rad.cos()) == 0.0 {
                return Err(CalcError::Domain);
            }
            snap(rad.tan())
        }
        Func::Asin | Func::Acos if !(-1.0..=1.0).contains(&x) => return Err(CalcError::Domain),
        Func::Asin => from_rad(x.asin()),
        Func::Acos => from_rad(x.acos()),
        Func::Atan => from_rad(x.atan()),
        Func::Ln | Func::Log if x <= 0.0 => return Err(CalcError::Domain),
        Func::Ln => x.ln(),
        Func::Log => x.log10(),
        Func::Sqrt if x < 0.0 => return Err(CalcError::Domain),
        Func::Sqrt => x.sqrt(),
        Func::Cbrt => x.cbrt(),
    };
    check(value)
}

/// Removes floating-point noise such as `sin(π) = 1.2e-16`.
fn snap(value: f64) -> f64 {
    if value.abs() < SNAP_EPSILON {
        0.0
    } else if (value.abs() - 1.0).abs() < SNAP_EPSILON {
        value.signum()
    } else {
        value
    }
}
