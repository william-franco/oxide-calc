use super::error::CalcError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Ln,
    Log,
    Sqrt,
    Cbrt,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    LParen,
    RParen,
    Func(Func),
}

/// Accepts both ASCII operators and the display glyphs used by the UI (`× ÷ − π √ ∛`).
pub fn tokenize(input: &str) -> Result<Vec<Token>, CalcError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let token = match c {
            ' ' | '\t' | '\n' => {
                i += 1;
                continue;
            }
            '0'..='9' | '.' | ',' => {
                let (value, next) = read_number(&chars, i)?;
                i = next;
                tokens.push(Token::Number(value));
                continue;
            }
            '+' => Token::Plus,
            '-' | '−' => Token::Minus,
            '*' | '×' | '·' => Token::Star,
            '/' | '÷' => Token::Slash,
            '^' => Token::Caret,
            '%' => Token::Percent,
            '!' => Token::Bang,
            '(' => Token::LParen,
            ')' => Token::RParen,
            'π' => Token::Number(std::f64::consts::PI),
            '√' => Token::Func(Func::Sqrt),
            '∛' => Token::Func(Func::Cbrt),
            c if c.is_ascii_alphabetic() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect::<String>().to_ascii_lowercase();
                tokens.extend(read_word(&word)?);
                continue;
            }
            _ => return Err(CalcError::InvalidExpression),
        };
        tokens.push(token);
        i += 1;
    }

    Ok(tokens)
}

fn read_number(chars: &[char], start: usize) -> Result<(f64, usize), CalcError> {
    let mut i = start;
    let mut text = String::new();
    let mut seen_dot = false;

    while i < chars.len() {
        match chars[i] {
            d @ '0'..='9' => text.push(d),
            '.' | ',' if !seen_dot => {
                seen_dot = true;
                text.push('.');
            }
            '.' | ',' => return Err(CalcError::InvalidExpression),
            _ => break,
        }
        i += 1;
    }

    // Scientific notation (`1.5e-7`), as produced by the formatter. A bare `e` stays Euler's number.
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        let mut j = i + 1;
        if j < chars.len() && (chars[j] == '+' || chars[j] == '-' || chars[j] == '−') {
            j += 1;
        }
        if j < chars.len() && chars[j].is_ascii_digit() {
            text.push('e');
            if chars[i + 1] == '-' || chars[i + 1] == '−' {
                text.push('-');
            }
            while j < chars.len() && chars[j].is_ascii_digit() {
                text.push(chars[j]);
                j += 1;
            }
            i = j;
        }
    }

    if text == "." {
        return Err(CalcError::InvalidExpression);
    }
    text.parse::<f64>()
        .map(|value| (value, i))
        .map_err(|_| CalcError::InvalidExpression)
}

/// Splits a run of letters into functions and constants, so `2pie` or `sinpi` work.
fn read_word(word: &str) -> Result<Vec<Token>, CalcError> {
    const WORDS: [(&str, Option<Func>); 12] = [
        ("asin", Some(Func::Asin)),
        ("acos", Some(Func::Acos)),
        ("atan", Some(Func::Atan)),
        ("sqrt", Some(Func::Sqrt)),
        ("cbrt", Some(Func::Cbrt)),
        ("sin", Some(Func::Sin)),
        ("cos", Some(Func::Cos)),
        ("tan", Some(Func::Tan)),
        ("log", Some(Func::Log)),
        ("ln", Some(Func::Ln)),
        ("pi", None),
        ("e", None),
    ];

    let mut tokens = Vec::new();
    let mut rest = word;
    'outer: while !rest.is_empty() {
        for (name, func) in WORDS {
            if let Some(tail) = rest.strip_prefix(name) {
                tokens.push(match (name, func) {
                    (_, Some(f)) => Token::Func(f),
                    ("pi", None) => Token::Number(std::f64::consts::PI),
                    _ => Token::Number(std::f64::consts::E),
                });
                rest = tail;
                continue 'outer;
            }
        }
        return Err(CalcError::InvalidExpression);
    }
    Ok(tokens)
}
