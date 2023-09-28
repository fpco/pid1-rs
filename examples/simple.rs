use std::time::Duration;

use pid1::Pid1Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pid1::relaunch_if_pid1(Pid1Settings {
        log: true,
        timeout: Duration::from_secs(2),
    })?;
    let id = std::process::id();
    println!("In the simple process, going to sleep. Process ID is {id}");
    let args = std::env::args().collect::<Vec<_>>();
    println!("Args: {args:?}");

    for _ in 0..4 {
        std::thread::sleep(std::time::Duration::from_secs(2));
        std::process::Command::new("date").spawn().unwrap();
    }

    if args.len() > 1 {
        println!("Going to sleep 500 seconds");
        std::thread::sleep(std::time::Duration::from_secs(500));
    }

    Ok(())
}
