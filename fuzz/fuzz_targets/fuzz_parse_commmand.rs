#![no_main]

use libfuzzer_sys::fuzz_target;

use tsed::commands::parse_command_finish;

fuzz_target!(|data: &[u8]| {
    // valid UTF-8 should not crash the command parser
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_command_finish(new_parser_input(&cmd));
    }
});
