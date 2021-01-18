mod error;

use error::Error;
use regex::Regex;

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct ExpressionParser {
    bounded_expression: Regex,
    modifier_expression: Regex,
    reroll_expression: Regex,
    explode_expression: Regex,
}

impl ExpressionParser {
    pub fn new() -> Self {
        ExpressionParser {
            bounded_expression: Regex::new(r#"^([Aa]|[Ss])?(\d+d)?(\d+)"#).unwrap(),
            modifier_expression: Regex::new(r#"([+-]\d+)"#).unwrap(),
            reroll_expression: Regex::new(r#"r(\d+)?"#).unwrap(),
            explode_expression: Regex::new(r#"!(\d+)?"#).unwrap(),
        }
    }

    pub fn parse(&self, expr: &str) -> Result<Expression> {
        let mut expression = Expression::default();

        match self.bounded_expression.captures(expr) {
            Some(captures) => {
                if let Some(group) = captures.get(1) {
                    expression.advantage = Some(
                        if group.as_str().as_bytes()[0].to_ascii_lowercase() == b'a' {
                            Advantage::Advantage
                        } else {
                            Advantage::Disadvantage
                        },
                    );
                }

                if let Some(group) = captures.get(2) {
                    let subexpr = group.as_str();
                    let subexpr = &subexpr[..subexpr.len() - 1];
                    expression.count = subexpr
                        .parse()
                        .map_err(|e| Error::BadInteger(subexpr.into(), e))?;
                } else {
                    expression.count = 1;
                }

                let max = captures
                    .get(3)
                    .ok_or_else(|| Error::BadExpression(expr.into()))?
                    .as_str();
                expression.max = max.parse().map_err(|e| Error::BadInteger(max.into(), e))?;
            }
            None => return Err(Error::BadExpression(expr.into())),
        }

        if let Some(text) = self.modifier_expression.find(expr) {
            expression.modifier = text
                .as_str()
                .parse()
                .map_err(|e| Error::BadInteger(text.as_str().into(), e))?;
        }

        expression.reroll = parse_threshold_token(expr, &self.reroll_expression, 1)?.map(Reroll);
        expression.explode =
            parse_threshold_token(expr, &self.explode_expression, expression.max)?.map(Explode);

        Ok(expression)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Expression {
    count: i32,
    max: i32,
    modifier: i32,
    advantage: Option<Advantage>,
    reroll: Option<Reroll>,
    explode: Option<Explode>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Advantage {
    Advantage,
    Disadvantage,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Reroll(i32);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Explode(i32);

fn parse_threshold_token(expr: &str, pattern: &Regex, default: i32) -> Result<Option<i32>> {
    match pattern.captures(expr) {
        Some(captures) => match captures.get(1).map(|x| x.as_str()) {
            Some(text) => Ok(Some(
                text.parse()
                    .map_err(|e| Error::BadInteger(text.into(), e))?,
            )),
            None => Ok(Some(default)),
        },
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Advantage, Explode, Expression, ExpressionParser, Reroll};

    #[test]
    fn bounded_expression() {
        let expression = parse("2d6");
        assert_eq!(count_max(2, 6), expression);
    }

    #[test]
    fn leading_bounded_expression() {
        let expression = parse("20");
        assert_eq!(count_max(1, 20), expression);
    }

    #[test]
    fn bounded_expression_with_reroll() {
        let actual = parse("2d6r");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(1)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_reroll_2() {
        let actual = parse("2d6r2");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(2)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_explode() {
        let actual = parse("2d6!");
        let expected = Expression {
            count: 2,
            max: 6,
            explode: Some(Explode(6)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_explode_5() {
        let actual = parse("2d6!5");
        let expected = Expression {
            count: 2,
            max: 6,
            explode: Some(Explode(5)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_reroll_and_explode() {
        let actual = parse("2d6r!");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(1)),
            explode: Some(Explode(6)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_reroll_and_explode_non_default_thresholds() {
        let actual = parse("2d6r2!5");
        let expected = Expression {
            count: 2,
            max: 6,
            reroll: Some(Reroll(2)),
            explode: Some(Explode(5)),
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn bounded_expression_with_advantage() {
        let a = parse("a20");
        let b = parse("a1d20");

        let expected = Expression {
            count: 1,
            max: 20,
            advantage: Some(Advantage::Advantage),
            ..Default::default()
        };

        assert_eq!(a, expected);
        assert_eq!(b, expected);
    }

    #[test]
    fn bounded_expression_with_disadvantage() {
        let a = parse("s20");
        let b = parse("s1d20");

        let expected = Expression {
            count: 1,
            max: 20,
            advantage: Some(Advantage::Disadvantage),
            ..Default::default()
        };

        assert_eq!(a, expected);
        assert_eq!(b, expected);
    }

    fn parse(s: &str) -> Expression {
        ExpressionParser::new().parse(s).unwrap()
    }

    fn count_max(count: i32, max: i32) -> Expression {
        Expression {
            count,
            max,
            ..Default::default()
        }
    }
}
