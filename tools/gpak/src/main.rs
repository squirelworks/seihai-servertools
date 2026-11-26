//! `seihai-gpak`: one tool of seihai-servertools on its own. The same commands
//! as `seihai-servertools gpak`, without the `gpak` word.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--version" | "-V"] => {
            println!("seihai-gpak {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => seihai_gpak::run(&words),
    }
}
