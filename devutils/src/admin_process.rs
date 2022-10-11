use std::ffi::{OsStr, OsString};
use std::io::ErrorKind;
use std::os::windows::ffi::OsStrExt;
use std::{io, mem};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

#[derive(Debug)]
pub(crate) struct Command {
    args: Vec<OsString>,
    program: OsString,
}

impl Command {
    pub(crate) fn new<S>(program: S) -> Self
    where
        S: AsRef<OsStr>,
    {
        Self {
            args: Vec::new(),
            program: program.as_ref().to_os_string(),
        }
    }

    pub fn arg<S>(&mut self, arg: S) -> &mut Self
    where
        S: AsRef<OsStr>,
    {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }

        self
    }

    pub fn spawn(&mut self) -> io::Result<Child> {
        let verb: Vec<u16> = OsStr::new("runas").encode_wide().chain(Some(0)).collect();
        let file: Vec<u16> = OsStr::new(&self.program).encode_wide().chain(Some(0)).collect();
        let params = make_params(&self.args)?;

        let mut shell_execute_info = SHELLEXECUTEINFOW {
            cbSize: mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR::from_raw(verb.as_ptr()),
            lpFile: PCWSTR::from_raw(file.as_ptr()),
            lpParameters: PCWSTR::from_raw(params.as_ptr()),
            ..Default::default()
        };

        let result = unsafe { ShellExecuteExW(&mut shell_execute_info) };
        if !result.as_bool() {
            return Err(io::Error::last_os_error());
        }

        if shell_execute_info.hProcess.is_invalid() {
            return Err(io::Error::new(ErrorKind::Other, "failed to spawn admin process"));
        }

        Ok(Child(shell_execute_info.hProcess))
    }
}

#[derive(Debug)]
pub(crate) struct Child(HANDLE);

impl Child {
    pub fn wait(self) -> io::Result<i32> {
        let result = unsafe { WaitForSingleObject(self.0, u32::MAX) };
        if result == WAIT_FAILED.0 {
            return Err(io::Error::last_os_error());
        }

        let mut exit_code = 0u32;
        let result = unsafe { GetExitCodeProcess(self.0, &mut exit_code) };
        if !result.as_bool() {
            return Err(io::Error::last_os_error());
        }

        let result = unsafe { CloseHandle(self.0) };
        if !result.as_bool() {
            return Err(io::Error::last_os_error());
        }

        Ok(exit_code as i32)
    }
}

fn make_params<I, S>(args: I) -> io::Result<Vec<u16>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut params = Vec::new();

    for (idx, arg) in args.into_iter().enumerate() {
        if idx > 0 {
            params.push(' ' as u16);
        }

        params.push('"' as u16);
        let mut backslashes = 0;

        for x in arg.as_ref().encode_wide() {
            if x == 0 {
                return Err(io::Error::new(ErrorKind::InvalidInput, "argument contains NULL byte"));
            }

            if x == '\\' as u16 {
                backslashes += 1;
            } else {
                if x == '"' as u16 {
                    params.extend((0..=backslashes).map(|_| '\\' as u16))
                }

                backslashes = 0;
            }

            params.push(x);
        }

        params.extend((0..backslashes).map(|_| '\\' as u16));
        params.push('"' as u16);
    }

    params.push(0);
    Ok(params)
}
