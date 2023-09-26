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

pub fn relaunch_if_pid1(option: Pid1Settings) -> Result<(), Error> {
    let pid = std::process::id();
    if pid == 1 {
        let child = relaunch()?;
        if option.log {
            println!("pid1-rs: Process running as PID 1");
        }
        pid1_handling(Some(child))
    } else {
        if option.log {
            eprintln!("pid1-rs: Process not running as Pid 1: PID {pid}");
        }
        Ok(())
    }
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

fn pid1_handling(child: Option<Child>) -> ! {
    let mut signals = Signals::new([SIGTERM, SIGINT, SIGCHLD]).unwrap();
    let child = child.map(|x| x.id());
    struct ExitStatus {
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
                            Err(_) => graceful_shutdown(),
                        }
                    }
                    None => std::process::exit(exit_code),
                }
            }
            if signal == SIGCHLD {
                let pid = match nix::sys::wait::wait().unwrap() {
                    WaitStatus::Exited(pid, exit_code) => {
                        let exit_status = ExitStatus { pid, exit_code };
                        Some(exit_status)
                    }
                    WaitStatus::Signaled(pid, signal, _) => {
                        let exit_status = ExitStatus {
                            pid,
                            // Translate signal to exit code
                            exit_code: signal as i32 + 128,
                        };
                        Some(exit_status)
                    }
                    WaitStatus::Stopped(_, _) => None,
                    WaitStatus::PtraceEvent(_, _, _) => None,
                    WaitStatus::PtraceSyscall(_) => None,
                    WaitStatus::Continued(_) => None,
                    WaitStatus::StillAlive => None,
                };
                (|| {
                    let child = child?;
                    let child_exit_status = pid?;
                    let pid = child_exit_status.pid;
                    let pid = u32::try_from(pid.as_raw()).ok()?;
                    if pid == child {
                        // Propagate child exit status code
                        std::process::exit(child_exit_status.exit_code);
                    }
                    Some(())
                })();
            }
        }
    }
}

fn graceful_shutdown() -> ! {
    // FIXME the name is a lie
    std::process::abort();
}
