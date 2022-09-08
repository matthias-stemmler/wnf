pub extern crate libtest_mimic;

use std::io;
use std::process::ExitCode;

use libtest_mimic::{Arguments, Trial};

use crate::system_runner::SystemRunner;

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
    match option_env!("WNF_SYSTEM_TESTS_ENABLED")
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("true") => match run_tests_as_system(tests) {
            Ok(..) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Failed to run tests as system: {err}");
                ExitCode::FAILURE
            }
        },

        _ => {
            println!("System tests are disabled, set WNF_SYSTEM_TESTS_ENABLED=true to enable");

            libtest_mimic::run(
                &Arguments::from_args(),
                tests.into_iter().map(|test| test.with_ignored_flag(true)).collect(),
            )
        }
        .exit(),
    }
}

fn run_tests_as_system(tests: Vec<Trial>) -> io::Result<()> {
    let system_runner = SystemRunner::from_args()?;
    system_runner.ensure_running_as_system()?;
    libtest_mimic::run(&Arguments::from_iter(system_runner.args()), tests).exit()
}
