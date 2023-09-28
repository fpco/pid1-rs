#[cfg(target_family = "unix")]
use nix::{
    sys::{signal::kill, wait::WaitStatus},
    unistd::Pid,
};
#[cfg(target_family = "unix")]
use signal_hook::{
    consts::{SIGCHLD, SIGINT, SIGTERM},
    iterator::Signals,
};
#[cfg(target_family = "unix")]
use std::ffi::c_int;
#[cfg(target_family = "unix")]
use std::process::Child;
use std::time::Duration;

/// The `Error` enum indicates that the [`relaunch_if_pid1`] was not
/// successful.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Failed when respawning of non-PID1 child process
    #[error("Failed when respawning non-PID1 child process: {0}")]
    SpawnChild(std::io::Error),
}

#[allow(clippy::needless_doctest_main)]
/// When run as PID 1, relaunch the current process as a child process
/// and do proper signal and zombie reaping in PID 1.
///
/// This function should be the first statement within your main
/// function.
///
/// # Examples
///
/// ```rust,no_test
/// use std::time::Duration;
/// use pid1::Builder;
///
/// fn main() {
///    let mut builder = Builder::new();
///    let builder = builder
///        .timeout(Duration::from_secs(2))
///        .enable_log(true)
///        .build();
///    pid1::relaunch_if_pid1(builder).expect("Relaunch failed");
///    println!("Hello world");
///    // Rest of the logic
/// }
/// ```
#[cfg(target_family = "unix")]
pub fn relaunch_if_pid1(settings: Pid1Settings) -> Result<(), Error> {
    let pid = std::process::id();
    if pid == 1 {
        let child = relaunch()?;
        if settings.inner.log {
            eprintln!("pid1-rs: Process running as PID 1");
        }
        pid1_handling(settings, child)
    } else {
        if settings.inner.log {
            eprintln!("pid1-rs: Process not running as Pid 1: PID {pid}");
        }
        Ok(())
    }
}

#[cfg(target_family = "windows")]
pub fn relaunch_if_pid1(option: Pid1Settings) -> Result<(), Error> {
    if option.inner.log {
        eprintln!("pid1-rs: PID1 capability not supported for Windows");
    }
    Ok(())
}

/// Settings for Pid1. The [`std::default::Default::default`] setting
/// doesn't log and has a timeout of 2 seconds.
#[derive(Debug, Copy, Clone, Default)]
pub struct Pid1Settings {
    inner: Builder,
}

impl Pid1Settings {
    pub fn builder() -> Builder {
        Builder::default()
    }
}

/// `Builder` acts as builder for initializing `Pid1Settings`.
#[derive(Debug, Copy, Clone)]
pub struct Builder {
    log: bool,
    timeout: Duration,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            log: Default::default(),
            timeout: Duration::from_secs(2),
        }
    }
}

impl Builder {
    /// Initializes the log builder with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Should the crate log to [`std::io::Stderr`]. This can be
    /// useful to detect whether it is running with PID 1. By default
    /// it is 'false'.
    pub fn enable_log(&mut self, enable_log: bool) -> &mut Self {
        self.log = enable_log;
        self
    }

    /// Duration to wait for the child process to exit. By default it
    /// is 2 seconds.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Build `Pid1Settings`.
    pub fn build(self) -> Pid1Settings {
        Pid1Settings { inner: self }
    }
}

#[cfg(target_family = "unix")]
fn relaunch() -> Result<Child, Error> {
    let exe = std::env::current_exe().unwrap();
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    std::process::Command::new(exe)
        .args(args)
        .spawn()
        .map_err(Error::SpawnChild)
}

/// Graceful exit: We dispatch the singal that got to the application,
/// followed by SIGTERM and SIGKILL.
#[cfg(target_family = "unix")]
fn gracefull_exit(settings: Pid1Settings, signal: c_int, child_pid: i32) -> Result<(), Error> {
    if signal == SIGINT {
        let _ = kill(Pid::from_raw(child_pid), Some(nix::sys::signal::SIGINT));
        std::thread::sleep(settings.inner.timeout);
    }
    // Send SIGTERM to the child process
    let _ = kill(Pid::from_raw(child_pid), Some(nix::sys::signal::SIGTERM));
    std::thread::sleep(settings.inner.timeout);

    // Okay, the child process is still present. Use the SIGKILL card.
    let _ = kill(Pid::from_raw(child_pid), Some(nix::sys::signal::SIGKILL));
    Ok(())
}

#[cfg(target_family = "unix")]
fn pid1_handling(settings: Pid1Settings, child: Child) -> ! {
    let mut signals = Signals::new([SIGTERM, SIGINT, SIGCHLD]).unwrap();
    let child = child.id() as i32;
    struct ProcessStatus {
        pid: Pid,
        exit_code: i32,
    }

    enum ShutdownThreadStatus {
        Triggered,
        NotTriggered,
    }

    let mut shutdown_thread = ShutdownThreadStatus::NotTriggered;

    loop {
        for signal in signals.forever() {
            if signal == SIGTERM || signal == SIGINT {
                // We do graceful exit in a separate thread so that
                // pid1 exits as soon as possible
                if let ShutdownThreadStatus::NotTriggered = shutdown_thread {
                    shutdown_thread = ShutdownThreadStatus::Triggered;
                    let _ = std::thread::spawn(move || gracefull_exit(settings, signal, child));
                }
                // We do not exit here since we want the SIGCHLD
                // handler to be invoked appropriately.
            }
            if signal == SIGCHLD {
                let child_process_status = match nix::sys::wait::wait().unwrap() {
                    WaitStatus::Exited(pid, exit_code) => {
                        let process_status = ProcessStatus { pid, exit_code };
                        Some(process_status)
                    }
                    WaitStatus::Signaled(pid, signal, _) => {
                        let process_status = ProcessStatus {
                            pid,
                            // Translate signal to exit code
                            exit_code: signal as i32 + 128,
                        };
                        Some(process_status)
                    }
                    WaitStatus::Stopped(_, _) => None,
                    #[cfg(any(target_os = "linux", target_os = "android"))]
                    WaitStatus::PtraceEvent(_, _, _) => None,
                    #[cfg(any(target_os = "linux", target_os = "android"))]
                    WaitStatus::PtraceSyscall(_) => None,
                    WaitStatus::Continued(_) => None,
                    WaitStatus::StillAlive => None,
                };
                (|| {
                    let child_process = child_process_status?;
                    let child_pid = child_process.pid;
                    let child_pid = child_pid.as_raw();
                    if child_pid == child {
                        // At this point, there could be other
                        // processes running. But once the main
                        // process dies, everything else is guaranteed
                        // to die. So we just exit here with the child's exit status code
                        std::process::exit(child_process.exit_code);
                    }
                    if settings.inner.log {
                        eprintln!("pid1-rs: Reaped PID {child_pid}");
                    }
                    Some(())
                })();
            }
        }
    }
}
