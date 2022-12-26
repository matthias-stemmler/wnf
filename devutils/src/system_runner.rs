//! Utilities for rerunning the current program under the `LocalSystem` account
//!
//! This is useful both in integration tests (see the [`system_tests`] module) and in examples.
//!
//! It is achieved by running the current program multiple times in "stages":
//! - "Initial" stage: The original invocation of the program
//! - "Admin" stage: Running as Administrator (through the Windows API function `ShellExecuteExW` with the `runas` verb)
//! - "System" stage: Running under the `LocalSystem` account through the `PAExec` utility program
//! - "Payload" stage: Running under the `LocalSystem` account again in order to capture standard IO (see below)
//!
//! The information which stage the current process is in is passed via the first command line argument. Other command
//! line arguments are passed through verbatim. Each process waits for the next one to exit and passes on its exit code
//! to the previous one.
//!
//! Standard IO (stdin, stdout and stderr) cannot be captured at all when running a process as Administrator, neither
//! can they be captured in all cases by PAExec due to a bug. Therefore, we set up named pipes for the following
//! purposes:
//! - forwarding stdin of the Initial stage to stdin of the Payload stage
//! - forwarding stdout of the Payload stage to stdout of the Initial stage
//! - forwarding stderr of the Payload stage to stderr of the Initial stage
//! - forwarding stderr of the Admin stage to stderr of the Initial stage (in case we fail in the Admin stage)
//! - forwarding stderr of the System stage to stderr of the Initial stage (in case we fail in the System stage)

#![deny(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::io::{ErrorKind, Read, Write};
use std::process::{Child, Command, Stdio};
use std::{env, io, process};

use crate::admin_process;
use crate::system_runner::stdio::Pipe;

const INTERNAL_ERROR_CODE: i32 = i32::MIN;

pub fn ensure_running_as_system() -> io::Result<()> {
    SystemRunner::from_args()?.ensure_running_as_system()
}

#[derive(Clone, Copy, Debug)]
struct PipelineId(u32);

impl PipelineId {
    fn new() -> Self {
        Self(process::id())
    }

    fn from_str(s: &str) -> Option<Self> {
        s.parse().ok().map(Self)
    }

    fn to_pipe_name(self, key: &str) -> String {
        format!("__system_runner__.{}.{}", self.0, key)
    }
}

impl ToString for PipelineId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    Initial,
    Admin,
    System,
    Payload,
}

impl Stage {
    fn id(self) -> &'static str {
        match self {
            Stage::Initial => "0",
            Stage::Admin => "1",
            Stage::System => "2",
            Stage::Payload => "3",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "0" => Some(Self::Initial),
            "1" => Some(Self::Admin),
            "2" => Some(Self::System),
            "3" => Some(Self::Payload),
            _ => None,
        }
    }

    fn next(self) -> Option<Self> {
        match self {
            Stage::Initial => Some(Stage::Admin),
            Stage::Admin => Some(Stage::System),
            Stage::System => Some(Stage::Payload),
            Stage::Payload => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct State {
    pipeline_id: PipelineId,
    stage: Stage,
}

impl State {
    fn initial() -> Self {
        Self {
            pipeline_id: PipelineId::new(),
            stage: Stage::Initial,
        }
    }

    fn next(self) -> Option<Self> {
        Some(Self {
            stage: self.stage.next()?,
            ..self
        })
    }

    fn from_arg<S>(arg: S) -> Option<Self>
    where
        S: AsRef<OsStr>,
    {
        let mut parts = arg.as_ref().to_str()?.strip_prefix("__system_runner__.")?.split('.');

        match (parts.next(), parts.next()) {
            (Some(pipeline_id), Some(stage_id)) => Some(Self {
                pipeline_id: PipelineId::from_str(pipeline_id)?,
                stage: Stage::from_id(stage_id)?,
            }),
            _ => None,
        }
    }

    fn to_arg(self) -> String {
        format!("__system_runner__.{}.{}", self.pipeline_id.to_string(), self.stage.id())
    }
}

#[derive(Debug)]
pub(crate) struct SystemRunner {
    args: Vec<OsString>,
    state: State,
}

impl SystemRunner {
    pub(crate) fn from_args() -> io::Result<Self> {
        let mut args_os = env::args_os().skip(1).peekable();

        match args_os.peek().and_then(State::from_arg) {
            Some(state) => Ok(Self {
                args: args_os.skip(1).collect(),
                state,
            }),

            None => Ok(Self {
                args: args_os.collect(),
                state: State::initial(),
            }),
        }
    }

    pub(crate) fn into_args(self) -> Vec<OsString> {
        self.args
    }

    pub(crate) fn ensure_running_as_system(&self) -> io::Result<()> {
        let payload_stdout = Pipe::with_name(self.state.pipeline_id.to_pipe_name("payload_stdout"));
        let payload_stderr = Pipe::with_name(self.state.pipeline_id.to_pipe_name("payload_stderr"));
        let payload_stdin = Pipe::with_name(self.state.pipeline_id.to_pipe_name("payload_stdin"));
        let admin_stderr = Pipe::with_name(self.state.pipeline_id.to_pipe_name("admin_stderr"));
        let system_stderr = Pipe::with_name(self.state.pipeline_id.to_pipe_name("system_stderr"));

        match self.state.stage {
            Stage::Initial => {
                payload_stdout.create_reading_to(io::stdout())?;
                payload_stderr.create_reading_to(io::stderr())?;
                payload_stdin.create_writing_from(io::stdin())?;
                admin_stderr.create_reading_to(io::stderr())?;
                system_stderr.create_reading_to(io::stderr())?;

                match self.rerun_as_admin()?.wait()? {
                    INTERNAL_ERROR_CODE => Err(io::Error::new(ErrorKind::Other, "rerunning as system failed")),
                    exit_code => process::exit(exit_code),
                }
            }

            Stage::Admin => {
                let mut stderr = admin_stderr.connect_writing()?;

                match self.rerun_as_system().and_then(SystemProcess::wait) {
                    Ok(exit_code) => process::exit(exit_code),
                    Err(err) => {
                        writeln!(stderr, "{err}").unwrap();
                        process::exit(INTERNAL_ERROR_CODE);
                    }
                }
            }

            Stage::System => {
                let mut stderr = system_stderr.connect_writing()?;

                let rerun = || {
                    let stdout_client = payload_stdout.connect_writing()?;
                    let stderr_client = payload_stderr.connect_writing()?;
                    let stdin_client = payload_stdin.connect_reading()?;

                    let mut process = self.rerun()?;

                    stdout_client.redirect_from(process.stdout());
                    stderr_client.redirect_from(process.stderr());
                    stdin_client.redirect_to(process.stdin());

                    process.wait()
                };

                match rerun() {
                    Ok(exit_code) => process::exit(exit_code),
                    Err(err) => {
                        writeln!(stderr, "{err}").unwrap();
                        process::exit(INTERNAL_ERROR_CODE);
                    }
                }
            }

            Stage::Payload => Ok(()),
        }
    }

    fn rerun_as_admin(&self) -> io::Result<AdminProcess> {
        let child = admin_process::Command::new(env::current_exe()?)
            .arg(self.state.next().unwrap().to_arg())
            .args(&self.args)
            .spawn()?;

        Ok(AdminProcess(child))
    }

    fn rerun_as_system(&self) -> io::Result<SystemProcess> {
        let child = Command::new(concat!(env!("CARGO_MANIFEST_DIR"), "/thirdparty/paexec.exe"))
            .arg("-s")
            .arg(&env::current_exe()?)
            .arg(self.state.next().unwrap().to_arg())
            .args(&self.args)
            .spawn()?;

        Ok(SystemProcess(child))
    }

    fn rerun(&self) -> io::Result<PayloadProcess> {
        let child = Command::new(env::current_exe()?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg(self.state.next().unwrap().to_arg())
            .args(&self.args)
            .spawn()?;

        Ok(PayloadProcess(child))
    }
}

#[derive(Debug)]
struct AdminProcess(admin_process::Process);

impl AdminProcess {
    fn wait(self) -> io::Result<i32> {
        self.0.wait()
    }
}

#[derive(Debug)]
struct SystemProcess(Child);

impl SystemProcess {
    fn wait(mut self) -> io::Result<i32> {
        match self.0.wait()?.code().unwrap() {
            exit_code if exit_code == INTERNAL_ERROR_CODE || exit_code >= 0 => Ok(exit_code),
            exit_code => Err(io::Error::new(
                ErrorKind::Other,
                format!("PAExec failed with exit code {exit_code}"),
            )),
        }
    }
}

#[derive(Debug)]
struct PayloadProcess(Child);

impl PayloadProcess {
    fn wait(mut self) -> io::Result<i32> {
        self.0.wait().map(|status| status.code().unwrap())
    }

    fn stdin(&mut self) -> impl Write {
        self.0.stdin.take().unwrap()
    }

    fn stdout(&mut self) -> impl Read {
        self.0.stdout.take().unwrap()
    }

    fn stderr(&mut self) -> impl Read {
        self.0.stderr.take().unwrap()
    }
}

mod stdio {
    use std::ffi::OsStr;
    use std::io::{ErrorKind, Read, Write};
    use std::{io, thread};

    use interprocess::os::windows::named_pipe::{ByteReaderPipeStream, ByteWriterPipeStream, PipeListenerOptions};

    #[derive(Debug)]
    pub(super) struct Pipe {
        name: String,
    }

    impl Pipe {
        pub(super) fn with_name(name: String) -> Self {
            Self { name }
        }

        pub(super) fn create_reading_to<W>(&self, mut writer: W) -> io::Result<()>
        where
            W: Write + Send + 'static,
        {
            let listener = PipeListenerOptions::new()
                .name(OsStr::new(&self.name))
                .create::<ByteReaderPipeStream>()?;

            thread::spawn(move || copy(&mut listener.accept()?, &mut writer));
            Ok(())
        }

        pub(super) fn create_writing_from<R>(&self, mut reader: R) -> io::Result<()>
        where
            R: Read + Send + 'static,
        {
            let listener = PipeListenerOptions::new()
                .name(OsStr::new(&self.name))
                .create::<ByteWriterPipeStream>()?;

            thread::spawn(move || copy(&mut reader, &mut listener.accept()?));
            Ok(())
        }

        pub(super) fn connect_reading(&self) -> io::Result<ReadClient> {
            ByteReaderPipeStream::connect(&self.name).map(ReadClient)
        }

        pub(super) fn connect_writing(&self) -> io::Result<WriteClient> {
            ByteWriterPipeStream::connect(&self.name).map(WriteClient)
        }
    }

    #[derive(Debug)]
    pub(super) struct ReadClient(ByteReaderPipeStream);

    impl ReadClient {
        pub(super) fn redirect_to<W>(mut self, mut writer: W)
        where
            W: Write + Send + 'static,
        {
            thread::spawn(move || copy(&mut self.0, &mut writer));
        }
    }

    #[derive(Debug)]
    pub(super) struct WriteClient(ByteWriterPipeStream);

    impl WriteClient {
        pub(super) fn redirect_from<R>(mut self, mut reader: R)
        where
            R: Read + Send + 'static,
        {
            thread::spawn(move || copy(&mut reader, &mut self.0));
        }
    }

    impl Write for WriteClient {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }

    fn copy<R, W>(reader: &mut R, writer: &mut W) -> io::Result<()>
    where
        R: Read,
        W: Write,
    {
        match io::copy(reader, writer) {
            Ok(..) => Ok(()),
            Err(err) if err.kind() == ErrorKind::BrokenPipe => Ok(()),
            Err(err) => Err(err),
        }
    }
}
