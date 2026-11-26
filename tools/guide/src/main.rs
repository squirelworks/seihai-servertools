//! `seihai-guide`: one tool of seihai-servertools on its own. The same topics
//! as `seihai-servertools guide`, without the `guide` word.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--version" | "-V"] => {
            println!("seihai-guide {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => seihai_guide::run(&words),
    }
}
