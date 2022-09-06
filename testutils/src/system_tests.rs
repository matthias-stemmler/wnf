pub extern crate libtest_mimic;

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{stdout, ErrorKind, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs, io, process};
use std::{mem, str};

use libtest_mimic::{Arguments, Trial};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, MAX_PATH, WAIT_FAILED};
use windows::Win32::Storage::FileSystem::{GetTempFileNameW, GetTempPathW};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows::Win32::System::WindowsProgramming::INFINITE;
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

#[macro_export]
macro_rules! system_tests {
    ($($test_name:ident),*) => {
        $crate::system_tests![$($test_name,)*];
    };

    ($($test_name:ident,)*) => {
        fn main() -> ::std::process::ExitCode {
            $(
                fn $test_name() -> ::std::result::Result<(), $crate::system_tests::libtest_mimic::Failed> {
                    self::$test_name();
                    ::std::result::Result::Ok(())
                }
            )*

            $crate::system_tests::test_main(::std::vec![
                $(
                    $crate::system_tests::libtest_mimic::Trial::test(
                        stringify!($test_name),
                        $test_name
                    )
                    .with_kind("system")
                ),*
            ])
        }
    };
}

pub fn test_main(tests: Vec<Trial>) -> ExitCode {
    if let Err(err) = run_tests_as_system(tests) {
        eprintln!("Failed to run tests as system: {err}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_tests_as_system(tests: Vec<Trial>) -> io::Result<()> {
    let system_runner = SystemRunner::from_args();
    let run_system_tests = true; // TODO take from env variable

    let tests = if run_system_tests {
        system_runner.ensure_running_as_system()?;
        tests
    } else {
        tests.into_iter().map(|test| test.with_ignored_flag(true)).collect()
    };

    libtest_mimic::run(&Arguments::from_iter(system_runner.args()), tests).exit()
}

struct SystemRunner {
    args: Vec<String>,
    is_running_as_system: bool,
}

impl SystemRunner {
    const RUNNING_AS_SYSTEM: &'static str = "__RUNNING_AS_SYSTEM";

    fn from_args() -> Self {
        let (running_as_system, args) = env::args().partition(|arg| arg == Self::RUNNING_AS_SYSTEM);

        Self {
            args,
            is_running_as_system: !running_as_system.is_empty(),
        }
    }

    fn args(&self) -> &[String] {
        &self.args
    }

    fn ensure_running_as_system(&self) -> io::Result<()> {
        if !self.is_running_as_system {
            reexecute_as_system(Self::RUNNING_AS_SYSTEM)?;
        }

        Ok(())
    }
}

fn reexecute_as_system(marker_arg: &str) -> io::Result<()> {
    let current_exe = env::current_exe()?;
    let current_exe_path = current_exe.display();

    let stdout_file = TempFile::new()?;
    let stdout_file_path = stdout_file.path().display();

    let args = env::args().skip(1).collect::<Vec<_>>().join(" ");

    // TODO take from env variable, falling back to 'psexec' (-> $PATH)
    let psexec_path = "c:\\tools\\pstools\\psexec.exe";

    let verb = CWideString::new("runas");
    let file = CWideString::new("cmd");
    let params = CWideString::new(&format!(
        "/c {psexec_path} -s {current_exe_path} {marker_arg} {args} >{stdout_file_path}"
    ));

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
        return Err(io::Error::new(
            ErrorKind::Other,
            format!("Failed to start psexec at {psexec_path}"),
        ));
    }

    let result = unsafe { WaitForSingleObject(shell_execute_info.hProcess, INFINITE) };
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

    match exit_code {
        0 | 101 => {
            stdout_file.read_to(&mut stdout())?;
            process::exit(exit_code as i32)
        }
        _ => Err(io::Error::new(
            ErrorKind::Other,
            format!("Failed to start psexec at {psexec_path}"),
        )),
    }
}

struct TempFile(PathBuf);

impl TempFile {
    fn new() -> io::Result<Self> {
        get_temp_filename().map(Self)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn read_to<W>(self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        io::copy(&mut File::open(&self.0)?, writer)?;
        Ok(())
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn get_temp_filename() -> io::Result<PathBuf> {
    let mut temp_path = [0u16; MAX_PATH as usize];
    let result = unsafe { GetTempPathW(&mut temp_path) };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut temp_filename = [0u16; MAX_PATH as usize];
    let result = unsafe { GetTempFileNameW(PCWSTR::from_raw(temp_path.as_ptr()), None, 0, &mut temp_filename) };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(OsString::from_wide(&temp_filename.split(|c| *c == 0).next().unwrap()).into())
}

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
