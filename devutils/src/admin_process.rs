//! Spawning processes as Administrator

use std::ffi::{OsStr, OsString};
use std::io::ErrorKind;
use std::os::windows::ffi::OsStrExt;
use std::{io, mem};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

/// A process builder similar to [`std::process::Command`] but spawning processes as Administrator
///
/// The main difference to [`std::process::Command`] is that this uses
/// [`ShellExecuteExW`](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecuteexw) with
/// the `runas` verb, while [`std::process::Command`] uses
/// [`CreateProcessW`](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)
///
/// While [`CreateProcessW`] can capture standard IO but cannot run a process as Administrator, [`ShellExecuteExW`] can
/// run a process as Administrator but cannot capture standard IO.
#[derive(Debug)]
pub(crate) struct Command {
    args: Vec<OsString>,
    program: OsString,
}

impl Command {
    /// Creates a new [`Command`] for launching the program at path `program`
    pub(crate) fn new<S>(program: S) -> Self
    where
        S: AsRef<OsStr>,
    {
        Self {
            args: Vec::new(),
            program: program.as_ref().to_os_string(),
        }
    }

    /// Adds an argument to pass to the program
    pub(crate) fn arg<S>(&mut self, arg: S) -> &mut Self
    where
        S: AsRef<OsStr>,
    {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Adds multiple arguments to pass to the program
    pub(crate) fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }

        self
    }

    /// Executes the program as Administrator as a child process, returning a handle to it
    ///
    /// Note that the standard IO (stdin, stdout, stderr) of the child process cannot be captured.
    pub(crate) fn spawn(&mut self) -> io::Result<Child> {
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

        // SAFETY:
        // The pointer is valid for reads and writes of `SHELLEXECUTEINFOW` because it comes from a live mutable
        // reference
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

/// Representation of a running or exited child process
#[derive(Debug)]
pub(crate) struct Child(HANDLE);

impl Child {
    /// Waits for the child to exit completely, returning the status that it exited with
    pub fn wait(self) -> io::Result<i32> {
        // SAFETY:
        // The handle in the first argument has not been closed
        let result = unsafe { WaitForSingleObject(self.0, u32::MAX) };
        if result == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }

        let mut exit_code = 0u32;

        // SAFETY:
        // - The handle in the first argument has not been closed
        // - The pointer in the second argument is valid for writes of `u32` because it comes from a live mutable
        // reference
        let result = unsafe { GetExitCodeProcess(self.0, &mut exit_code) };
        if !result.as_bool() {
            return Err(io::Error::last_os_error());
        }

        // SAFETY:
        // The handle has not been closed
        let result = unsafe { CloseHandle(self.0) };
        if !result.as_bool() {
            return Err(io::Error::last_os_error());
        }

        Ok(exit_code as i32)
    }
}

/// Encodes the given arguments as a null-terminated "wide" (i.e. potentially ill-formed UTF16-encoded) parameter string
///
/// This has been adapted from the implementation in the standard library used by [`std::process::Command`].
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! make_params_tests {
        (
            $(
                $name:ident: [$($input:literal),*] => [$($expected:literal),*],
            )*
        ) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(
                        make_params::<_, &OsStr>([$(OsStr::new($input)),*]).unwrap(),
                        [$($expected as u16),*],
                    );
                }
            )*
        }
    }

    make_params_tests![
        empty: [] => [0],
        single: ["ab"] => ['"', 'a', 'b', '"', 0],
        multiple: ["ab", "cd", "ef"] => ['"', 'a', 'b', '"', ' ', '"', 'c', 'd', '"', ' ', '"', 'e', 'f', '"', 0],
        quote_escaped: ["a\"b"] => ['"', 'a', '\\', '"', 'b', '"', 0],
        backslash_not_escaped: ["a\\b"] => ['"', 'a', '\\', 'b', '"', 0],
        backslash_quote_both_escaped: ["a\\\"b"] => ['"', 'a', '\\', '\\', '\\', '"', 'b', '"', 0],
        two_backslashes_quote_all_escaped: ["a\\\\\"b"] => ['"', 'a', '\\', '\\', '\\', '\\', '\\', '"', 'b', '"', 0],
        backslash_at_end_escaped: ["ab\\"] => ['"', 'a', 'b', '\\', '\\', '"', 0],
    ];

    #[test]
    fn fail_on_argument_containing_null() {
        assert!(make_params([OsStr::new("a\0b")]).is_err());
    }
}
