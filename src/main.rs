use std::fs::File;
use std::io::BufReader;

use json_lexer::JsonLexer;

fn main() -> std::io::Result<()> {
    let f = File::open("test.json")?;

    let buf_reader = BufReader::new(f);

    let lexer = JsonLexer::new(buf_reader);

    lexer.tokenize()?;

    Ok(())
}
