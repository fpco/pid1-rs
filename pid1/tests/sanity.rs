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
        // Use rm -f to stop and remove the container. This is robust
        // and ensures cleanup even if tests fail to stop the container.
        let output = Command::new("docker")
            .args(["rm", "-f", self.name.as_str()])
            .output();

        if let Ok(output) = output {
            if !output.status.success() {
                // It is possible that the container was already removed, so
                // we don't want to panic here.
                eprintln!(
                    "pid1-rs-test: Could not remove container {}. Stderr: {}",
                    self.name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}

#[test]
fn sanity_test() {
    let container = Container::new("pid1rstest".to_owned());
    let output = container.run().unwrap();
    assert!(output.status.success(), "Process exited successfully");
    let stdout = String::from_utf8_lossy(&output.stdout);
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
                "4",
            ]);
            output.unwrap()
        });

        std::thread::sleep(Duration::from_secs(2)); // Give container time to start.

        let zombie_result = s.spawn(|| {
            let zombie_output = container
                .plain_run(&["exec", "-t", container.name.as_str(), "zombie"])
                .unwrap();
            zombie_output
        });

        (result.join().unwrap(), zombie_result.join().unwrap())
    });

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Process exited successfully");

    assert!(
        stdout.contains(&"pid1-rs: Reaped PID"),
        "Successfully Reaped process",
    );

    assert!(
        zombie_output.status.success(),
        "Process exited successfully"
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
                "5",
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
            let child_pid_str = String::from_utf8_lossy(&child_pid_output.stdout);
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

    let stdout = String::from_utf8_lossy(&output.stdout);
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

    let stdout = String::from_utf8_lossy(&output.stdout);
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
    let (run_output, zombie_check_output) = std::thread::scope(|s| {
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
                    "10",
                ])
                .unwrap()
        });
        std::thread::sleep(Duration::from_secs(2)); // Give container time to start.

        // 2. Concurrently spawn multiple processes that will become zombies.
        // We run a command in a subshell `(...)` and in the background `&`.
        // The `sh` process that docker exec starts exits immediately, orphaning
        // the subshell process. The subshell process is then adopted by PID 1.
        // It then sleeps for a second and exits, becoming a zombie for pid1 to reap.
        for _ in 0..3 {
            s.spawn(|| {
                container
                    .plain_run(&["exec", container.name.as_str(), "sh", "-c", "sleep 1 &"])
                    .unwrap();
            });
        }
        // Allow time for zombies to be created and reaped (1s sleep + buffer).
        std::thread::sleep(Duration::from_secs(5));

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

        // 4. Clean up is handled by the main process exiting after its sleep
        // duration. This allows the `docker run` command to finish naturally.
        (result.join().unwrap(), zombie_check_output)
    });

    let stdout = String::from_utf8_lossy(&run_output.stdout);
    // We expect at least 3 zombies to be reaped. In practice, this may be higher
    // due to the shell processes from `docker exec` and the main child process
    // also being reaped by pid1. We assert that there are at least 3 reaps,
    // and then rely on the zombie check to confirm that *all* zombies were handled.
    let reaped_count = stdout.matches("pid1-rs: Reaped PID").count();
    assert!(
        reaped_count >= 3,
        "Expected to reap at least 3 zombie processes, but reaped {}. stdout:\n{}",
        reaped_count,
        stdout
    );

    // The zombie check command succeeds (exit code 0) if no zombies are found.
    if zombie_check_output.status.code() != Some(0) {
        // If the check fails, get more debug info from the container.
        let ps_output = container
            .plain_run(&["exec", container.name.as_str(), "ps", "-aux"])
            .unwrap();
        let ps_stdout = String::from_utf8_lossy(&ps_output.stdout);
        let zombie_stderr = String::from_utf8_lossy(&zombie_check_output.stderr);
        panic!(
            "Zombie check failed with code {:?}. Stderr:\n{}\n\nps -aux output:\n{}",
            zombie_check_output.status.code(),
            zombie_stderr,
            ps_stdout
        );
    }
}

#[test]
fn reaps_orphaned_grandchildren() {
    let container = Container::new("pid1rstest".to_owned());
    // This test validates that pid1 can adopt and reap orphaned "grandchildren".
    // 1. We run `/simple` with `--create-grandchildren`, which spawns 3 `sleep` processes.
    // 2. The `/simple` process (the parent) then sleeps and exits.
    // 3. This orphans the `sleep` processes, which should be adopted by pid1.
    // 4. As the `sleep` processes exit, pid1 should reap them.
    // 5. Finally, pid1 reaps the original `/simple` child and exits.
    let output = container
        .plain_run(&[
            "run",
            "--name",
            container.name.as_str(),
            "-t",
            container.image.as_str(),
            "/simple",
            "--create-grandchildren",
            "--sleep",
            "3", // Long enough for children to be spawned, short enough for test speed.
        ])
        .unwrap();

    assert!(
        output.status.success(),
        "Container should exit successfully. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // We expect to reap the 3 grandchildren + the main `/simple` child process.
    let reaped_count = stdout.matches("pid1-rs: Reaped PID").count();
    assert!(
        reaped_count >= 4,
        "Expected to reap at least 4 processes (3 grandchildren + 1 child), but found {}. stdout:\n{}",
        reaped_count,
        stdout
    );
}
