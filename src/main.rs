use clap::Parser;
use regex::Regex;
use std::io::{self, Result};
use std::process::exit;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[clap()]
    pattern: String,
    /// The replacement text (may include backreferences $1)
    #[clap()]
    replacement: String,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let stdin = io::stdin();

    let regex = match Regex::new(&args.pattern) {
        Ok(regex) => regex,
        Err(err) => {
            println!("{}", err);
            exit(1);
        }
    };
    let mut buf = String::new();
    while stdin.read_line(&mut buf)? != 0 {
        print!("{}", regex.replace(&buf, &args.replacement));
        buf.clear();
    }
    Ok(())
}
