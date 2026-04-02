use std::time::Duration;

use nix::unistd::{fork, ForkResult};

// This program creates a zombie process
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::process::id();
    println!("Process ID is {id}");

    match unsafe { fork() } {
        Ok(ForkResult::Parent { .. }) => {
            println!("Parent process: going to sleep and exit");
            // We are sleeping so that the child process that exits
            // without being waited upon appears in the process table
            // as zombie. When this parent process exits, the zombie child
            // will be orphaned and adopted by pid1.
            std::thread::sleep(Duration::from_secs(1));
            std::process::exit(0);
        }
        Ok(ForkResult::Child) => {
            std::process::exit(0);
        }
        Err(_) => println!("Fork failed"),
    }

    Ok(())
}
