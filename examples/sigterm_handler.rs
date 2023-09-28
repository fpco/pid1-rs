use std::time::Duration;

use pid1::Builder;
use signal_hook::consts::SIGTERM;
use signal_hook::iterator::Signals;

// This program handles sigterm and exits
fn main() {
    let mut builder = Builder::new();
    let builder = builder
        .timeout(Duration::from_secs(2))
        .enable_log(true)
        .build();
    pid1::relaunch_if_pid1(builder).expect("Relaunch failed");
    println!("This APP can be killed by SIGTERM (15)");
    let mut signals = Signals::new([SIGTERM]).unwrap();
    for signal in signals.forever() {
        println!("App got SIGTERM {signal}, going to exit");
        std::process::exit(0);
    }
}
