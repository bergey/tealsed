use clap::Parser;
use regex::Regex;
use std::io;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[clap()]
    pattern: String,
    /// The replacement text (may include backreferences $1)
    #[clap()]
    replacement: String
}

fn main() {
    let args = Cli::parse();
    let stdin = io::stdin();

    let regex = Regex::new(&args.pattern).unwrap();
    let mut buf = String::new();
    while stdin.read_line(&mut buf).unwrap() != 0 {
        print!("{}", regex.replace(&buf, &args.replacement));
        buf.clear();
    }
}
