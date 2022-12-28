use clap::Parser;
use std::io;
use std::io::{BufRead, Write};

mod commands;
mod regex;
use commands::{Command, Function, match_address, parse_command_finish};
use crate::regex::parser::{Syntax, new_regex_input};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The pattern to find
    #[command()]
    command_or_files: Vec<String>, // s/regex/replacement/
    #[arg(short='e', long="expression")]
    commands: Vec<String>,
    #[arg(short='E', long="regexp-extended", help="posix extended regexp syntax")]
    extended_syntax: bool,
    #[arg(short='T', help="tealsed regexp syntax; default if invoked as tsed")]
    teal_syntax: bool,
    #[arg(long, help="accept some GNU extensions")]
    gnu: bool,
    #[arg(short='n', long="quiet", help="do not print every line")]
    no_print: bool,
    #[arg(long, help="print intermediate results")]
    debug: bool,
}

fn run_commands<R>(commands: &[Command], input: R, output: &mut dyn Write, no_print: bool) -> io::Result<()>
where R: Iterator<Item = io::Result<String>> {
    // input buffer, reused for each line
    let mut buf = String::new();
    let mut line_number = 0;

    // swap the roles of these buffers as we make subsequent replacements
    let mut read = String::new();
    let mut write = String::new();

    let mut hold = String::new();

    // for each command, a boolean to track whether we are within its address range
    let mut in_matching_range = Vec::with_capacity(commands.len());
    for _ in commands {
        in_matching_range.push(false);
    }

    for r_line in input {
        let line = r_line?;
        line_number += 1;
        read.clear();
        read.push_str(&line);

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
                use Function::{*};

                match &cmd.function {
                    Fd => {
                        read.clear();
                        break;
                    },
                    D => {
                        if let Some(ix) = read.find('\n') {
                            write.push_str(&read[ix+1..]);
                            let tmp = read;
                            read = write;
                            write = tmp;
                            write.clear();
                        } else {
                            read.clear();
                        }
                    },
                    Fg => {
                        read.clear();
                        read.push_str(&hold);
                    },
                    G => {
                        read.push_str("\n");
                        read.push_str(&hold);
                    },
                    Fh => {
                        hold.clear();
                        hold.push_str(&read);
                    },
                    H => {
                        hold.push_str("\n");
                        hold.push_str(&read);
                    },
                    Fi(text) => writeln!(output, "{}", text).unwrap(),
                    Fp => writeln!(output, "{}", read).unwrap(),
                    Fs(regex, replacement) => {
                        // TODO greedy match
                        let changed = regex::replace(&regex, &read, &mut write, replacement);
                        if changed {
                            let tmp = read;
                            read = write;
                            write = tmp;
                            write.clear();
                        }
                    },
                    Fx => {
                        let tmp = read;
                        read = hold;
                        hold = tmp;
                    }
                }

            }
        }
        if !no_print { writeln!(output, "{}", read).unwrap(); }
        buf.clear();
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args = Cli::parse();

    // one syntax for all regexen in all commands
    let syntax = match (args.extended_syntax, args.teal_syntax) {
        (true, false) => Syntax::Extended,
        (false, true) => Syntax::Teal,
        (true, true) => panic!("-E and -T are incompatible; pick one"),
        (false, false) => 
        {
            let invoked_as = std::env::current_exe()?;
            let name = invoked_as.file_name().expect("could not discover own name");
            if name == "tsed" {
                Syntax::Teal
            } else {
                Syntax::Basic
            }
        }
    };

    let commands: Vec<Command> =
        if args.commands.len() == 0 {
            match args.command_or_files.first() {
                Some(arg) => {
                    let mut s = new_regex_input(&arg);
                    s.extra.syntax = syntax;
                    parse_command_finish(s)
                        .map(|cmd| Vec::from([cmd]))?
                },
                None => Vec::new()
            }
        } else {
            args.commands.iter()
                .map(|cmd| parse_command_finish(new_regex_input(&cmd)))
                .collect::<io::Result<Vec<Command>>>()?
        };

    if args.debug {
        eprintln!("{:?}", commands)
    }

    let file_args = if args.commands.len() == 0 {
        &args.command_or_files[1..]
    } else {
        &args.command_or_files
    };

    let stdout = io::stdout();
    let mut out_handle = stdout.lock();

    if file_args.len() == 0 {
        let stdin = io::stdin();
        let in_handle = stdin.lock();
        run_commands(&commands, &mut in_handle.lines(), &mut out_handle, args.no_print)?;
    } else {
        for filename in file_args {
            let file = std::fs::File::open(filename)?;
            let buf_reader = io::BufReader::new(file);
            run_commands(&commands, &mut buf_reader.lines(), &mut out_handle, args.no_print)?;
        }
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use assert_ok::assert_ok;

    fn test_commands(cmd_strs: &[&str], input: &str, expected: &str) {
        let r_commands = cmd_strs.iter()
                .map(|cmd| parse_command_finish(new_regex_input(&cmd)))
                .collect::<io::Result<Vec<Command>>>();
        let commands = assert_ok!(r_commands);
        let mut lines = Vec::new();
        lines.push(Ok(input.to_owned()));
        let mut output = Vec::new();
        assert_ok!(
            run_commands(&commands, lines.into_iter(), &mut output, false));
        let mut actual = assert_ok!( String::from_utf8(output) );
        let last = actual.pop();
        assert_eq!(last, Some('\n'));
        assert_eq!(actual, expected);
    }

    fn test_one_command(command: &str, input: &str, expected: &str) {
        test_commands(&[command], input, expected)
    }

    #[test]
    fn replace() {
        test_one_command("s/a/b/", "ack", "bck")
    }

    #[test]
    fn replace_end() {
        test_one_command("s/$/d/", "foo", "food")
    }
}
