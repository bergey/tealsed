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
    command_or_files: Vec<String>, // s/regex/replacement/
    #[arg(short='e')]
    commands: Vec<String>,
    #[arg(short='E', help="posix extended regexp syntax (ignored)")]
    extended_syntax: bool,
    #[arg(short='n', help="do not print every line")]
    no_print: bool,
}

fn run_commands<R>(commands: &[Command], mut input: R, no_print: bool) -> io::Result<()> where R: io::BufRead {
    // keep reusing these buffers
    let mut buf = String::new();
    let mut line_number = 0;
    // swap the roles of these buffers as we make subsequent replacements
    let mut read = String::new();
    let mut write = String::new();
    let mut in_matching_range = Vec::with_capacity(commands.len());
    for _ in commands {
        in_matching_range.push(false);
    }
    while input.read_line(&mut buf)? != 0 {
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
                    Function::D => {
                        read.clear();
                        break;
                    },
                    Function::DD => {
                        if let Some(ix) = read.find('\n') {
                            write.push_str(&read[ix+1..]);
                            let tmp = read;
                            read = write;
                            write = tmp;
                            write.clear();
                        } else {
                            read.clear();
                        }
                    }
                    Function::P => print!("{}", read),
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
                }
            }
        }
        if !no_print { print!("{}", read); }
        buf.clear();
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args = Cli::parse();


    let commands: Vec<Command> =
        if args.commands.len() == 0 {
            match args.command_or_files.first() {
                Some(arg) => parse_command_finish(&arg)
                    .map(|cmd| Vec::from([cmd]))?,
                None => Vec::new()
            }
        } else {
            args.commands.iter()
                .map(|cmd| parse_command_finish(&cmd))
                .collect::<io::Result<Vec<Command>>>()?
        };

    let file_args = if args.commands.len() == 0 {
        &args.command_or_files[1..]
    } else {
        &args.command_or_files
    };

    if file_args.len() == 0 {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        run_commands(&commands, &mut handle, args.no_print)?;
    } else {
        for filename in file_args {
            let file = std::fs::File::open(filename)?;
            let mut buf_reader = io::BufReader::new(file);
            run_commands(&commands, &mut buf_reader, args.no_print)?;
        }
    }

    Ok(())
}
