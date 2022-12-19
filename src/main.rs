use clap::Parser;
use std::io;

mod commands;
mod regex;
use commands::{Command, Function, match_address, parse_command_finish};

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
    let mut line_number = 0;
    // swap the roles of these buffers as we make subsequent replacements
    let mut read = String::new();
    let mut write = String::new();
    let mut in_matching_range = Vec::with_capacity(commands.len());
    for _ in &commands {
        in_matching_range.push(false);
    }
    while stdin.read_line(&mut buf)? != 0 {
        line_number += 1;
        read.clear();
        read.push_str(&buf);
        for (cmd_index, cmd) in commands.iter().enumerate() {
            let should_apply = match (&cmd.start, &cmd.end) {
                (None, None) => true,
                (Some(addr), None) => match_address(&addr, &read, line_number),
                (Some(start), Some(end)) =>
                    if in_matching_range[cmd_index] {
                        let stop = match_address(&end, &read, line_number);
                        in_matching_range[cmd_index] = !stop;
                        true
                    } else {
                        let start = match_address(&start, &read, line_number);
                        in_matching_range[cmd_index] = !start;
                        start
                    },
                (None, Some(end)) => panic!("end address has no matching start {:?}", end)
            };
            if should_apply {
                match &cmd.function {
                    Function::S(regex, replacement) => {
                        // TODO greedy match
                        let changed = regex::replace(&regex, &read, &mut write, replacement);
                        if changed {
                            let tmp = read;
                            read = write;
                            write = tmp;
                            write.clear();
                        }
                    },
                    Function::D => {
                        read.clear();
                    }
                }
            }
        }
        print!("{}", read);
        buf.clear();
    }
    Ok(())
}
