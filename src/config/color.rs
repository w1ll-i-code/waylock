use std::{error, fmt, num, str};
use crate::config::ConfigError;

#[derive(Debug)]
pub enum Error {
    InvalidLength,
    InvalidPrefix,
    ParseInt(num::ParseIntError),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::InvalidLength | Self::InvalidPrefix => None,
            Self::ParseInt(err) => err.source(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => write!(f, "invalid length, color must have exactly 6 digits"),
            Self::InvalidPrefix => write!(f, "invalid color prefix, must start with '#' or '0x'"),
            Self::ParseInt(err) => write!(f, "parse error: {}", err),
        }
    }
}

pub fn from_str(s: &str) -> Result<u32, ConfigError> {
    let digits = None
        .or_else(|| s.strip_prefix("0x"))
        .or_else(|| s.strip_prefix('#'))
        .map_or(Err(ConfigError::Color(Error::InvalidPrefix)), Ok)?;

    if digits.len() != 6 {
        return Err(ConfigError::Color(Error::InvalidLength));
    }

    match u32::from_str_radix(digits, 16) {
        Ok(number) => Ok(number),
        Err(err) => Err(ConfigError::Color(Error::ParseInt(err))),
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::config::ConfigError;

    macro_rules! test {
        ($name: ident: $str: expr, $result: pat) => {
            #[test]
            fn $name() {
                assert!(matches!(super::from_str($str), $result));
            }
        };
    }

    test!(no_prefix_6_digit: "01abEF", Err(ConfigError::Color(Error::InvalidPrefix)));
    test!(binary_prefix_6_digit: "0b01abEF", Err(ConfigError::Color(Error::InvalidPrefix)));
    test!(alphabetic_prefix_6_digit: "a01abEF", Err(ConfigError::Color(Error::InvalidPrefix)));

    test!(octothorpe_6_digit: "#01abEF", Ok(_));
    test!(octothorpe_short: "#01234", Err(ConfigError::Color(Error::InvalidLength)));
    test!(octothorpe_long: "#01234567", Err(ConfigError::Color(Error::InvalidLength)));
    test!(octothorpe_invalid_digit: "#012z45", Err(ConfigError::Color(Error::ParseInt(_))));

    test!(hex_6_digit: "#01abEF", Ok(_));
    test!(hex_short: "#01234", Err(ConfigError::Color(Error::InvalidLength)));
    test!(hex_long: "#01234567", Err(ConfigError::Color(Error::InvalidLength)));
    test!(hex_invalid_digit: "#012z45", Err(ConfigError::Color(Error::ParseInt(_))));
}