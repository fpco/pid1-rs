use std::time::Duration;

use pid1::Pid1Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(2))
        .launch()?;
    let id = std::process::id();
    println!("Process ID is {id}");
    main_inner()
}

#[tokio::main]
async fn main_inner() -> Result<(), Box<dyn std::error::Error>> {
    println!("Going to sleep from main_inner");
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(())
}
