use std::{
    ffi::OsString,
    io::{self, BufRead, Write},
    process::Command,
    str::FromStr,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        for line in stdin.lock().lines() {
            let args: Vec<OsString> = line
                .unwrap()
                .split(' ')
                .map(|item| OsString::from_str(item).unwrap())
                .collect();
            let exe = args.get(0).unwrap();
            if exe == "exit" {
                std::process::exit(0);
            }
            let output = Command::new(exe).args(&args[1..]).output().unwrap();
            print!("{}", String::from_utf8(output.stdout).unwrap());
            print!("$ ");
            io::stdout().flush().unwrap();
        }
    }
}
