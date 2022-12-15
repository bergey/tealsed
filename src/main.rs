use clap::Parser;
use std::io;

mod commands;
mod regex;
use commands::{Command, parse_command};

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
