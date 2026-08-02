//! Crash log: a panic hook that leaves a file where the saves live.
//!
//! A native player whose game dies gets a closed window and nothing else —
//! the panic message goes to a console nobody has open. Installing this
//! hook first thing in `main` appends every panic (message, location,
//! timestamp) to `crash_log.txt` in the same per-game app-data directory
//! the save file uses, so "it crashed" can become a file the player can
//! find and share. The previous hook still runs, so console output and
//! test harness behavior are unchanged.
//!
//! On wasm this is a no-op: browser panics already land in the developer
//! console, and there is no filesystem to leave a file on.
//!
//! ```no_run
//! macroquad_toolkit::crash::install_crash_log("my_game");
//! ```

/// Install a panic hook that appends panics to `crash_log.txt` in the
/// game's app-data directory (falling back to the working directory when
/// no app-data path exists). Chains to the previously installed hook.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_crash_log(game_name: &'static str) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let path = crate::persistence::get_app_data_path(game_name, "crash_log.txt")
            .unwrap_or_else(|| std::path::PathBuf::from("crash_log.txt"));
        let unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0);
        let entry = format!("--- panic at unix time {unix_seconds} ---\n{info}\n");

        // A game that has never saved has no app-data directory yet, and a
        // crash log that only works after the first save would miss the
        // crashes most worth catching.
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        use std::io::Write;
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(mut file) => {
                let _ = file.write_all(entry.as_bytes());
                eprintln!("Crash log written to {}", path.display());
            }
            Err(error) => {
                eprintln!("Could not write crash log to {}: {error}", path.display());
            }
        }

        previous(info);
    }));
}

/// No-op on wasm: browser panics reach the developer console already, and
/// there is no filesystem to leave a log on.
#[cfg(target_arch = "wasm32")]
pub fn install_crash_log(_game_name: &'static str) {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    /// A panic inside a catch_unwind must leave a crash log behind, and the
    /// entry must carry the panic message. Run in one test rather than two,
    /// because panic hooks are process-global.
    #[test]
    fn a_panic_leaves_a_log_where_the_saves_live() {
        super::install_crash_log("toolkit_crash_test");

        let result = std::panic::catch_unwind(|| {
            panic!("deliberate test panic: the meter is still running");
        });
        assert!(result.is_err(), "the panic must actually happen");

        let path = crate::persistence::get_app_data_path("toolkit_crash_test", "crash_log.txt")
            .expect("an app-data path exists on native");
        let log = std::fs::read_to_string(&path).expect("the crash log was written");
        assert!(
            log.contains("deliberate test panic: the meter is still running"),
            "the log does not carry the panic message: {log:?}"
        );

        // Leave no artifact for the next run to misread.
        let _ = std::fs::remove_file(&path);
    }
}
