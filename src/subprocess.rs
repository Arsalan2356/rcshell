//! Non-blocking subprocesses for the main-loop futures.
//!
//! `std::process::Command::output()` blocks the GTK main thread until the child
//! exits. These run the child through `gio::Subprocess` and await its output,
//! so the main loop keeps handling input and other futures in the meantime.
//! GSubprocess also reaps the child, so nothing is left as a zombie.

use std::ffi::OsStr;

/// Run `argv` and return its stdout. stderr is discarded.
///
/// Returns an empty string if the command can't be started or its output
/// isn't valid UTF-8.
pub async fn output(argv: &[&str]) -> String {
    let argv: Vec<&OsStr> = argv.iter().map(OsStr::new).collect();

    let Ok(proc) = gio::Subprocess::newv(
        &argv,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE,
    ) else {
        return String::new();
    };

    match proc.communicate_utf8_future(None).await {
        Ok((Some(stdout), _)) => stdout.to_string(),
        _ => String::new(),
    }
}

/// Run `script` with `sh -c` and return its stdout.
pub async fn sh(script: &str) -> String {
    output(&["sh", "-c", script]).await
}
