mod error;

use std::str::FromStr;

use error::Error;

type Result<T, E = error::Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct CompoundExpression {
    expressions: Vec<Expression>,
}

impl FromStr for CompoundExpression {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expressions = parse(s)?;
        Ok(Self { expressions })
    }
}

#[derive(Clone, Debug)]
enum State {
    /// Found non-emitting boundary token bang (!) at idx.
    Bang { idx: usize },

    /// Has encountered no control characters since the previous boundary token.
    Base { idx: usize },

    /// Found control character dice (d).
    Bounded { idx: usize },

    /// Found non-emitting boundary token reroll (r) at idx.
    Reroll { idx: usize },
}

impl State {
    fn idx(&self) -> usize {
        match self {
            State::Bang { idx }
            | State::Base { idx }
            | State::Bounded { idx }
            | State::Reroll { idx } => *idx,
        }
    }

    fn into_bounded(self) -> Self {
        State::Bounded { idx: self.idx() }
    }
}

impl Default for State {
    fn default() -> Self {
        State::Bounded { idx: 0 }
    }
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
                let ExpressionPair { count, value } = parse_expression(expr)?;
                self.count = count;
                self.value = Value::Fixed(value);
            }

            State::Bounded { .. } => {
                let ExpressionPair { count, value } = parse_expression(expr)?;
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

// d20 with advantage + 10 + 2d10 both with disadvantage reroll 2 and under with explosion minus 3
// a20+10+s2d10r2!-3
fn parse(s: &str) -> Result<Vec<Expression>> {
    let mut state = State::Bounded { idx: 0 };
    let mut compound_expression = Vec::new();
    let mut expression = Expression::default();

    for (current_idx, u) in s.bytes().enumerate() {
        match u.to_ascii_lowercase() {
            // Advantage/disadvantage signifier. Either of these is an emitting boundary token.
            u @ b'a' | u @ b's' => {
                expression.apply_state(s, &state, current_idx)?;
                if !expression.is_empty() {
                    compound_expression.push(expression);
                }

                expression = Expression::default();
                expression.advantage = u == b'a';
                expression.disadvantage = u == b's';
                state = State::Base {
                    idx: current_idx + 1,
                };
            }

            b'+' | b'-' => {
                expression.apply_state(s, &state, current_idx)?;
                if !expression.is_empty() {
                    compound_expression.push(expression);
                }

                expression = Expression::default();
                expression.invert = u == b'-';
                state = State::Base {
                    idx: current_idx + 1,
                };
            }

            b'd' => {
                state = state.into_bounded();
            }

            b'r' => {
                expression.apply_state(s, &state, current_idx)?;
                state = State::Reroll {
                    idx: current_idx + 1,
                };
            }

            b'!' => {
                expression.apply_state(s, &state, current_idx)?;
                state = State::Bang {
                    idx: current_idx + 1,
                };
            }

            _ => (),
        }
    }

    expression.apply_state(s, &state, s.len())?;
    compound_expression.push(expression);
    Ok(compound_expression)
}

/// Parses a base dice expression, e.g. 2d6
fn parse_expression(expr: &str) -> Result<ExpressionPair> {
    let mut parts = dbg!(expr).split(|c| c == 'd' || c == 'D');
    let left = parts.next().ok_or_else(|| Error::bad_expression(expr))?;
    let right = parts.next();

    // Expressions must only a maximum of two parts at this level.
    if parts.next().is_some() {
        return Err(Error::bad_expression(expr));
    }

    match right {
        Some(right) => Ok(ExpressionPair {
            count: left.parse().map_err(|e| Error::parse_integer(e, expr))?,
            value: right.parse().map_err(|e| Error::parse_integer(e, expr))?,
        }),
        None => Ok(ExpressionPair {
            count: 1,
            value: left.parse().map_err(|e| Error::parse_integer(e, expr))?,
        }),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        dbg!(super::parse("a20+10+s2d10r2!7-3").unwrap());
        todo!()
    }
}
