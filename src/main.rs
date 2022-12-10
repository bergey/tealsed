// use crate::parser::*;

use clap::Parser;
use regex::Regex;
use regex_syntax::ast::{Ast};
use std::io;
use std::process::exit;

mod parser;
use parser::{posix};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[clap()]
    command: String, // s/regex/replacement/
}

fn split_on(s: &str, sep: &char) -> Vec<String> {
    let mut ret = Vec::new();
    let mut backslash = false;
    let mut begin = 0;
    for (i, c) in s.char_indices() {
        if c == *sep && !backslash {
            // let mut part = String::new();
            // part.push_str(&s[begin..i]);
            // ret.push(part);
            ret.push(s[begin..i].to_string());
            begin = i + 1;
        }
        backslash = c == '\\';
    }
    ret
}

// TODO better error handling
fn parse_command(cmd: &str) -> Result<(Ast, String), String> {
    let mut chars = cmd.chars();
    match chars.next().unwrap() {
        's' => {
            let sep = chars.next().unwrap();
            let mut words = split_on(&cmd[2..], &sep);
            if words.len() == 2 {
                match posix(&words[0]) {
                    Ok(regex) => Ok((regex, words.pop().unwrap())),
                    Err(err) => Err(format!("error parsing regex: {}", err)),
                }
            } else {
                Err(format!(
                    "unexpected number of command arguments: {}",
                    words.len()
                ))
            }
        }
        _ => Err("unknown command letter".to_string()),
    }
}

fn main() -> io::Result<()> {
    let args = Cli::parse();
    let stdin = io::stdin();

    let (regex, replacement) = match parse_command(&args.command) {
        Ok((regex_ast, replacement)) => {
            (Regex::new(&format!("{}", regex_ast)).unwrap(), replacement)
        }
        Err(err) => {
            eprintln!("{}", err);
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
