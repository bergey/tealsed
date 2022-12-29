pub mod equivalent;

pub mod parser;
pub use parser::parse;

pub mod parser_state;
pub use parser_state::new_parser_input;

mod replace;
pub use replace::{replace, replace_all};
    
