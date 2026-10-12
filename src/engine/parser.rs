use super::error::CalcError;
use super::lexer::{Func, Token};

const MAX_DEPTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Neg(Box<Expr>),
    Percent(Box<Expr>),
    Factorial(Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Func, Box<Expr>),
}

/// Recursive-descent parser. Precedence, lowest to highest:
/// `+ -`, `* /` (and implicit multiplication), unary sign, `^` (right-assoc), postfix `! %`.
/// Missing closing parentheses at the end of input are accepted.
pub fn parse(tokens: &[Token]) -> Result<Expr, CalcError> {
    if tokens.is_empty() {
        return Err(CalcError::Empty);
    }
    let mut parser = Parser { tokens, pos: 0, depth: 0 };
    let expr = parser.expression()?;
    if parser.pos != tokens.len() {
        return Err(CalcError::InvalidExpression);
    }
    Ok(expr)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.peek();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn enter(&mut self) -> Result<(), CalcError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            Err(CalcError::InvalidExpression)
        } else {
            Ok(())
        }
    }

    fn expression(&mut self) -> Result<Expr, CalcError> {
        self.enter()?;
        let mut lhs = self.term()?;
        while let Some(token @ (Token::Plus | Token::Minus)) = self.peek() {
            self.advance();
            let rhs = self.term()?;
            let op = if token == Token::Plus { BinOp::Add } else { BinOp::Sub };
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        self.depth -= 1;
        Ok(lhs)
    }

    fn term(&mut self) -> Result<Expr, CalcError> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    BinOp::Mul
                }
                Some(Token::Slash) => {
                    self.advance();
                    BinOp::Div
                }
                Some(Token::Number(_) | Token::Func(_) | Token::LParen) => BinOp::Mul,
                _ => break,
            };
            let rhs = self.unary()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, CalcError> {
        self.enter()?;
        let expr = match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                Expr::Neg(Box::new(self.unary()?))
            }
            Some(Token::Plus) => {
                self.advance();
                self.unary()?
            }
            _ => self.power()?,
        };
        self.depth -= 1;
        Ok(expr)
    }

    fn power(&mut self) -> Result<Expr, CalcError> {
        let base = self.postfix()?;
        if self.peek() == Some(Token::Caret) {
            self.advance();
            let exponent = self.unary()?;
            return Ok(Expr::Binary(BinOp::Pow, Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<Expr, CalcError> {
        let mut expr = self.primary()?;
        loop {
            match self.peek() {
                Some(Token::Bang) => expr = Expr::Factorial(Box::new(expr)),
                Some(Token::Percent) => expr = Expr::Percent(Box::new(expr)),
                _ => break,
            }
            self.advance();
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, CalcError> {
        self.enter()?;
        let expr = match self.advance() {
            Some(Token::Number(value)) => Expr::Num(value),
            Some(Token::LParen) => {
                let inner = self.expression()?;
                match self.peek() {
                    Some(Token::RParen) => {
                        self.advance();
                    }
                    None => {}
                    _ => return Err(CalcError::InvalidExpression),
                }
                inner
            }
            // `sin(x)^2` applies `^2` to the call; `sin x` takes a unary operand.
            Some(Token::Func(func)) => {
                let arg = if self.peek() == Some(Token::LParen) { self.primary()? } else { self.unary()? };
                Expr::Call(func, Box::new(arg))
            }
            None => return Err(CalcError::InvalidExpression),
            Some(_) => return Err(CalcError::InvalidExpression),
        };
        self.depth -= 1;
        Ok(expr)
    }
}
