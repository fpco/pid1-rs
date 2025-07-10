use rand::distributions::{Alphanumeric, DistString};
use std::{process::Command, time::Duration};

#[derive(Clone)]
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
            let container = container.clone();
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
            let container = container.clone();
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
            let container = container.clone();
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
            let container = container.clone();
            let ps_output = container
                .plain_run(&[
                    "exec",
                    "-t",
                    container.name.as_str(),
                    "ps",
                    "-o",
                    "pid",
                    "a",
                ])
                .unwrap();
            let ps_output = String::from_utf8(ps_output.stdout).unwrap();
            let ps_output = ps_output.lines().skip(2).next().unwrap().trim();

            println!("Child process: {ps_output}");

            container
                .plain_run(&[
                    "exec",
                    "-t",
                    container.name.as_str(),
                    "kill",
                    "-12",
                    ps_output,
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
            let container = container.clone();
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
            let container = container.clone();
            let ps_output = container
                .plain_run(&[
                    "exec",
                    "-t",
                    container.name.as_str(),
                    "ps",
                    "-o",
                    "pid",
                    "a",
                ])
                .unwrap();
            let ps_output = String::from_utf8(ps_output.stdout).unwrap();
            let ps_output = ps_output.lines().skip(2).next().unwrap().trim();

            println!("Child process: {ps_output}");

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
            let container = container.clone();
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
            let container = container.clone();
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
