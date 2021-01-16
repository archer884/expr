use std::{fmt::Display, num::ParseIntError};

#[derive(Clone, Debug)]
pub enum Error {
    BadExpression { expr: String },
    ParseInteger { error: ParseIntError, expr: String },
}

impl Error {
    pub fn bad_expression(expr: impl Into<String>) -> Self {
        Error::BadExpression { expr: expr.into() }
    }

    pub fn parse_integer(e: ParseIntError, expr: impl Into<String>) -> Self {
        Error::ParseInteger {
            error: e,
            expr: expr.into(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadExpression { expr } => write!(f, "Malformed expression: {}", expr),
            Error::ParseInteger { error, expr } => write!(
                f,
                "The expression {:?} contains an invalid integer: {}",
                expr, error
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Error::ParseInteger { error, .. } = self {
            Some(error)
        } else {
            None
        }
    }
}
