mod error;
mod parser;

use std::str::FromStr;

use parser::State;

pub use error::Error;

pub type Result<T, E = error::Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct CompoundExpression {
    expressions: Vec<Expression>,
}

impl FromStr for CompoundExpression {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expressions = parser::parse(s)?;
        Ok(Self { expressions })
    }
}

#[derive(Clone, Debug, Default)]
struct Expression {
    count: u32,
    value: Value,
    invert: bool,
    advantage: bool,
    disadvantage: bool,
    reroll: Option<i32>,
    explode: Option<i32>,
}

impl Expression {
    fn apply_state(&mut self, s: &str, state: &State, current_idx: usize) -> Result<()> {
        let expr = &s[state.idx()..current_idx];
        if expr.is_empty() {
            return Ok(());
        }

        match state {
            State::Bang { .. } => {
                let threshold = expr.parse().map_err(|e| Error::parse_integer(e, expr))?;
                self.explode = Some(threshold);
            }

            State::Base { .. } => {
                let ExpressionPair { count, value } = parser::parse_expression(expr)?;
                self.count = count;
                self.value = Value::Fixed(value);
            }

            State::Bounded { .. } => {
                let ExpressionPair { count, value } = parser::parse_expression(expr)?;
                self.count = count;
                self.value = Value::Bounded(value);
            }

            State::Reroll { .. } => {
                let threshold = expr.parse().map_err(|e| Error::parse_integer(e, expr))?;
                self.reroll = Some(threshold);
            }
        }

        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

struct ExpressionPair {
    count: u32,
    value: i32,
}

#[derive(Copy, Clone, Debug)]
enum Value {
    Fixed(i32),
    Bounded(i32),
}

impl Default for Value {
    fn default() -> Self {
        Value::Fixed(0)
    }
}
