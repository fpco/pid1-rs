fn main() -> Result<(), Box<dyn std::error::Error>> {
    pid1::relaunch_if_pid1()?;
    let id = std::process::id();
    println!("In the simple process, going to sleep. Process ID is {id}");
    let args = std::env::args().collect::<Vec<_>>();
    println!("Args: {args:?}");
    for _ in 0..4 {
        std::process::Command::new("date").spawn().unwrap();
    }
    std::thread::sleep(std::time::Duration::from_secs(5));
    Ok(())
}