use std::time::Duration;

use pid1::Pid1Settings;
use signal_hook::consts::SIGTERM;
use signal_hook::iterator::Signals;

// This program handles sigterm and exits
fn main() {
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(2))
        .launch()
        .expect("Launch failed");
    println!("This APP cannot be killed by SIGTERM (15)");
    let mut signals = Signals::new([SIGTERM]).unwrap();
    for signal in signals.forever() {
        println!("App got SIGTERM {signal}, but *NOT* going to exit");
    }
}
