// Compare tealsed output to one of the older implementations

use tsed::commands::*;

use std::process;
use regex::Regex;

// generators store the most recently yielded value T, but also need
// to track whether they have ever produced a value, or have produced
// the last value (in case accidentally called again).
enum Progress<T> { Start , Just(T) , End }

impl<T> Progress<T> {
    pub fn from_option(opt: Option<T>) -> Progress<T> {
        match opt {
            Some(val) => Progress::Just(val),
            None => Progress::End
        }
    }
}

trait Generator {
    type Yield;
    type State;

    // resume should advance the Generator iff next has never been called
    // this way we don't need to insist that next() is called before resume(),
    // nor make the caller handle the Progress::Start value
    fn resume(&mut self) -> Option<Self::Yield>;
    fn next(&mut self) -> Option<Self::Yield>;
    fn state(&self) -> Self::State;
}

// Only the zero-arg constructors
const FUNCTIONS: &'static [Function] = &[ Function::Equals, Function::D, Function::Fd, Function::G, Function::Fg, Function::H, Function::Fh, Function::Fp, Function::Fx ];
const NAMES : &'static [&'static str] = &["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"];

#[derive(Clone, Debug)]
struct Names {
    strings: std::slice::Iter<'static, &'static str>,
    used_count: usize,
}

impl Names {
    fn new() -> Names {
        Names {
            strings: NAMES.iter(),
            used_count: 0,
        }
    }

    fn used(&self) -> &[&'static str] {
        &NAMES[0..self.used_count+1]
    }
}

impl Iterator for Names {
    type Item = &'static str;
    
    fn next(&mut self) -> Option<&'static str> {
        let ret = self.strings.next();
        if ret.is_some() {
            self.used_count += 1;
        }
        ret.map(|s| *s)
    }
}

struct AddressIter {
    before: Names,
    current: Progress<Address>,
    after: Names,
}

impl AddressIter {
    fn new(names: Names) -> AddressIter {
        AddressIter { before: names.clone(), current: Progress::Start, after: names }
    }
}

impl Generator for AddressIter {
    type Yield = Address;
    type State = Names;

    fn next(&mut self) -> Option<Address> {
        use Address::*;
        let ret = match &self.current {
            Progress::Start => Some(LineNumber(1)),
            Progress::Just(LineNumber(_)) => {
                self.after = self.before.clone();
                self.after.next().map( |name| Context(Regex::new(&name).unwrap()))
            },
            _ => None,
        };
        self.current = Progress::from_option(ret.clone());
        ret
    }

    fn resume(&mut self) -> Option<Address> {
        match &self.current {
            Progress::Start => self.next(),
            Progress::Just(addr) => Some(addr.clone()),
            Progress::End => None,
        }
    }

    fn state(&self) -> Names {
        self.after.clone()
    }
}

struct FunctionIter {
    before: Names,
    current: Progress<Function>,
    count: usize,
    after: Names
}

impl FunctionIter {
    fn new(names: Names) -> FunctionIter {
        FunctionIter { before: names.clone(), current: Progress::Start, count: 0, after: names }
    }
}

impl Generator for FunctionIter {
    type Yield = Function;
    type State = Names;

    fn next(&mut self) -> Option<Function> {
        use Function::*;
        self.count += 1;
        match self.count {
            1 => self.after.next().map( |n| Fi(n.to_string())),
            2 => {
                self.after = self.before.clone();
                let o_regex = self.after.next();
                let o_replacement = self.after.next();
                match (o_regex, o_replacement) {
                    (Some(regex), Some(replacement)) =>
                        Some(Fs(Regex::new(regex).unwrap(), replacement.to_string())),
                    _ => None
                }
            },
            _ => FUNCTIONS.get(self.count - 3).map(|f| f.clone())
        }
    }

    fn resume(&mut self) -> Option<Function> {
        match &self.current {
            Progress::Start => self.next(),
            Progress::Just(f) => Some(f.clone()),
            Progress::End => None,
        }
    }

    fn state(&self) -> Names {
        self.after.clone()
    }
}

struct CommandIter {
    address_count: i8, // [-1, 3], the bounds representing before / after
    start: AddressIter,
    end: AddressIter,
    function: FunctionIter,
    // before is start.before; after is function.after
}

impl CommandIter {
    fn new(names: Names) -> CommandIter {
        CommandIter {
            address_count: -1,
            start: AddressIter::new(names.clone()),
            end: AddressIter::new(names.clone()),
            function: FunctionIter::new(names),
        }
    }

    fn resume_addr_pair(&mut self) -> Option<(Option<Address>, Option<Address>)> {
        match self.address_count {
            -1 => self.next_addr_pair(),
            0 => Some((None, None)),
            1 => Some((self.start.resume(), None)),
            2 => Some((self.start.resume(), self.end.resume())) ,
            _ => None
        }
    }

    // [(None, None)] ++ [(Some a, None) | a <- AddressIter] ++ [(Some a, Some b) | a <- AddressIter, b <- AddressIter ]
    fn next_addr_pair(&mut self) -> Option<(Option<Address>, Option<Address>)> {
        match self.address_count {
            -1 => {
                self.address_count += 1;
                Some((None, None))
            }
            0 => {
                // there's only one pair with 0 addresses, so we know we already yielded the last such element
                self.address_count += 1;
                Some((self.start.next(), None))
            }
            1 => {
                let start = self.start.next();
                if start.is_some() {
                    Some((start, None))
                } else {
                    self.address_count += 1;
                    self.start = AddressIter::new(self.start.before.clone());
                    // don't need to reset end because we have not been advancing it
                    Some((self.start.next(), self.end.next()))
                }
            },
            2 => {
                match self.end.next() {
                    Some(end) => Some((self.start.resume(), Some(end))),
                    None => {
                        let start = self.start.next();
                        self.end = AddressIter::new(self.start.state());
                        if start.is_none() {
                            self.address_count += 1; // finished
                        }
                        Some((start, self.end.next()))
                    }
                }
            },
            _ => None
        }
    }

    fn addr_state(&self) -> Names {
        if self.address_count == 2 {
            self.end.state()
        } else {
            self.start.state()
        }
    }
}

impl Generator for CommandIter {
    type Yield = Command;
    type State = Names;

    fn resume(&mut self) -> Option<Command> {
        match (self.resume_addr_pair(), self.function.resume()) {
            (Some((start, end)), Some(function)) => Some(Command { start, end, function }),
            _ => None
        }
    }

    fn next(&mut self) -> Option<Command> {
        match (self.resume_addr_pair(), self.function.next()) {
            (Some((start, end)), Some(function)) => Some(Command { start, end, function }),
            (Some(_), None) => {
                self.next_addr_pair();
                self.function = FunctionIter::new(self.addr_state());
                self.next() // avoid repeating the same match we're in
            }
            (None, _) => None,
        }
    }

    fn state(&self) -> Names {
        self.function.state()
    }
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
    let mut gen = CommandIter::new(Names::new());
    // for cmd in gen {
    while let Some(cmd) = gen.next() {
        let mut gen2 = CommandIter::new(gen.state());
        while let Some(c2) = gen2.next() {
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
