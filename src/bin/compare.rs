// Compare tealsed output to one of the older implementations

use tsed::commands::*;

use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;

// Only the zero-arg constructors
const FUNCTIONS: &'static [Function] = &[ Function::Equals, Function::D, Function::Fd, Function::G, Function::Fg, Function::H, Function::Fh, Function::Fp, Function::Fx ];
const NAMES : &'static [&'static str] = &["alpha", "bravo"];

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
    names: Rc<RefCell<Names>>,
    count: u8,
}

impl AddressIter {
    fn new(names: Rc<RefCell<Names>>) -> AddressIter {
        AddressIter { names, count: 0}
    }

    fn reset(&mut self) {
        self.count = 0;
    }
}

impl Iterator for AddressIter {
    type Item = Address;

    fn next(&mut self) -> Option<Address> {
        use Address::*;
        self.count += 1;
        match self.count {
            1 => {
                // share line number supply between iterators?
                Some(LineNumber(1))   
            },
            2 => {
                let mut names = self.names.borrow_mut();
                names.next().map( |name| Context(Regex::new(&name).unwrap()))
            },
            _ => None,
        }
    }
}

struct FunctionIter {
    // multiple iterators share a supply of names so that names are unique within each command
    names: Rc<RefCell<Names>>,
    count: usize
}

impl FunctionIter {
    fn new(names: Rc<RefCell<Names>>) -> FunctionIter {
        FunctionIter { names, count: 0 }
    }

    fn reset(&mut self) {
        self.count = 0;
    }
}

impl Iterator for FunctionIter {
    type Item = Function;

    fn next(&mut self) -> Option<Function> {
        use Function::*;
        self.count += 1;
        let mut names = self.names.borrow_mut();
        match self.count {
            1 => names.next().map( |n| Fi(n.to_string())),
            2 => {
                let o_regex = names.next();
                let o_replacement = names.next();
                match (o_regex, o_replacement) {
                    (Some(regex), Some(replacement)) =>
                        Some(Fs(Regex::new(regex).unwrap(), replacement.to_string())),
                    _ => None
                }
            },
            _ => FUNCTIONS.get(self.count - 3).map(|f| f.clone())
        }
    }
}

struct CommandIter {
    names: Rc<RefCell<Names>>,
    address: u8, // 0, 1, or 2
    start: Option<Address>, // last chosen, keep using until end runs out
    start_iter: AddressIter,
    end: Option<Address>,
    end_iter: AddressIter,
    function: FunctionIter,
}

impl CommandIter {
    fn new() -> CommandIter {
        let names = Rc::new(RefCell::new(Names::new()));
        CommandIter {
            names: names.clone(),
            address: 1, // 0 case is handled before first call to next_addr_pair
            start: None,
            start_iter: AddressIter::new(names.clone()),
            end: None,
            end_iter: AddressIter::new(names.clone()),
            function: FunctionIter::new(names.clone()),
        }
    }

    // [(None, None)] ++ [(Some a, None) | a <- AddressIter] ++ [(Some a, Some b) | a <- AddressIter, b <- AddressIter ]
    fn next_addr_pair(&mut self) -> Option<(Option<Address>, Option<Address>)> {
        match self.address {
            // 0 => {
            //     self.address += 1;
            //     Some((None, None))
            // }G,
            1 => {
                let start = self.start_iter.next();
                if start.is_some() {
                    Some((start, None))
                } else {
                    self.address += 1;
                    self.start_iter.reset();
                    self.start = self.start_iter.next();
                    self.next_addr_pair()
                }
            },
            2 => {
                match self.end_iter.next() {
                    Some(end) => Some((self.start.clone(), Some(end))),
                    None => {
                        self.start = self.start_iter.next();
                        self.end_iter.reset();
                        if self.start.is_none() {
                            self.address += 1;
                        }
                        self.next_addr_pair()
                    }
                }
            },
            _ => None
        }
    }
}

impl Iterator for CommandIter {
    type Item = Command;

    fn next(&mut self) -> Option<Command> {
        // fresh name supply for each command
        // eventually this will be fresh per Program (list of Commands)
        self.names.replace(Names::new());
        match self.function.next() {
            Some(function) => Some(Command { start: self.start.clone(), end: self.end.clone(), function }),
            None => {
                // try the next address
                self.function.reset();
                match self.next_addr_pair() {
                    // TODO move this into next_addr_pair?
                    Some((start, end)) => {
                        self.start = start;
                        self.end = end;
                        self.next()
                    },
                    None => None
                }
            }
        }
    }
}

fn main() {
    let gen = CommandIter::new();
    for cmd in gen {
        println!("{}", cmd);
    }
}
