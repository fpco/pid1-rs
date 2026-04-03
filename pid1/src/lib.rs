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

const RELAUNCH_GUARD_ENV: &str = "__PID1_RS_RELAUNCHED";

/// The `Error` enum indicates that the [`relaunch_if_pid1`] was not
/// successful.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Failed when respawning of non-PID1 child process
    #[error("Failed when respawning non-PID1 child process: {0}")]
    SpawnChild(std::io::Error),
    /// Could not determine the executable path for relaunch
    #[error("Could not determine executable path for relaunch")]
    ExecutableNotFound,
}

/// Relaunch process as PID with default value of [`Pid1Settings`]
#[cfg(target_family = "unix")]
pub fn relaunch_if_pid1() -> Result<(), Error> {
    Pid1Settings::default().launch()
}

#[cfg(target_family = "windows")]
pub fn relaunch_if_pid1() -> Result<(), Error> {
    Ok(())
}

/// Settings for Pid1. The [`std::default::Default::default`] setting
/// doesn't log and has a timeout of 2 seconds.
#[derive(Debug, Copy, Clone)]
pub struct Pid1Settings {
    log: bool,
    timeout: Duration,
    sub_reaper: bool,
}

impl Pid1Settings {
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

    /// Enable the subreaper feature on Linux (kernel >= 3.4). This is disabled
    /// by default, and is not needed if the process is already running as PID 1.
    /// This ensures that any orphaned descendants of this
    /// process will be reparented to it, rather than to the host's
    /// init process. This is useful for cleaning up process trees,
    /// such as those involving daemonized children.
    pub fn enable_sub_reaper(&mut self, enable: bool) -> &mut Self {
        self.sub_reaper = enable;
        self
    }

    #[allow(clippy::needless_doctest_main)]
    /// When run as PID 1, relaunch the current process as a child process
    /// and do proper signal and zombie reaping in PID 1.
    ///
    /// This function should be the first statement within your main
    /// function.
    ///
    /// # Example
    ///
    /// ```rust,no_test
    /// use std::time::Duration;
    /// use pid1::Pid1Settings;
    ///
    /// fn main() {
    ///     Pid1Settings::new()
    ///         .enable_log(true)
    ///         .timeout(Duration::from_secs(2))
    ///         .launch()
    ///         .expect("Launch failed");
    ///     println!("Hello world");
    ///     // Rest of the logic...
    /// }
    /// ```
    ///
    /// Note that this function is only applicable for Unix
    /// systems. For Windows, it will return [`Ok(())`].
    #[cfg(target_family = "unix")]
    pub fn launch(self) -> Result<(), Error> {
        // This environment variable is used to prevent the relaunched
        // child process from re-initializing pid1 logic.
        if std::env::var_os(RELAUNCH_GUARD_ENV).is_some() {
            return Ok(());
        }

        let pid = std::process::id();
        if pid == 1 {
            // Install signal handler before we launch child process
            let signals = Signals::new([SIGTERM, SIGINT, SIGCHLD]).unwrap();
            let child = relaunch()?;
            if self.log {
                eprintln!("pid1-rs: Process running as PID 1");
            }
            pid1_handling(self, signals, child)
        } else {
            #[cfg(target_os = "linux")]
            if self.sub_reaper {
                // Set the subreaper flag. This ensures that any orphaned descendants
                // of this process will be reparented to it, rather than to the
                // host's init process. This is crucial for cleaning up complex
                // process trees and daemonized children.
                if let Err(e) = nix::sys::prctl::set_child_subreaper(true) {
                    if self.log {
                        eprintln!("pid1-rs: Could not set subreaper: {e}");
                    }
                }
                // Start a background thread to reap adopted children.
                std::thread::spawn(move || subreaper_thread(self));
            } else if self.log {
                eprintln!(
                    "pid1-rs: Warning: process is not PID 1 and subreaper is not enabled. \
                     Orphaned processes may not be reaped."
                );
            }
            Ok(())
        }
    }
    #[cfg(target_family = "windows")]
    pub fn launch(self) -> Result<(), Error> {
        if self.log {
            eprintln!("pid1-rs: PID1 capability not supported for Windows");
        }
        Ok(())
    }

    /// Do proper reaping and signal handling on the [`Child`]
    /// process. This method is only available for Unix systems.
    #[cfg(target_family = "unix")]
    pub fn pid1_handling(self, signals: Signals, child: Child) -> ! {
        pid1_handling(self, signals, child)
    }
}

impl Default for Pid1Settings {
    fn default() -> Self {
        Self {
            log: Default::default(),
            timeout: Duration::from_secs(2),
            sub_reaper: false,
        }
    }
}

#[cfg(target_family = "unix")]
fn relaunch() -> Result<Child, Error> {
    let mut args = std::env::args_os();
    let exe = args.next().ok_or(Error::ExecutableNotFound)?;
    std::process::Command::new(exe)
        .args(args)
        .env(RELAUNCH_GUARD_ENV, "1")
        .spawn()
        .map_err(Error::SpawnChild)
}

// New struct to hold process status
#[cfg(target_family = "unix")]
struct ProcessStatus {
    pid: Pid,
    exit_code: i32,
}

/// The reap_zombies function reaps all available zombie processes.
///
/// It returns an Option containing the exit code of the main child if it exited.
#[cfg(target_family = "unix")]
fn reap_zombies(settings: Pid1Settings, main_child_pid: Option<i32>) -> Option<i32> {
    let mut main_child_exit_code = None;

    // Multiple child processes can exit in quick succession, but the
    // operating system may only deliver a single SIGCHLD signal.
    // This is known as signal coalescing. To handle this, we loop
    // with a non-blocking `waitpid` call to reap all zombies.
    // Using a blocking `wait` would hang if there are no more
    // children to reap, preventing us from handling other signals.
    // Reference: https://stackoverflow.com/a/8398491/1651941
    loop {
        let wait_status = match nix::sys::wait::waitpid(
            None,
            Some(nix::sys::wait::WaitPidFlag::WNOHANG),
        ) {
            Ok(status) => status,
            Err(nix::errno::Errno::ECHILD) => {
                // No more children to wait for
                break;
            }
            Err(e) => {
                if settings.log {
                    eprintln!("pid1-rs: Error in waitpid: {e}");
                }
                break;
            }
        };

        let child_process_status = match wait_status {
            WaitStatus::Exited(pid, exit_code) => Some(ProcessStatus { pid, exit_code }),
            WaitStatus::Signaled(pid, signal, _) => {
                // Translate signal to exit code
                let exit_code = signal as i32 + 128;
                Some(ProcessStatus { pid, exit_code })
            }
            WaitStatus::StillAlive => {
                // No more children to reap now
                break;
            }
            WaitStatus::Stopped(..) => None,
            #[cfg(any(target_os = "linux", target_os = "android"))]
            WaitStatus::PtraceEvent(..) | WaitStatus::PtraceSyscall(..) => None,
            WaitStatus::Continued(..) => None,
        };

        if let Some(child_process) = child_process_status {
            let child_pid = child_process.pid.as_raw();
            if let Some(main_child_pid) = main_child_pid {
                if child_pid == main_child_pid {
                    // Main child has exited. We'll exit with its status code,
                    // but only after reaping any other children that may have
                    // exited in this same signal batch.
                    main_child_exit_code = Some(child_process.exit_code);
                }
            }
            if settings.log {
                eprintln!("pid1-rs: Reaped PID {child_pid}");
            }
        }
    }
    main_child_exit_code
}

#[cfg(target_family = "unix")]
fn subreaper_thread(settings: Pid1Settings) {
    let mut signals = Signals::new([SIGCHLD]).unwrap();
    for _ in signals.forever() {
        // We don't have a "main child" in this context, so we pass None.
        reap_zombies(settings, None);
    }
}

/// Graceful exit: We dispatch the singal that got to the application,
/// followed by SIGTERM and SIGKILL.
#[cfg(target_family = "unix")]
fn graceful_exit(settings: Pid1Settings, signal: c_int, child_pid: i32) -> Result<(), Error> {
    if signal == SIGINT {
        let _ = kill(Pid::from_raw(child_pid), Some(nix::sys::signal::SIGINT));
        std::thread::sleep(settings.timeout);
    }
    // Send SIGTERM to the child process
    let _ = kill(Pid::from_raw(child_pid), Some(nix::sys::signal::SIGTERM));
    std::thread::sleep(settings.timeout);

    // Okay, the child process is still present. Use the SIGKILL card.
    let _ = kill(Pid::from_raw(child_pid), Some(nix::sys::signal::SIGKILL));
    Ok(())
}

#[cfg(target_family = "unix")]
fn pid1_handling(settings: Pid1Settings, mut signals: Signals, child: Child) -> ! {
    let child_pid = child.id() as i32;

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
                    let _ = std::thread::spawn(move || graceful_exit(settings, signal, child_pid));
                }
                // We do not exit here since we want the SIGCHLD
                // handler to be invoked appropriately.
            }
            if signal == SIGCHLD {
                if let Some(exit_code) = reap_zombies(settings, Some(child_pid)) {
                    std::process::exit(exit_code);
                }
            }
        }
    }
}
