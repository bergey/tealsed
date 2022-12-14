// use crate::parser::*;

use clap::Parser;
use std::io;

mod commands;
mod regex;
use commands::Command;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[command()]
    command_or_file: String, // s/regex/replacement/
    #[arg(short='e')]
    commands: Vec<String>,
    #[arg(short='E', help="posix extended regexp syntax")]
    extended_syntax: bool,
    #[arg(short='R', help="rust regexp syntax")]
    rust_syntax: bool,
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
fn parse_command(cmd: &str, syntax: regex::Syntax) -> Result<Command, std::io::Error> {
    let mut chars = cmd.chars();
    match chars.next().unwrap() {
        's' => {
            let sep = chars.next().unwrap();
            let mut words = split_on(&cmd[2..], &sep);
            if words.len() == 2 {
                regex::parse(syntax, &words[0])
                    .map(|regex| Command::S(regex, words.pop().unwrap()))
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidInput, format!(
                    "unexpected number of command arguments: {}",
                    words.len()
                )))
            }
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown command letter".to_string())),
    }
}

fn main() -> io::Result<()> {
    let args = Cli::parse();

    let syntax = match (std::env::args().nth(0), args.rust_syntax, args.extended_syntax) {
        (_, true, false) => regex::Syntax::Rust,
        (_, false, true) => regex::Syntax::PosixExtended,
        (_, true, true) => panic!("must pick one of -R or -E"),
        (Some(cmd), false, false) if cmd == "sed" => regex::Syntax::PosixExtended, // TODO posix basic should be default
        _ => regex::Syntax::Rust,
    };

    let stdin = io::stdin();


    let commands: Vec<Command> = if args.commands.len() == 0 {
        let c = parse_command(&args.command_or_file, syntax)?;
        Vec::from([c])
    } else {
        args.commands.iter().map(|c| parse_command(&c, syntax)).collect::<io::Result<Vec<Command>>>()?
    };

    // keep reusing these buffers
    let mut buf = String::new();
    // swap the roles of these buffers as we make subsequent replacements
    let mut read = String::new();
    let mut write = String::new();
    while stdin.read_line(&mut buf)? != 0 {
        read.clear();
        read.push_str(&buf);
        for c in &commands {
            match c {
                Command::S(regex, replacement) => {
                    // TODO greedy match
                    let changed = regex::replace(regex, &read, &mut write, replacement);
                    if changed {
                        let tmp = read;
                        read = write;
                        write = tmp;
                    }
                }
            }
        }
        print!("{}", read);
        buf.clear();
    }
    Ok(())
}
