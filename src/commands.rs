use ::regex::Regex;

pub enum Command {
    S(Regex, String),
}
