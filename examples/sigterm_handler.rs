use std::time::Duration;

use pid1::Pid1Settings;
use signal_hook::iterator::Signals;
use signal_hook::consts::SIGTERM;

// This program handles sigterm and exits
fn main()  {
    pid1::relaunch_if_pid1(Pid1Settings { log: true, timeout: Duration::from_secs(2) }).expect("Relaunch failed");
    println!("This APP can be killed by SIGTERM (15)");
    let mut signals = Signals::new([SIGTERM]).unwrap();
    for signal in signals.forever() {
        println!("App got SIGTERM {signal}, going to exit");
        std::process::exit(0);
    }
}
