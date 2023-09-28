use std::time::Duration;

use pid1::Builder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = Builder::new();
    let builder = builder
        .timeout(Duration::from_secs(2))
        .enable_log(true)
        .build();
    pid1::relaunch_if_pid1(builder)?;
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
