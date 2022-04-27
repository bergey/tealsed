#[macro_use] extern crate lalrpop_util;
lalrpop_mod!(pub posix); // synthesized by LALRPOP

use clap::Parser;
use regex::Regex;
use std::io::{self, Result};
use std::process::exit;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[clap()]
    command: String // s/regex/replacement/
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let stdin = io::stdin();

    let (regex, replacement) = match posix::CommandParser::new().parse(&args.command) {
        Ok((p, r)) => match Regex::new(&p) {
            Ok(regex) => (regex, r),
            Err(err) => {
                println!("error parsing regex: {}", err);
                exit(1);
            }
        },
        Err(err) => {
            println!("error parsing command: {}", err);
            exit(1);
        }
    };
    let mut buf = String::new();
    while stdin.read_line(&mut buf)? != 0 {
        print!("{}", regex.replace(&buf, &replacement));
        buf.clear();
    }
    Ok(())
}
