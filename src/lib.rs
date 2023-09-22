use std::process::Child;

use nix::sys::wait::WaitStatus;
use signal_hook::{
    consts::{SIGCHLD, SIGINT, SIGTERM},
    iterator::Signals,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed when respawning non-PID1 child process: {0}")]
    SpawnChild(std::io::Error),
}

pub fn relaunch_if_pid1() -> Result<(), Error> {
    if std::process::id() == 1 {
        let child = relaunch()?;
        pid1_handling(Some(child))
    } else {
        Ok(())
    }
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
    let child = child.map(|x| x.id());
    let mut signals = Signals::new(&[SIGTERM, SIGINT, SIGCHLD]).unwrap();
    loop {
        for signal in signals.pending() {
            if signal == SIGTERM || signal == SIGINT {
                graceful_shutdown();
            }
            if signal == SIGCHLD {
                let pid = match nix::sys::wait::wait().unwrap() {
                    WaitStatus::Exited(pid, _) => Some(pid),
                    WaitStatus::Signaled(pid, _, _) => Some(pid),
                    WaitStatus::Stopped(_, _) => None,
                    WaitStatus::PtraceEvent(_, _, _) => None,
                    WaitStatus::PtraceSyscall(_) => None,
                    WaitStatus::Continued(_) => None,
                    WaitStatus::StillAlive => None,
                };
                (|| {
                    let child = child?;
                    let pid = pid?;
                    let pid = u32::try_from(pid.as_raw()).ok()?;
                    if pid == child {
                        graceful_shutdown()
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
