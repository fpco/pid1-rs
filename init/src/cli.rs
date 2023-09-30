use std::{
    collections::HashMap,
    ffi::{CString, NulError, OsStr, OsString},
    path::PathBuf,
    time::Duration,
};

use clap_lex::{OsStrExt, RawArgs};
use pid1::Pid1Settings;

const HELP: &str = "\
pid1

USAGE:
  pid1 [OPTIONS] command [args]

FLAGS:
  -h, --help              Prints help information

OPTIONS:
  -e, --env ENV           Override environment variables. Can be specified multiple times
  -w, --workdir DIR       Command working directory
  -t, --timeout TIMEOUT   Timeout (in seconds) to wait for child process to exit
  -v, --verbose           Turn on verbose output

ARGS:
  <INPUT>

EXAMPLES:

pid1 --workdir=/home/user ls /
pid1 -w/home/sibi ls /
pid1 --env=HELLO=WORLD --env=FOO=BAR printenv";

type BoxedError = Box<dyn std::error::Error>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Args {
    pub(crate) workdir: Option<PathBuf>,
    pub(crate) timeout: u8,
    pub(crate) child_process: Option<String>,
    pub(crate) child_args: Vec<String>,
    pub(crate) show_help: bool,
    pub(crate) verbose: bool,
    pub(crate) override_env: HashMap<OsString, OsString>,
}

fn to_env_pair(env: &OsStr) -> Result<(&OsStr, &OsStr), BoxedError> {
    let env_value = env.split_once("=");
    match env_value {
        None => {
            Err(format!("Environment variable not separated by delimter =. Got {env:?}").into())
        }
        Some((key, value)) => Ok((key, value)),
    }
}

pub(crate) fn parse_args(
    raw: impl IntoIterator<Item = impl Into<std::ffi::OsString>>,
) -> Result<Args, BoxedError> {
    let mut args = Args {
        workdir: None,
        timeout: 2,
        child_process: None,
        child_args: vec![],
        show_help: false,
        verbose: false,
        override_env: HashMap::new(),
    };
    let raw = RawArgs::new(raw);
    let mut cursor = raw.cursor();
    let mut seen_child_process = false;
    while let Some(arg) = raw.next(&mut cursor) {
        if !seen_child_process {
            if let Some((long, value)) = arg.to_long() {
                match long {
                    Ok("help") => match value {
                        Some(value) => {
                            return Err(
                                format!("--help doesn't take value. Passed {value:?}.").into()
                            )
                        }
                        None => {
                            args.show_help = true;
                            return Ok(args);
                        }
                    },
                    Ok("env") => match value {
                        Some(env_value) => {
                            let (key_env, value_env) = to_env_pair(env_value)?;
                            args.override_env
                                .insert(key_env.to_os_string(), value_env.to_os_string());
                        }
                        None => return Err("--env takes value. No value passed.".into()),
                    },
                    Ok("workdir") => match value {
                        Some(dir) => {
                            args.workdir = Some(PathBuf::from(dir));
                        }
                        None => return Err("--workdir takes directory. No value passed.".into()),
                    },
                    Ok("timeout") => match value {
                        Some(timeout_arg) => {
                            let timeout = timeout_arg.to_str();
                            let timeout = match timeout {
                                Some(timeout) => timeout,
                                None => {
                                    return Err(
                                        format!("Invalid timeout {timeout_arg:?} passed").into()
                                    )
                                }
                            };
                            let timeout = str::parse(timeout)?;
                            args.timeout = timeout;
                        }
                        None => return Err("--timeout takes seconds. No value passed.".into()),
                    },
                    _ => return Err(format!("Unexpected flag: --{}", arg.display()).into()),
                }
            } else if let Some(mut shorts) = arg.to_short() {
                while let Some(short) = shorts.next_flag() {
                    match short {
                        Ok('h') => {
                            let value = shorts.next_value_os();
                            match value {
                                None => {
                                    args.show_help = true;
                                    return Ok(args);
                                }
                                Some(help_value) => {
                                    return Err(format!(
                                        "-h doesn't take value. Passed {help_value:?}."
                                    )
                                    .into())
                                }
                            }
                        }
                        Ok('e') => {
                            let value = shorts.next_value_os();
                            match value {
                                Some(env_value) => {
                                    let (key_env, value_env) = to_env_pair(env_value)?;
                                    args.override_env
                                        .insert(key_env.to_os_string(), value_env.to_os_string());
                                }
                                None => return Err("No environment variables passed".into()),
                            }
                        }
                        Ok('w') => {
                            let value = shorts.next_value_os();
                            match value {
                                Some(dir) => {
                                    args.workdir = Some(PathBuf::from(dir));
                                }
                                None => return Err("No working directory passed".into()),
                            }
                        }
                        Ok('t') => {
                            let timeout_arg = shorts.next_value_os();
                            match timeout_arg {
                                Some(timeout_arg) => {
                                    let timeout_opt = timeout_arg.to_str();
                                    match timeout_opt {
                                        Some(timeout) => {
                                            let timeout = str::parse(timeout)?;
                                            args.timeout = timeout;
                                        }
                                        None => {
                                            return Err(format!(
                                                "Invalid timeout {timeout_arg:?} passed"
                                            )
                                            .into())
                                        }
                                    }
                                }
                                None => return Err("No timeout passed".into()),
                            }
                        }
                        _ => return Err(format!("Unexpected flag: -{short:?}").into()),
                    }
                }
            } else {
                seen_child_process = true;
                let value = arg
                    .to_value_os()
                    .to_owned()
                    .into_string()
                    .map_err(|o| format!("Invalid argument: {o:?}"))?;
                args.child_process = Some(value.clone());
                args.child_args.push(value);
            }
        } else {
            let value = arg
                .to_value_os()
                .to_owned()
                .into_string()
                .map_err(|o| format!("Invalid argument: {o:?}"))?;
            args.child_args.push(value)
        }
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::parse_args;
    use super::Args;

    #[test]
    fn test_parse_long_flag() {
        let arg = parse_args(["--workdir=/home/sibi", "ls"]).unwrap();
        assert_eq!(
            arg,
            Args {
                workdir: Some(PathBuf::from("/home/sibi")),
                timeout: 2,
                child_process: Some("ls".into()),
                child_args: vec!["ls".into()],
                show_help: false,
                verbose: false,
                override_env: HashMap::new()
            }
        );
    }

    #[test]
    fn test_parse_short_flag() {
        let arg = parse_args(["-w/home/sibi", "ls"]).unwrap();
        assert_eq!(
            arg,
            Args {
                workdir: Some(PathBuf::from("/home/sibi")),
                timeout: 2,
                child_process: Some("ls".into()),
                child_args: vec!["ls".into()],
                show_help: false,
                verbose: false,
                override_env: HashMap::new()
            }
        );
    }

    #[test]
    fn test_parse_timeout() {
        let arg = parse_args(["--workdir=/home/sibi", "--timeout=3", "ls", "/", "java"]).unwrap();
        assert_eq!(
            arg,
            Args {
                workdir: Some(PathBuf::from("/home/sibi")),
                timeout: 3,
                child_process: Some("ls".into()),
                child_args: vec!["ls".into(), "/".into(), "java".into()],
                show_help: false,
                verbose: false,
                override_env: HashMap::new()
            }
        );
    }

    #[test]
    fn mix_short_long_flag() {
        let arg = parse_args(["--workdir=/home/sibi", "-t3", "ls", "/", "java"]).unwrap();
        assert_eq!(
            arg,
            Args {
                workdir: Some(PathBuf::from("/home/sibi")),
                timeout: 3,
                child_process: Some("ls".into()),
                child_args: vec!["ls".into(), "/".into(), "java".into()],
                show_help: false,
                verbose: false,
                override_env: HashMap::new()
            }
        );
    }

    #[test]
    fn long_help_flag() {
        let arg = parse_args(["--help", "--workdir=/home/sibi", "-t3", "ls", "/", "java"]).unwrap();
        assert_eq!(
            arg,
            Args {
                workdir: None,
                timeout: 2,
                child_process: None,
                child_args: vec![],
                show_help: true,
                verbose: false,
                override_env: HashMap::new()
            }
        );
    }

    #[test]
    fn short_help_flag() {
        let arg = parse_args(["-h", "--workdir=/home/sibi", "-t3", "ls", "/", "java"]).unwrap();
        assert_eq!(
            arg,
            Args {
                workdir: None,
                timeout: 2,
                child_process: None,
                child_args: vec![],
                show_help: true,
                verbose: false,
                override_env: HashMap::new()
            }
        );
    }

    #[test]
    fn short_help_with_value() {
        let arg = parse_args(["-hfake", "--workdir=/home/sibi", "-t3", "ls", "/", "java"]);
        assert!(arg.is_err());
    }
}

fn print_help(exit_code: i32) -> ! {
    if exit_code == 0 {
        print!("{HELP}");
    } else {
        eprint!("{HELP}");
    }
    std::process::exit(exit_code);
}

struct ExecParams {
    path: CString,
    args: Vec<CString>,
}

fn to_exec_params(path: String, args: Vec<String>) -> Result<ExecParams, NulError> {
    let path = CString::new(path)?;
    let args = args
        .into_iter()
        .map(CString::new)
        .collect::<Result<Vec<CString>, NulError>>();
    let args = args?;
    Ok(ExecParams { path, args })
}

pub(crate) fn handle_arg(arg: Args) {
    if arg.show_help {
        print_help(0);
    }
    let child_process = match arg.child_process.clone() {
        Some(path) => path,
        None => print_help(0),
    };

    if let Some(workdir) = arg.workdir {
        let result = std::env::set_current_dir(&workdir);
        if let Err(error) = result {
            eprintln!(
                "Failed changing current working dir to {workdir:?} with {:?}",
                error.raw_os_error()
            );
            std::process::exit(1);
        }
    }

    for (key, value) in arg.override_env {
        std::env::set_var(key, value);
    }

    let pid = std::process::id();
    if pid != 1 {
        // todo: fix env
        let exec_params = to_exec_params(child_process, arg.child_args);
        let exec_params = match exec_params {
            Ok(exec_params) => exec_params,
            Err(nul_error) => {
                eprintln!("Error while marshaling. Got error: {nul_error}");
                std::process::exit(1);
            }
        };

        let status = nix::unistd::execvp(&exec_params.path, &exec_params.args[..]);
        eprintln!("Impossible: execvp failed with: {status:?}");
    } else {
        let child = std::process::Command::new(child_process)
            .args(&arg.child_args[1..])
            .spawn()
            .expect("Spawn failed");
        Pid1Settings::new()
            .enable_log(arg.verbose)
            .timeout(Duration::from_secs(arg.timeout.into()))
            .pid1_handling(child)
    }
}
