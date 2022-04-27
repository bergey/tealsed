use clap::Parser;
use regex::Regex;
use std::io;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[clap()]
    pattern: String
}

fn main() {
    let args = Cli::parse();
    let stdin = io::stdin();

    let regex = Regex::new(&args.pattern).unwrap();
    let mut buf = String::new();
    while stdin.read_line(&mut buf).unwrap() != 0 {
        if regex.is_match(&buf) {
            print!("{}", buf);
        }
        buf.clear();
    }
}
