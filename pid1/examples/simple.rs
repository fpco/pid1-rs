use std::time::Duration;

use pid1::Pid1Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(2))
        .launch()?;
    let id = std::process::id();
    println!("In the simple process, going to sleep. Process ID is {id}");
    let args = std::env::args().collect::<Vec<_>>();
    println!("Args: {args:?}");

    for _ in 0..4 {
        std::thread::sleep(std::time::Duration::from_secs(2));
        std::process::Command::new("date").spawn().unwrap();
    }

    if args.len() > 1 {
        let duration = &args[2];
        let duration = duration.parse().expect("Expected int value");
        println!("Going to sleep {duration} seconds");
        std::thread::sleep(std::time::Duration::from_secs(duration));
    }

    Ok(())
}
