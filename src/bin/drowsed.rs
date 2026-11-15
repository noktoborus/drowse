//! drowsed: the system service that does the actions of its stages while
//! every session is idle.

use std::process::ExitCode;

fn main() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            drowse::journal::error(&error);
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(drowse::service::serve()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            drowse::journal::error(&error);
            ExitCode::FAILURE
        }
    }
}
