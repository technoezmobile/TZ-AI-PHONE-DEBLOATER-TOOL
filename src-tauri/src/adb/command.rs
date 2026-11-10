use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Creation flag for Windows to suppress flashing cmd windows
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Output limit to prevent runaway memory usage (5 MB)
const MAX_OUTPUT_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub full_command: String,
}

#[derive(Debug, Clone)]
pub struct AdbExecutor {
    adb_path: PathBuf,
    default_timeout: Duration,
}

impl AdbExecutor {
    pub fn new(adb_path: impl AsRef<Path>) -> Self {
        Self {
            adb_path: adb_path.as_ref().to_path_buf(),
            default_timeout: Duration::from_secs(25),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    pub fn timeout(&self) -> Duration {
        self.default_timeout
    }

    pub fn adb_path(&self) -> &Path {
        &self.adb_path
    }

    /// Construct a base std::process::Command with no-window flag on Windows
    fn prepare_command(&self) -> Command {
        let mut cmd = Command::new(&self.adb_path);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }

    /// Execute an ADB command with arguments array, enforced timeout, and process reaping
    pub fn run_raw(&self, args: &[&str]) -> Result<CommandResult, String> {
        self.run_raw_with_timeout(args, self.default_timeout)
    }

    /// Execute command with explicit timeout duration
    pub fn run_raw_with_timeout(
        &self,
        args: &[&str],
        timeout: Duration,
    ) -> Result<CommandResult, String> {
        let mut cmd = self.prepare_command();
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let full_command = format!(
            "{} {}",
            self.adb_path.display(),
            if args.first() == Some(&"pair") {
                "pair [redacted]".to_string()
            } else {
                args.join(" ")
            }
        );

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn command '{full_command}': {e}"))?;

        // Drain both pipes concurrently so a full stderr pipe cannot deadlock stdout.
        fn drain<R: Read + Send + 'static>(mut pipe: R) -> thread::JoinHandle<(Vec<u8>, bool)> {
            thread::spawn(move || {
                let mut retained = Vec::new();
                let mut overflow = false;
                let mut chunk = [0u8; 8192];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let available = MAX_OUTPUT_BYTES.saturating_sub(retained.len());
                            retained.extend_from_slice(&chunk[..n.min(available)]);
                            overflow |= n > available;
                        }
                    }
                }
                (retained, overflow)
            })
        }
        let stdout = drain(child.stdout.take().ok_or("Missing stdout pipe")?);
        let stderr = drain(child.stderr.take().ok_or("Missing stderr pipe")?);
        let start = std::time::Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < timeout => thread::sleep(Duration::from_millis(10)),
                result => {
                    let _ = child.kill();
                    let _ = child.wait();
                    // Do not wait for pipes possibly inherited by a long-lived ADB server.
                    return Err(match result {
                        Err(e) => format!("Process wait failed: {e}"),
                        _ => format!(
                            "Command timed out after {}ms; child terminated and reaped",
                            timeout.as_millis()
                        ),
                    });
                }
            }
        };
        // A child may exit while its descendants retain the pipe handles. Bound this wait too.
        while !stdout.is_finished() || !stderr.is_finished() {
            if start.elapsed() >= timeout {
                return Err("Command timed out while draining output".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        let (out, out_overflow) = stdout.join().map_err(|_| "stdout reader failed")?;
        let (err, err_overflow) = stderr.join().map_err(|_| "stderr reader failed")?;
        if out_overflow || err_overflow {
            return Err("ADB output exceeded the safe limit".into());
        }
        Ok(CommandResult {
            success: status.success(),
            exit_code: status.code(),
            stdout: String::from_utf8_lossy(&out).trim().to_string(),
            stderr: String::from_utf8_lossy(&err).trim().to_string(),
            full_command,
        })
    }

    /// Execute an ADB command targeting a specific device serial explicitly.
    /// Returns error if serial is empty to prevent untargeted mutations.
    pub fn run_device(&self, serial: &str, args: &[&str]) -> Result<CommandResult, String> {
        if serial.trim().is_empty() {
            return Err("Device serial cannot be empty. Every device action must target an explicit device.".into());
        }

        let mut combined_args = Vec::with_capacity(args.len() + 2);
        combined_args.push("-s");
        combined_args.push(serial);
        combined_args.extend_from_slice(args);

        self.run_raw(&combined_args)
    }

    /// Execute `adb -s <serial> shell <args...>`
    pub fn run_shell(&self, serial: &str, shell_args: &[&str]) -> Result<CommandResult, String> {
        let mut args = vec!["shell"];
        args.extend_from_slice(shell_args);
        self.run_device(serial, &args)
    }
}

/// Package name validator preventing command injection
pub fn is_valid_package_name(pkg: &str) -> bool {
    if pkg.is_empty() || pkg.len() > 128 {
        return false;
    }
    // Android is a valid system package identifier
    if pkg == "android" {
        return true;
    }

    // Must contain at least one dot, no spaces, no control characters, no shell metacharacters
    let parts: Vec<&str> = pkg.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    for part in parts {
        if part.is_empty() {
            return false;
        }
        let first_char = part.chars().next().unwrap();
        // Parts must start with a letter
        if !first_char.is_ascii_alphabetic() {
            return false;
        }
        // Remaining characters in part can be alphanumeric or underscore
        for c in part.chars().skip(1) {
            if !c.is_ascii_alphanumeric() && c != '_' {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_package_names() {
        assert!(is_valid_package_name("android"));
        assert!(is_valid_package_name("com.android.settings"));
        assert!(is_valid_package_name("org.lineageos.jelly"));
        assert!(is_valid_package_name("com.google.android.apps.messaging"));
        assert!(is_valid_package_name("com.example.app_one"));
    }

    #[test]
    fn test_invalid_package_names_injection() {
        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("nodots"));
        assert!(!is_valid_package_name("com..example"));
        assert!(!is_valid_package_name("com.example; rm -rf /"));
        assert!(!is_valid_package_name("com.example | ls"));
        assert!(!is_valid_package_name("com.example && echo 1"));
        assert!(!is_valid_package_name("com.123.test"));
        assert!(!is_valid_package_name("com.example.app$value"));
        assert!(!is_valid_package_name("com.example.app`test`"));
        assert!(!is_valid_package_name("com.example. app"));
    }

    #[test]
    fn test_empty_serial_rejected() {
        let executor = AdbExecutor::new("adb");
        let res = executor.run_device("", &["shell", "ls"]);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("serial cannot be empty"));
    }

    #[test]
    fn test_executor_timeout_setting() {
        let executor = AdbExecutor::new("adb").with_timeout(Duration::from_secs(5));
        assert_eq!(executor.timeout(), Duration::from_secs(5));
    }
}
