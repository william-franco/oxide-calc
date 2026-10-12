use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcError {
    Empty,
    InvalidExpression,
    DivisionByZero,
    Overflow,
    Domain,
    FactorialRange,
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Empty => "Expressão vazia",
            Self::InvalidExpression => "Expressão inválida",
            Self::DivisionByZero => "Divisão por zero",
            Self::Overflow => "Resultado muito grande",
            Self::Domain => "Fora do domínio da função",
            Self::FactorialRange => "Fatorial só para inteiros de 0 a 170",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CalcError {}
