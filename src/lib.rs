use std::process::Child;

use nix::{
    sys::{signal::kill, wait::WaitStatus},
    unistd::Pid,
};
use signal_hook::{
    consts::{SIGCHLD, SIGINT, SIGTERM},
    iterator::Signals,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed when respawning non-PID1 child process: {0}")]
    SpawnChild(std::io::Error),
}

#[cfg(target_family = "unix")]
pub fn relaunch_if_pid1(option: Pid1Settings) -> Result<(), Error> {
    let pid = std::process::id();
    if pid == 1 {
        let child = relaunch()?;
        if option.log {
            eprintln!("pid1-rs: Process running as PID 1");
        }
        pid1_handling(&option, Some(child))
    } else {
        if option.log {
            eprintln!("pid1-rs: Process not running as Pid 1: PID {pid}");
        }
        Ok(())
    }
}

#[cfg(target_family = "windows")]
pub fn relaunch_if_pid1(option: Pid1Settings) -> Result<(), Error> {
    if option.log {
        eprintln!("pid1-rs: PID1 capability not supported for Windows");
    }
    Ok(())
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Pid1Settings {
    pub log: bool,
}

fn relaunch() -> Result<Child, Error> {
    let exe = std::env::current_exe().unwrap();
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    std::process::Command::new(exe)
        .args(args)
        .spawn()
        .map_err(Error::SpawnChild)
}

fn pid1_handling(settings: &Pid1Settings, child: Option<Child>) -> ! {
    let mut signals = Signals::new([SIGTERM, SIGINT, SIGCHLD]).unwrap();
    let child = child.map(|x| x.id());
    struct ProcessStatus {
        pid: Pid,
        exit_code: i32,
    }

    loop {
        for signal in signals.forever() {
            if signal == SIGTERM || signal == SIGINT {
                let exit_code = signal + 128;
                match child {
                    Some(child_pid) => {
                        let pid = Pid::from_raw(child_pid as i32);
                        let nix_signal = if signal == SIGTERM {
                            nix::sys::signal::SIGTERM
                        } else {
                            nix::sys::signal::SIGINT
                        };
                        let result = kill(pid, Some(nix_signal));
                        match result {
                            Ok(()) => std::process::exit(exit_code),
                            Err(errno) => {
                                if settings.log {
                                    eprintln!(
                                        "pid1-rs: kill() failed on {pid} with errno: {errno}"
                                    );
                                }
                                std::process::exit(exit_code)
                            }
                        }
                    }
                    None => std::process::exit(exit_code),
                }
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
                    let child = child?;
                    let child_exit_status = child_process_status?;
                    let child_pid = child_exit_status.pid;
                    let child_pid = u32::try_from(child_pid.as_raw()).ok()?;
                    if child_pid == child {
                        // Propagate child exit status code
                        std::process::exit(child_exit_status.exit_code);
                    }
                    Some(())
                })();
            }
        }
    }
}
