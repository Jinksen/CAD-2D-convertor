//! Starts the bundled geometry service in installed and portable builds.
//!
//! Development (`scripts/dev.ps1`) and the checkout launcher (`scripts/run.ps1`) start the
//! service themselves and pass its token in `CAD2MAXWELL_SESSION_TOKEN`; then nothing is spawned.

use std::fs::{self, File};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const BACKEND_EXE: &str = "backend/cad2maxwell-backend.exe";
const PORT: u16 = 8000;

/// Token of the service this window talks to, and the child process when we started it.
#[derive(Default)]
pub struct Backend {
    pub token: Option<String>,
    child: Arc<Mutex<Option<Child>>>,
}

fn new_token() -> Result<String, String> {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).map_err(|error| format!("No secure random source: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn port_in_use() -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], PORT)),
        Duration::from_millis(300),
    )
    .is_ok()
}

fn spawn(executable: &Path, token: &str, log_dir: &Path) -> Result<Child, String> {
    fs::create_dir_all(log_dir)
        .map_err(|error| format!("Cannot create the log folder: {error}"))?;
    let log = File::create(log_dir.join("backend.log"))
        .map_err(|error| format!("Cannot create the backend log: {error}"))?;
    let log_err = log.try_clone().map_err(|error| error.to_string())?;
    let mut command = Command::new(executable);
    command
        .env("CAD2MAXWELL_SESSION_TOKEN", token)
        .env("CAD2MAXWELL_PARENT_PID", std::process::id().to_string())
        .current_dir(executable.parent().unwrap_or(Path::new(".")))
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(log_err);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .map_err(|error| format!("Cannot start the geometry service: {error}"))
}

fn report(message: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("CAD2Maxwell")
        .set_description(message)
        .show();
}

/// The installer places the service next to the executable.
fn bundled_executable() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?.parent()?.join(BACKEND_EXE);
    path.is_file().then_some(path)
}

fn log_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(|dir| {
            PathBuf::from(dir)
                .join("com.cad2maxwell.desktop")
                .join("logs")
        })
        .unwrap_or_else(|| std::env::temp_dir().join("CAD2Maxwell"))
}

/// Decides which service to use. Errors are shown to the user; the window still opens
/// and its status bar reports the service as offline.
pub fn start() -> Backend {
    if let Ok(token) = std::env::var("CAD2MAXWELL_SESSION_TOKEN") {
        return Backend {
            token: Some(token),
            ..Backend::default()
        };
    }
    let Some(executable) = bundled_executable() else {
        return Backend::default();
    };
    if port_in_use() {
        report(
            "Port 8000 is already in use, so the geometry service cannot start.\n\nClose the other CAD2Maxwell window (or the program using port 8000) and start CAD2Maxwell again.",
        );
        return Backend::default();
    }
    let log_dir = log_dir();
    match new_token()
        .and_then(|token| spawn(&executable, &token, &log_dir).map(|child| (token, child)))
    {
        Ok((token, child)) => {
            let child = Arc::new(Mutex::new(Some(child)));
            watch(Arc::clone(&child), log_dir);
            Backend {
                token: Some(token),
                child,
            }
        }
        Err(error) => {
            report(&format!("{error}\n\nLog folder: {}", log_dir.display()));
            Backend::default()
        }
    }
}

/// Tells the user, with the log folder, when the service dies while the window is open.
fn watch(child: Arc<Mutex<Option<Child>>>, log_dir: PathBuf) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let exited = match child.lock() {
                // None: stop() took it on exit, so the end is expected.
                Ok(mut guard) => match guard.as_mut() {
                    Some(process) => process.try_wait().ok().flatten(),
                    None => return,
                },
                Err(_) => return,
            };
            if let Some(status) = exited {
                report(&format!(
                    "The geometry service stopped unexpectedly ({status}). Import and sectioning are unavailable until CAD2Maxwell is restarted.

Details are in backend.log in:
{}",
                    log_dir.display()
                ));
                return;
            }
        }
    });
}

/// Stops the service we started. It also exits by itself when this process dies.
pub fn stop(backend: &Backend) {
    if let Ok(mut child) = backend.child.lock()
        && let Some(mut child) = child.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::new_token;

    #[test]
    fn tokens_are_long_unique_hex() {
        let (a, b) = (new_token().unwrap(), new_token().unwrap());
        assert_eq!(a.len(), 48);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
