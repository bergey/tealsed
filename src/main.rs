use clap::Parser;
use std::io;

mod commands;
mod regex;
use commands::{Command, Function, parse_command_finish};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[command()]
    command_or_file: String, // s/regex/replacement/
    #[arg(short='e')]
    commands: Vec<String>,
    #[arg(short='E', help="posix extended regexp syntax (ignored)")]
    extended_syntax: bool,
}

fn main() -> io::Result<()> {
    let args = Cli::parse();

    let stdin = io::stdin();


    let commands: Vec<Command> =
        if args.commands.len() == 0 {
            parse_command_finish(&args.command_or_file)
                .map(|cmd| Vec::from([cmd]))?
        } else {
            args.commands.iter()
                .map(|cmd| parse_command_finish(&cmd))
                .collect::<io::Result<Vec<Command>>>()?
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
            // TODO handle addresses
            match &c.function {
                Function::S(regex, replacement) => {
                    // TODO greedy match
                    let changed = regex::replace(&regex, &read, &mut write, replacement);
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
