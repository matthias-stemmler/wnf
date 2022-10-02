use std::ffi::OsStr;
use std::io::{stderr, stdout, ErrorKind};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, io, iter, mem, process};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, WAIT_FAILED};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

use crate::temp_file::TempFile;

const ARG_PREFIX: &str = "__SYSTEM_RUNNER";
const ARG_MARKER_ADMIN: &str = "ADMIN";
const ARG_MARKER_SYSTEM: &str = "SYSTEM";
const ARG_SEPARATOR: &str = "::";

#[derive(Debug)]
enum SystemRunnerStage {
    Initial,
    RunningAsAdmin {
        stdout_file_path: PathBuf,
        stderr_file_path: PathBuf,
    },
    RunningAsSystem,
}

#[derive(Debug)]
pub struct SystemRunner {
    args: Vec<String>,
    stage: SystemRunnerStage,
}

impl SystemRunner {
    pub fn from_args() -> io::Result<Self> {
        let prefix = format!("{ARG_PREFIX}{ARG_SEPARATOR}");
        let (system_runner_args, args) = env::args().partition(|arg| arg.starts_with(&prefix));

        Ok(Self {
            args,

            stage: {
                let mut system_runner_args = system_runner_args.iter().filter_map(|arg| arg.strip_prefix(&prefix));

                match system_runner_args.next() {
                    None => SystemRunnerStage::Initial,

                    Some(arg) if system_runner_args.next().is_none() => {
                        let mut parts = arg.split(ARG_SEPARATOR);

                        match (parts.next(), parts.next(), parts.next()) {
                            (Some(ARG_MARKER_ADMIN), Some(stdout_file_path), Some(stderr_file_path)) => {
                                SystemRunnerStage::RunningAsAdmin {
                                    stdout_file_path: stdout_file_path.into(),
                                    stderr_file_path: stderr_file_path.into(),
                                }
                            }

                            (Some(ARG_MARKER_SYSTEM), None, None) => SystemRunnerStage::RunningAsSystem,

                            _ => return Err(io::Error::new(ErrorKind::Other, "Invalid system runner argument")),
                        }
                    }

                    _ => {
                        return Err(io::Error::new(
                            ErrorKind::Other,
                            "Found more than one system runner argument",
                        ))
                    }
                }
            },
        })
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn ensure_running_as_system(&self) -> io::Result<()> {
        match &self.stage {
            SystemRunnerStage::Initial => self.reexecute_as_admin()?,

            SystemRunnerStage::RunningAsAdmin {
                stdout_file_path,
                stderr_file_path,
            } => {
                if let Err(err) = self.reexecute_as_system(stdout_file_path) {
                    let _ = fs::write(stderr_file_path, format!("{err}\n"));
                }
            }

            _ => (),
        }

        Ok(())
    }

    fn reexecute_as_admin(&self) -> io::Result<()> {
        let mut stdout_file = TempFile::new()?;
        let mut stderr_file = TempFile::new()?;

        let exit_code = elevate_self(
            iter::once(format!(
                "{ARG_PREFIX}{ARG_SEPARATOR}{ARG_MARKER_ADMIN}{ARG_SEPARATOR}{}{ARG_SEPARATOR}{}",
                stdout_file.path().display(),
                stderr_file.path().display()
            ))
            .chain(self.args.iter().skip(1).cloned()),
        )?;

        stdout_file.read_to(&mut stdout())?;
        stderr_file.read_to(&mut stderr())?;

        process::exit(exit_code);
    }

    fn reexecute_as_system(&self, stdout_file_path: &Path) -> io::Result<()> {
        let output = Command::new(concat!(env!("CARGO_MANIFEST_DIR"), "/thirdparty/paexec.exe"))
            .arg("-s")
            .arg(&env::current_exe()?)
            .arg(format!("{ARG_PREFIX}{ARG_SEPARATOR}{ARG_MARKER_SYSTEM}"))
            .args(self.args.iter().skip(1))
            .output()?;

        fs::write(stdout_file_path, output.stdout)?;

        let exit_code = output.status.code().unwrap();
        if exit_code < 0 {
            return Err(io::Error::new(ErrorKind::Other, "PAExec failed"));
        }

        process::exit(exit_code);
    }
}

fn elevate_self(args: impl IntoIterator<Item = String>) -> io::Result<i32> {
    let verb = CWideString::new("runas");
    let file = CWideString::new(&env::current_exe()?);

    let params = CWideString::new(
        &args
            .into_iter()
            .map(|arg| format!(r#""{arg}""#))
            .collect::<Vec<_>>()
            .join(" "),
    );

    let mut shell_execute_info = SHELLEXECUTEINFOW {
        cbSize: mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: verb.as_pcwstr(),
        lpFile: file.as_pcwstr(),
        lpParameters: params.as_pcwstr(),
        ..Default::default()
    };

    let result = unsafe { ShellExecuteExW(&mut shell_execute_info) };
    if !result.as_bool() {
        return Err(io::Error::last_os_error());
    }

    if shell_execute_info.hProcess.is_invalid() {
        return Err(io::Error::new(ErrorKind::Other, "Failed to elevate"));
    }

    let result = unsafe { WaitForSingleObject(shell_execute_info.hProcess, u32::MAX) };
    if result == WAIT_FAILED.0 {
        return Err(io::Error::last_os_error());
    }

    let mut exit_code = 0u32;
    let result = unsafe { GetExitCodeProcess(shell_execute_info.hProcess, &mut exit_code) };
    if !result.as_bool() {
        return Err(io::Error::last_os_error());
    }

    let result = unsafe { CloseHandle(shell_execute_info.hProcess) };
    if !result.as_bool() {
        return Err(io::Error::last_os_error());
    }

    Ok(exit_code as i32)
}

#[derive(Debug)]
struct CWideString(Vec<u16>);

impl CWideString {
    fn new<S>(s: &S) -> Self
    where
        S: AsRef<OsStr> + ?Sized,
    {
        Self(OsStr::new(s).encode_wide().chain(Some(0)).collect())
    }

    fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.0.as_ptr())
    }
}
