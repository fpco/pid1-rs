use rand::distributions::{Alphanumeric, DistString};
use std::{process::Command, time::Duration};

struct Container {
    name: String,
    image: String,
}

impl Container {
    pub fn new(image: String) -> Self {
        let name = Alphanumeric.sample_string(&mut rand::thread_rng(), 5);
        Container { name, image }
    }

    pub fn run(&self) -> Result<std::process::Output, std::io::Error> {
        Command::new("docker")
            .args([
                "run",
                "--name",
                self.name.as_str(),
                "-t",
                self.image.as_str(),
            ])
            .output()
    }

    pub fn plain_run(&self, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
        Command::new("docker").args(args).output()
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        let status = Command::new("docker")
            .args(["rm", self.name.as_str()])
            .spawn();
        if let Err(status) = status {
            eprintln!("Error dropping container {status}");
        }
    }
}

#[test]
fn sanity_test() {
    let container = Container::new("pid1rstest".to_owned());
    let output = container.run().unwrap();
    assert!(output.status.success(), "Process exited successfully");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&"pid1-rs: Process running as PID 1"),
        "One process runs as pid1",
    );
}

#[test]
fn reaps_zombie_process() {
    let container = Container::new("pid1rstest".to_owned());
    let (output, zombie_output) = std::thread::scope(|s| {
        let result = s.spawn(|| {
            let output = container.plain_run(&[
                "run",
                "--name",
                container.name.as_str(),
                "-t",
                container.image.as_str(),
                "/simple",
                "--sleep",
                "20",
            ]);
            output.unwrap()
        });

        let zombie_result = s.spawn(|| {
            std::thread::sleep(Duration::from_secs(2));
            let zombie_output = container
                .plain_run(&["exec", "-t", container.name.as_str(), "zombie"])
                .unwrap();
            zombie_output
        });

        (result.join().unwrap(), zombie_result.join().unwrap())
    });

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success(), "Process exited successfully");

    assert!(
        zombie_output.status.success(),
        "Process exited successfully"
    );

    assert!(
        stdout.contains(&"pid1-rs: Reaped PID"),
        "Successfully Reaped process",
    );
}

#[test]
fn child_process_status_code() {
    let container = Container::new("pid1rstest".to_owned());
    let (output, exec_process) = std::thread::scope(|s| {
        let result = s.spawn(|| {
            let output = container.plain_run(&[
                "run",
                "--name",
                container.name.as_str(),
                "-t",
                container.image.as_str(),
                "/simple",
                "--sleep",
                "20",
            ]);
            output.unwrap()
        });

        let kill_result = s.spawn(|| {
            std::thread::sleep(Duration::from_secs(2));
            let child_pid_output = container
                .plain_run(&[
                    "exec",
                    container.name.as_str(),
                    "cat",
                    "/proc/1/task/1/children",
                ])
                .unwrap();
            let child_pid_str = String::from_utf8(child_pid_output.stdout).unwrap();
            let child_pid = child_pid_str.trim();

            println!("Child process: {child_pid}");

            container
                .plain_run(&[
                    "exec",
                    "-t",
                    container.name.as_str(),
                    "kill",
                    "-12",
                    child_pid,
                ])
                .unwrap()
        });

        (result.join().unwrap(), kill_result.join().unwrap())
    });

    assert!(!output.status.success(), "Pid1 process exited");
    assert_eq!(
        output.status.code().unwrap(),
        140,
        "Exit code is 140 (128 + 12)"
    );
    assert!(exec_process.status.success(), "Killed process successfully");
}

#[test]
fn sigterm_handling() {
    let container = Container::new("pid1rstest".to_owned());
    let (output, exec_process) = std::thread::scope(|s| {
        let result = s.spawn(|| {
            let output = container.plain_run(&[
                "run",
                "--name",
                container.name.as_str(),
                "-t",
                container.image.as_str(),
                "sigterm_handler",
            ]);
            output.unwrap()
        });

        let kill_result = s.spawn(|| {
            std::thread::sleep(Duration::from_secs(2));
            container
                .plain_run(&["exec", "-t", container.name.as_str(), "kill", "1"])
                .unwrap()
        });

        (result.join().unwrap(), kill_result.join().unwrap())
    });

    assert!(output.status.success(), "Pid1 exited successfully");
    assert!(exec_process.status.success(), "Killed process successfully");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("App got SIGTERM 15, going to exit"),
        "Application got SIGTERM from pid1"
    );
}

#[test]
fn sigterm_ignore() {
    let container = Container::new("pid1rstest".to_owned());
    let (output, exec_process) = std::thread::scope(|s| {
        let result = s.spawn(|| {
            let output = container.plain_run(&[
                "run",
                "--name",
                container.name.as_str(),
                "-t",
                container.image.as_str(),
                "sigterm_loop",
            ]);
            output.unwrap()
        });

        let kill_result = s.spawn(|| {
            std::thread::sleep(Duration::from_secs(2));
            container
                .plain_run(&["exec", "-t", container.name.as_str(), "kill", "1"])
                .unwrap()
        });

        (result.join().unwrap(), kill_result.join().unwrap())
    });

    assert!(!output.status.success(), "Pid1 exited unsuccessfully");
    assert_eq!(
        output.status.code().unwrap(),
        137,
        "pid1 exited with 9 (137 - 128) status code"
    );
    assert!(exec_process.status.success(), "Killed process successfully");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("This APP cannot be killed by SIGTERM (15)"),
        "Application ignores SIGTERM"
    );

    assert!(
        stdout.contains("App got SIGTERM 15, but *NOT* going to exit"),
        "Application got SIGTERM"
    );
}

#[test]
fn reaps_multiple_zombie_processes() {
    let container = Container::new("pid1rstest".to_owned());
    // This test simulates a scenario where multiple orphaned processes are created
    // in quick succession. This can lead to coalesced SIGCHLD signals.
    // A correct pid1 implementation should reap all of them.
    let (_run_output, zombie_check_output) = std::thread::scope(|s| {
        // 1. Run a long-running process in the container as PID 1's child.
        let result = s.spawn(|| {
            container
                .plain_run(&[
                    "run",
                    "--name",
                    container.name.as_str(),
                    "-t",
                    container.image.as_str(),
                    "/simple",
                    "--sleep",
                    "20",
                ])
                .unwrap()
        });
        std::thread::sleep(Duration::from_secs(2)); // Give container time to start.

        // 2. Concurrently spawn multiple processes that will become zombies.
        // The `zombie` example forks a child and the parent exits, orphaning the child.
        // The child then exits, becoming a zombie to be reaped by pid1.
        for _ in 0..3 {
            s.spawn(|| {
                container
                    .plain_run(&["exec", "-t", container.name.as_str(), "zombie"])
                    .unwrap();
            });
        }
        std::thread::sleep(Duration::from_secs(5)); // Allow time for zombies to be created.

        // 3. Check for zombie processes inside the container.
        // This command exits with 0 if no zombies are found, and 1 otherwise.
        // `grep -q Z` exits with 0 if 'Z' (zombie state) is found, 1 otherwise.
        // The `!` negates the exit code, so the whole command succeeds (exits 0)
        // if no zombies are found.
        let zombie_check_output = container
            .plain_run(&[
                "exec",
                "-t",
                container.name.as_str(),
                "sh",
                "-c",
                "! cat /proc/*/status 2>/dev/null | grep 'State:' | grep -q Z",
            ])
            .unwrap();

        // 4. Clean up by stopping the container. This allows the `docker run`
        // command to finish, preventing the test from hanging.
        let _ = container.plain_run(&["stop", "-t", "1", container.name.as_str()]);
        (result.join().unwrap(), zombie_check_output)
    });

    // The zombie check command succeeds (exit code 0) if no zombies are found.
    // Exit code 1 means zombies were found.
    // Exit code 2 means there was an I/O error during the check.
    assert_eq!(
        zombie_check_output.status.code(),
        Some(0),
        "Zombie check failed. Stderr:\n{}",
        String::from_utf8_lossy(&zombie_check_output.stderr)
    );
}
