use clap::Parser;
#[cfg(target_family = "unix")]
use pid1::Pid1Settings;
#[cfg(target_family = "unix")]
use signal_hook::{
    consts::{SIGCHLD, SIGINT, SIGTERM},
    iterator::Signals,
};
#[cfg(target_family = "unix")]
use std::os::unix::process::CommandExt;
#[cfg(target_family = "unix")]
use std::time::Duration;
use std::{error::Error, ffi::OsString, path::PathBuf};

#[derive(Parser, Debug, PartialEq)]
pub(crate) struct Pid1App {
    /// Specify working direcory
    #[arg(short, long, value_name = "DIR")]
    pub(crate) workdir: Option<PathBuf>,
    /// Timeout (in seconds) to wait for child proess to exit
    #[arg(short, long, value_name = "TIMEOUT", default_value_t = 2)]
    pub(crate) timeout: u8,
    /// Turn on verbose output
    #[arg(short, long, default_value_t = false)]
    pub(crate) verbose: bool,
    /// Override environment variables. Can specify multiple times.
    #[arg(short, long, value_parser=parse_key_val::<OsString, OsString>)]
    pub(crate) env: Vec<(OsString, OsString)>,
    /// Process arguments
    #[arg(required = true)]
    child_process: Vec<String>,
}

impl Pid1App {
    #[cfg(target_family = "unix")]
    pub(crate) fn run(self) -> ! {
        let mut child = std::process::Command::new(&self.child_process[0]);
        let child = child.args(&self.child_process[1..]);
        if let Some(workdir) = &self.workdir {
            child.current_dir(workdir);
        }
        for (key, value) in &self.env {
            child.env(key, value);
        }
        let pid = std::process::id();
        if pid != 1 {
            let status = child.exec();
            eprintln!("execvp failed with: {status:?}");

            std::process::exit(1);
        } else {
            // Install signal handlers before launching child process
            let signals = Signals::new([SIGTERM, SIGINT, SIGCHLD]).unwrap();
            let child = child.spawn();
            let child = match child {
                Ok(child) => child,
                Err(err) => {
                    eprintln!(
                        "pid1: {} spawn failed. Got error: {err}",
                        self.child_process[0]
                    );
                    std::process::exit(1);
                }
            };

            Pid1Settings::new()
                .enable_log(self.verbose)
                .timeout(Duration::from_secs(self.timeout.into()))
                .pid1_handling(signals, child)
        }
    }

    #[cfg(target_family = "windows")]
    pub(crate) fn run(self) -> ! {
        eprintln!("pid1: Not supported on Windows");
        std::process::exit(1);
    }
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}
