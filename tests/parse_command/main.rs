use bolero::check;

use tsed::commands::parse_command_finish;
use tsed::regex::parser_state::new_parser_input;

fn main() {
    check!().with_type().for_each(|value: &String| {
        let _ = parse_command_finish(new_parser_input(&value));
    });
}
