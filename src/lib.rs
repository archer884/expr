mod error;
mod parser;

use std::{cmp, str::FromStr};

use parser::State;

pub use error::Error;

pub type Result<T, E = error::Error> = std::result::Result<T, E>;

pub trait RngProvider {
    fn next(&mut self, max: i32) -> i32;
}

#[derive(Clone, Debug)]
pub struct CompoundExpression {
    expressions: Vec<Expression>,
}

impl CompoundExpression {
    pub fn realize(&self, provider: &mut impl RngProvider) -> RealizedCompoundExpression {
        let mut sum = 0;
        let realized_expressions = self
            .expressions
            .iter()
            .map(|x| {
                let realized = x.realize(provider);
                sum += realized.realized;
                realized
            })
            .collect();

        RealizedCompoundExpression {
            sum,
            realized_expressions,
        }
    }
}

impl FromStr for CompoundExpression {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expressions = parser::parse(s)?;
        Ok(Self { expressions })
    }
}

#[derive(Clone, Debug)]
pub struct RealizedCompoundExpression {
    sum: i32,
    realized_expressions: Vec<RealizedExpression>,
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

        match state {
            State::Bang { .. } => {
                let threshold = if expr.is_empty() {
                    self.max()
                } else {
                    expr.parse().map_err(|e| Error::parse_integer(e, expr))?
                };
                self.explode = Some(threshold);
            }

            State::Base { .. } => {
                if expr.is_empty() {
                    return Ok(());
                }

                let ExpressionPair { count, value } = parser::parse_expression(expr)?;
                self.count = count;
                self.value = Value::Fixed(value);
            }

            State::Bounded { .. } => {
                if expr.is_empty() {
                    return Ok(());
                }

                let ExpressionPair { count, value } = parser::parse_expression(expr)?;
                self.count = count;
                self.value = Value::Bounded(value);
            }

            State::Reroll { .. } => {
                let threshold = if expr.is_empty() {
                    1
                } else {
                    expr.parse().map_err(|e| Error::parse_integer(e, expr))?
                };
                self.reroll = Some(threshold);
            }
        }

        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn realize(&self, provider: &mut impl RngProvider) -> RealizedExpression {
        let max = match self.value {
            Value::Bounded(max) => max,
            Value::Fixed(n) => return RealizedExpression::fixed(n),
        };

        let mut total = 0;
        for _ in 0..self.count {
            total += self.roll_with_rerolls_and_explosions(max, provider);
        }

        if self.invert {
            RealizedExpression {
                realized: -total,
                max: -(self.count as i32),
                min: -(self.count as i32 * max),
            }
        } else {
            RealizedExpression {
                realized: total,
                max: self.count as i32 * max,
                min: self.count as i32,
            }
        }
    }

    fn roll_with_rerolls_and_explosions(&self, max: i32, provider: &mut impl RngProvider) -> i32 {
        let mut total = 0;
        loop {
            let result = self.apply_advantage_and_disadvantage(max, provider);

            if self.explode.map(|x| result >= x).unwrap_or_default() {
                total += result;
                continue;
            }

            if self.reroll.map(|x| x >= result).unwrap_or_default() {
                continue;
            }

            return total + result;
        }
    }

    fn apply_advantage_and_disadvantage(&self, max: i32, provider: &mut impl RngProvider) -> i32 {
        match (self.advantage, self.disadvantage) {
            (true, false) => {
                let left = provider.next(max);
                let right = provider.next(max);
                cmp::max(left, right)
            }
            (false, true) => {
                let left = provider.next(max);
                let right = provider.next(max);
                cmp::min(left, right)
            }
            _ => provider.next(max),
        }
    }

    fn max(&self) -> i32 {
        match self.value {
            Value::Fixed(n) | Value::Bounded(n) => n,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RealizedExpression {
    min: i32,
    max: i32,
    realized: i32,
}

impl RealizedExpression {
    fn fixed(n: i32) -> Self {
        Self {
            min: n,
            max: n,
            realized: n,
        }
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

#[cfg(test)]
mod tests {
    use crate::{CompoundExpression, RngProvider};

    struct MockRngProvider<I> {
        source: I,
    }

    impl<I> MockRngProvider<I>
    where
        I: Iterator<Item = i32>,
    {
        fn new(source: I) -> Self {
            Self { source }
        }
    }

    impl<I> RngProvider for MockRngProvider<I>
    where
        I: Iterator<Item = i32>,
    {
        fn next(&mut self, _: i32) -> i32 {
            self.source.next().unwrap()
        }
    }

    #[test]
    fn can_reroll() {
        let mut provider = MockRngProvider::new([6, 1, 5].iter().cloned().cycle());
        let expression: CompoundExpression = dbg!("2d6r".parse().unwrap());
        let results = dbg!(expression.realize(&mut provider));
        assert_eq!(11, results.sum);
    }

    #[test]
    fn can_reroll_2() {
        let mut provider = MockRngProvider::new([6, 2, 5].iter().cloned().cycle());
        let expression: CompoundExpression = dbg!("2d6r2".parse().unwrap());
        let results = dbg!(expression.realize(&mut provider));
        assert_eq!(11, results.sum);
    }

    #[test]
    fn can_explode() {
        let mut provider = MockRngProvider::new([6, 2, 5].iter().cloned().cycle());
        let expression: CompoundExpression = dbg!("2d6!".parse().unwrap());
        let results = dbg!(expression.realize(&mut provider));
        assert_eq!(13, results.sum);
    }

    #[test]
    fn can_explode_2() {
        let mut provider = MockRngProvider::new([6, 2, 5, 4].iter().cloned().cycle());
        let expression: CompoundExpression = dbg!("2d6!5".parse().unwrap());
        let results = dbg!(expression.realize(&mut provider));
        assert_eq!(17, results.sum);
    }
}
