mod cli;

use std::env::{args_os, self};

use cli::{handle_arg, parse_args};

fn main() {
    let args: Vec<_> = args_os().skip(1).collect();

    println!("inside main");
    for (key, value) in env::vars_os() {
        println!("{key:?}: {value:?}");
    }

    let pid1_arg = parse_args(args);
    match pid1_arg {
        Ok(pid1_arg) => {
            handle_arg(pid1_arg);
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
}
