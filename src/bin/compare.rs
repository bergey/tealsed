// Compare tealsed output to one of the older implementations

use tsed::commands::*;

use regex::Regex;
use std::process;

// Only the zero-arg constructors
const FUNCTIONS: &'static [Function] = &[
    Function::Equals,
    Function::D,
    Function::Fd,
    Function::G,
    Function::Fg,
    Function::H,
    Function::Fh,
    Function::Fp,
    Function::Fx,
];
const NAMES: &'static [&'static str] = &["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"];

fn address_iter<U: Iterator<Item = u64>, N: Iterator<Item = &'static str>>(
    lines: U,
    names: N,
) -> impl Iterator<Item = Address> {
    lines
        .map(Address::LineNumber)
        .chain(names.map(|n| Address::Context(Regex::new(n).unwrap())))
}
fn s_iter(names: &'static [&'static str]) -> impl Iterator<Item = Function> {
    names.iter().flat_map(|n| {
        names
            .iter()
            .map(|s| Function::Fs(Regex::new(n).unwrap(), s.to_string()))
    })
}

fn function_iter(names: &'static [&'static str]) -> impl Iterator<Item = Function> {
    FUNCTIONS
        .into_iter()
        .cloned()
        .chain(names.iter().map(|n| Function::Fi(n.to_string())))
        .chain(s_iter(names))
}

fn command_iter(names: &'static [&'static str]) -> impl Iterator<Item = Command> {
    function_iter(names)
        .map(|f| Command {
            start: None,
            end: None,
            function: f,
        })
        .chain(
            address_iter(1..7, names.into_iter().cloned()).flat_map(|a| {
                function_iter(NAMES).map(move |function| Command {
                    start: Some(a.clone()),
                    end: None,
                    function,
                })
            }),
        )
        .chain(
            address_iter(1..7, names.into_iter().cloned()).flat_map(|s| {
                address_iter(1..7, names.into_iter().cloned()).flat_map(move |e| {
                    let s = s.clone();
                    function_iter(NAMES).map(move |function| Command {
                        start: Some(s.clone()),
                        end: Some(e.clone()),
                        function,
                    })
                })
            }),
        )
}

fn invoke(program: &str, commands: &[Command]) -> std::io::Result<process::Output> {
    let mut p = process::Command::new(program);
    for c in commands {
        p.arg("-e");
        p.arg(format!("{}", c));
    }
    p.arg("compare_test_data.txt");
    p.output()
}

fn main() -> std::io::Result<()> {
    // let mut gen = CommandIter::new(Names::new());
    // while let Some(cmd) = gen.next() {
    for cmd in command_iter(NAMES) {
        // TODO exclude this in the generator
        match (&cmd.start, &cmd.end, &cmd.function) {
            (Some(_), Some(_), Function::Equals) => continue,
            _ => (),
        };
        let commands = vec![cmd.clone()];
        let expected = invoke("sed", &commands)?;
        if !expected.status.success() {
            println!("sed {cmd} exited with {}", expected.status);
        }
        let actual = invoke("target/debug/tsed", &commands)?;
        if !actual.status.success() {
            println!("tsed {cmd} exited with {}", expected.status);
        }
        if expected.stdout != actual.stdout {
            let s_expected = String::from_utf8_lossy(&expected.stdout);
            let s_actual = String::from_utf8_lossy(&actual.stdout);
            println!("{cmd} {:?}\nexpected:\n{s_expected}\ngot:\n{s_actual}", cmd);
        }
    }
    // pairs of commands
    for cmd in command_iter(NAMES) {
        for c2 in command_iter(NAMES) {
            let commands = vec![cmd.clone(), c2.clone()];
            let display_commands = format!("'{cmd}' '{c2}'");
            let expected = invoke("sed", &commands)?;
            if !expected.status.success() {
                println!("sed {display_commands} exited with {}", expected.status);
            }
            let actual = invoke("tsed", &commands)?;
            if !actual.status.success() {
                println!("tsed {display_commands} exited with {}", expected.status);
            }
            if expected.stdout != actual.stdout {
                let s_expected = String::from_utf8_lossy(&expected.stdout);
                let s_actual = String::from_utf8_lossy(&actual.stdout);
                println!("{display_commands}\nexpected:\n{s_expected}\ngot:\n{s_actual}");
            }
        }
    }
    Ok(())
}
