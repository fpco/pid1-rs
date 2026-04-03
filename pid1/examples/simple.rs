use std::time::Duration;

use pid1::Pid1Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Subreaper is enabled by default in this example. It can be disabled by
    // setting the PID1_NO_SUBREAPER environment variable.
    let subreaper_enabled = std::env::var("PID1_NO_SUBREAPER").is_err();

    Pid1Settings::new()
        .enable_log(true)
        .enable_sub_reaper(subreaper_enabled)
        .timeout(Duration::from_secs(2))
        .launch()?;
    let id = std::process::id();
    println!("In the simple process, going to sleep. Process ID is {id}");
    let args: Vec<String> = std::env::args().collect();
    println!("Args: {args:?}");

    if args.iter().any(|arg| arg == "--create-grandchildren") {
        println!("Spawning 3 grandchildren processes that will be orphaned.");
        for _ in 0..3 {
            // These children will become orphans because this parent process (ie. /simple) does not
            // wait for them to complete before exiting. They will be adopted and
            // reaped by the container's PID 1 process.
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
