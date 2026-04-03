use std::time::Duration;

use pid1::Pid1Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(2))
        .launch()?;
    let id = std::process::id();
    println!("In the simple process, going to sleep. Process ID is {id}");
    let args: Vec<String> = std::env::args().collect();
    println!("Args: {args:?}");

    if args.iter().any(|arg| arg == "--create-grandchildren") {
        println!("Spawning 3 grandchildren processes that will be orphaned.");
        for _ in 0..3 {
            // These children will be orphaned when `simple` exits, and then
            // adopted and reaped by pid1.
            std::process::Command::new("sleep").arg("1").spawn()?;
        }
    }

    if let Some(pos) = args.iter().position(|r| r == "--sleep") {
        if let Some(duration_str) = args.get(pos + 1) {
            if let Ok(duration) = duration_str.parse::<u64>() {
                println!("Going to sleep {duration} seconds");
                std::thread::sleep(std::time::Duration::from_secs(duration));
            }
        }
    }

    Ok(())
}
