use std::fs::File;
use std::io::{BufReader, Read};

pub struct JsonLexer<R: Read> {
    reader: R,
}

#[derive(Debug)]
pub enum JsonError {
    InvalidCharacter(char),
    InvalidEscapeSequence(char),

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
    Number(u64),
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
                b't' => {
                    tokens.push(JsonToken::Boolean(Self::consume_literal(
                        &mut byte_iter,
                        b"true",
                    )?));
                }
                b'f' => {
                    tokens.push(JsonToken::Boolean(Self::consume_literal(
                        &mut byte_iter,
                        b"false",
                    )?));
                }

                // strings
                b'"' => tokens.push(JsonToken::String(Self::consume_string(&mut byte_iter)?)),
                _ => tokens.push(JsonToken::Invalid),
            }
        }

        tokens.iter().for_each(|t| match t {
            JsonToken::String(_) => {
                println!("{:?}", t);
            }
            _ => println!("{:?}", t),
        });

        Ok(())
    }

    fn consume_string<I>(byte_iter: &mut I) -> Result<String, JsonError>
    where
        I: Iterator<Item = Result<u8, std::io::Error>>,
    {
        let mut str = String::new();

        while let Some(byte) = byte_iter.next().transpose()? {
            match byte {
                b'"' => return Ok(str),
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

        return Ok(str);
    }

    fn consume_literal<I>(byte_iter: &mut I, expected: &[u8]) -> Result<bool, JsonError>
    where
        I: Iterator<Item = Result<u8, std::io::Error>>,
    {
        let len = expected.len();
        for &expected_byte in &expected[1..len] {
            match byte_iter.next() {
                Some(Ok(byte)) if byte == expected_byte => continue,
                Some(Ok(byte)) => return Err(JsonError::InvalidCharacter(byte as char)),
                Some(Err(e)) => return Err(JsonError::IoError(e)),
                None => return Err(JsonError::UnexpectedEndOfInput),
            }
        }

        println!("expected: {:?}", String::from_utf8(Vec::from(expected)));

        if expected[0] == b't' {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // fn consume_number<I>(byte_iter: &mut I) -> Res {}
}

fn main() -> std::io::Result<()> {
    let f = File::open("test.json")?;

    let buf_reader = BufReader::new(f);

    let lexer = JsonLexer::new(buf_reader);

    lexer.tokenize()?;

    Ok(())
}
