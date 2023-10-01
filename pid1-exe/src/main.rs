mod cli;

use clap::Parser;

use crate::cli::Pid1App;

fn main() {
    let cli = Pid1App::parse();
    cli.run()
}
