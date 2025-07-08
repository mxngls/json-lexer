use std::io::Read;
use std::iter::Peekable;

pub struct JsonLexer<R: Read> {
    reader: R,
}

#[derive(Debug)]
pub enum JsonError {
    InvalidCharacter(char),
    InvalidEscapeSequence(char),
    InvalidNumber(f64),

    IoError(std::io::Error),

    UnexpectedEndOfInput,
}

impl From<std::io::Error> for JsonError {
    fn from(err: std::io::Error) -> Self {
        JsonError::IoError(err)
    }
}

impl From<JsonError> for std::io::Error {
    fn from(err: JsonError) -> Self {
        match err {
            JsonError::InvalidCharacter(c) => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid character: '{}'", c),
            ),
            JsonError::InvalidEscapeSequence(c) => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid escape sequence: \\{}", c),
            ),
            JsonError::InvalidNumber(n) => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid number: {}", n),
            ),
            JsonError::IoError(io_err) => io_err,
            JsonError::UnexpectedEndOfInput => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unexpected end of input"),
            ),
        }
    }
}

#[derive(Debug)]
enum JsonToken {
    Colon,
    Comma,
    Invalid,
    ObjectEnd,
    ObjectStart,
    String(String),
    Boolean(bool),
    Number(f64),
}

impl<R: Read> JsonLexer<R> {
    pub fn new(reader: R) -> Self {
        JsonLexer { reader }
    }

    pub fn tokenize(self) -> Result<(), JsonError> {
        let mut tokens = Vec::new();
        let mut byte_iter = self.reader.bytes().peekable();

        while let Some(byte) = byte_iter.next().transpose()? {
            match byte {
                // skip whitespace
                b' ' | b'\t' | b'\n' | b'\r' => continue,

                b'{' => tokens.push(JsonToken::ObjectStart),
                b'}' => tokens.push(JsonToken::ObjectEnd),

                b':' => tokens.push(JsonToken::Colon),
                b',' => tokens.push(JsonToken::Comma),

                // booleans
                b'f' | b't' => {
                    tokens.push(Self::consume_literal(&mut byte_iter, &byte)?);
                }

                // numbers
                b'0'..=b'9' | b'+' | b'-' => {
                    tokens.push(Self::consume_number(&mut byte_iter, &byte)?);
                }

                // strings
                b'"' => tokens.push(Self::consume_string(&mut byte_iter)?),

                // invalid or not yet implemented
                _ => tokens.push(JsonToken::Invalid),
            }
        }

        tokens.iter().for_each(|t| {
            print!("[");
            match t {
                JsonToken::String(s) => print!("String: \"{}\"", s),
                JsonToken::Number(n) => print!("Number: {}", n),
                JsonToken::Boolean(b) => print!("Boolean: {}", b),
                _ => print!("{:?}", t),
            };
            print!("], ");
        });

        println!();

        Ok(())
    }

    fn consume_string<I>(byte_iter: &mut I) -> Result<JsonToken, JsonError>
    where
        I: Iterator<Item = Result<u8, std::io::Error>>,
    {
        let mut str = String::new();

        while let Some(byte) = byte_iter.next().transpose()? {
            match byte {
                b'"' => return Ok(JsonToken::String(str)),
                b'\\' => {
                    if let Some(escaped) = byte_iter.next().transpose()? {
                        match escaped {
                            b'"' => str.push('"'),
                            b'\\' => str.push('\\'),
                            b'b' => str.push('\u{08}'),
                            b'f' => str.push('\u{0C}'),
                            b'n' => str.push('n'),
                            b'r' => str.push('r'),
                            b't' => str.push('t'),
                            b'u' => todo!("Implement parsing of unicode expace sequencees"),
                            _ => return Err(JsonError::InvalidEscapeSequence(escaped as char)),
                        };
                    } else {
                        return Err(JsonError::UnexpectedEndOfInput);
                    }
                }
                _ => {}
            };
            str.push(byte as char);
        }

        return Ok(JsonToken::String(str));
    }

    fn consume_literal<I>(byte_iter: &mut I, byte: &u8) -> Result<JsonToken, JsonError>
    where
        I: Iterator<Item = Result<u8, std::io::Error>>,
    {
        let expected: &[u8] = if *byte == b'f' { b"false" } else { b"true" };
        let len = expected.len();
        for &expected_byte in &expected[1..len] {
            match byte_iter.next() {
                Some(Ok(byte)) if byte == expected_byte => continue,
                Some(Ok(byte)) => return Err(JsonError::InvalidCharacter(byte as char)),
                Some(Err(e)) => return Err(JsonError::IoError(e)),
                None => return Err(JsonError::UnexpectedEndOfInput),
            }
        }

        if expected[0] == b't' {
            Ok(JsonToken::Boolean(true))
        } else {
            Ok(JsonToken::Boolean(false))
        }
    }

    fn consume_number<I>(byte_iter: &mut Peekable<I>, byte: &u8) -> Result<JsonToken, JsonError>
    where
        I: Iterator<Item = Result<u8, std::io::Error>>,
    {
        let mut is_negative = false;
        let mut is_fraction = false;
        let mut n: f64 = 0.0;
        let mut nfraction = 1;

        // possible signs
        match byte {
            b'+' => (),
            b'-' => is_negative = true,
            b'0'..=b'9' => {
                n = (*byte - b'0') as f64;
            }
            _ => return Err(JsonError::InvalidNumber(0.0)),
        };

        while let Some(Ok(peeked)) = byte_iter.peek() {
            match peeked {
                // integer part
                b'0'..=b'9' if !is_fraction => {
                    let Some(byte) = byte_iter.next().transpose()? else {
                        return Err(JsonError::UnexpectedEndOfInput);
                    };
                    let digit = (byte - b'0') as f64;
                    n = n * 10.0 + digit;

                    if n > f64::MAX / 10.0 {
                        return Err(JsonError::InvalidNumber(n));
                    }
                }
                // decimal part
                b'0'..=b'9' => {
                    let Some(byte) = byte_iter.next().transpose()? else {
                        return Err(JsonError::UnexpectedEndOfInput);
                    };
                    let digit = (byte - b'0') as f64;
                    n = n + (digit / 10_f64.powi(nfraction));
                    nfraction += 1;
                }
                // decimal point
                b'.' => {
                    if is_fraction {
                        return Err(JsonError::InvalidNumber(n));
                    }
                    is_fraction = true;
                    byte_iter.next().transpose()?;
                }
                _ => {
                    break;
                }
            }
        }

        if is_negative {
            n = -n;
        }

        return Ok(JsonToken::Number(n));
    }
}
