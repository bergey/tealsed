// Compare tealsed output to one of the older implementations

use tsed::commands::*;

use lazy_static::lazy_static;
use regex::Regex;

struct Generator<'a> {
    names: std::slice::Iter<'a, &'static str>,
    address: usize,
    last_line_number: u64,
    function: usize,
}

const FUNCTIONS: &'static [Function] = &[ Function::Equals, Function::D, Function::Fd, Function::G, Function::Fg, Function::H, Function::Fh, Function::Fp, Function::Fx ];
const NAMES : &'static [&'static str] = &["alpha", "bravo"];

impl<'a> Generator<'a> {
    fn new() -> Generator<'a> {
        Generator {
            names: NAMES.iter(),
            address: 0,
            last_line_number: 0,
            function: 0,
        }
    }

    fn next_address(&mut self) -> Option<Address> {
        use Address::*;
        self.address += 1;
        match self.address {
            1 => {
                self.last_line_number += 1;
                Some(LineNumber(self.last_line_number))   
            },
            2 => {
                let o_name = self.names.next();
                o_name.map( |name| Context(Regex::new(&name).unwrap()))
            },
            _ => None,
        }
    }

    fn reset_address(&mut self) {
        self.address = 0;
    }

    fn next_function(&mut self) -> Option<Function> {
        use Function::*;
        self.function += 1;
        match self.function {
            1 => self.names.next().map( |n| Fi(n.to_string())),
            2 => {
                let o_regex = self.names.next();
                let o_replacement = self.names.next();
                match (o_regex, o_replacement) {
                    (Some(regex), Some(replacement)) =>
                        Some(Fs(Regex::new(regex).unwrap(), replacement.to_string())),
                    _ => None
                }
            },
            _ => FUNCTIONS.get(self.function - 3).map(|f| f.clone())
        }
    }

    fn reset_function(&mut self) {
        self.function = 0;
    }

}

fn main() {
    let mut gen = Generator::new();
    println!("{:?}", gen.next_function());
    // TODO loop
}
