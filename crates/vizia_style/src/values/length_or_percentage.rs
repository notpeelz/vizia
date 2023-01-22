use crate::{macros::impl_parse, Length, LengthValue, Parse, Percentage};
use cssparser::*;

/// A length or a percentage value.
#[derive(Debug, Clone, PartialEq)]
pub enum LengthOrPercentage {
    Length(Length),
    Percentage(f32),
}

impl Default for LengthOrPercentage {
    fn default() -> Self {
        LengthOrPercentage::Length(Length::default())
    }
}

impl LengthOrPercentage {
    // TODO - Function to return the length in pixels given the necessary input parameters
    // > dpi, font_size, size of 0 char, viewport size, min of bounds
    pub fn to_pixels(&self, min_bounds: f32) -> f32 {
        match self {
            LengthOrPercentage::Length(length) => {
                match length {
                    Length::Value(val) => match val {
                        LengthValue::Px(pixels) => {
                            return *pixels;
                        }

                        _ => {}
                    },

                    // TODO
                    Length::Calc(_l) => {
                        todo!();
                    }
                }
            }

            LengthOrPercentage::Percentage(val) => {
                return val * min_bounds;
            }
        }

        0.0
    }
}

impl_parse! {
    LengthOrPercentage,

    try_parse {
        Length,
        Percentage,
    }
}

impl From<LengthValue> for LengthOrPercentage {
    fn from(value: LengthValue) -> Self {
        LengthOrPercentage::Length(Length::Value(value))
    }
}

impl From<Length> for LengthOrPercentage {
    fn from(length: Length) -> Self {
        LengthOrPercentage::Length(length)
    }
}

impl From<Percentage> for LengthOrPercentage {
    fn from(percentage: Percentage) -> Self {
        LengthOrPercentage::Percentage(percentage.0)
    }
}

impl From<&str> for LengthOrPercentage {
    fn from(s: &str) -> Self {
        let mut input = ParserInput::new(&s);
        let mut parser = Parser::new(&mut input);
        LengthOrPercentage::parse(&mut parser).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::assert_parse;

    assert_parse! {
        LengthOrPercentage, parse_length_percentage,

        percentage {
            LengthOrPercentage::Percentage,
        }

        length {
            LengthOrPercentage::Length,
        }
    }
}
