//! Cross-process advisory locking to synchronize state when GUI and CLI run concurrently.
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct CrossProcessLock {
    path: PathBuf,
    name: String,
}

fn get_lock_dir() -> PathBuf {
    let base = if cfg!(debug_assertions) {
        std::env::var_os("TZ_DEBLOATER_TEST_DATA")
            .map(PathBuf::from)
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."))
    };
    let dir = base.join("TZ_AI_Debloater").join("locks");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn is_process_running(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new("tasklist");
        cmd.args(&["/FI", &format!("PID eq {pid}"), "/NH"]);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        if let Ok(output) = cmd.output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains(&pid.to_string());
        }
        true
    }

    #[cfg(unix)]
    {
        // kill -0 <pid> tests whether a process exists
        if let Ok(status) = std::process::Command::new("kill")
            .args(&["-0", &pid.to_string()])
            .status()
        {
            status.success()
        } else {
            true
        }
    }

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        true
    }
}

impl CrossProcessLock {
    /// Acquire an exclusive cross-process lock by name, polling until timeout_ms
    pub fn acquire(name: &str, timeout_ms: u64) -> Result<Self, String> {
        let lock_dir = get_lock_dir();
        let clean_name = name.replace([':', '/', '\\', ' '], "_");
        let path = lock_dir.join(format!("{clean_name}.lock"));
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    use std::io::Write;
                    let pid = std::process::id();
                    let now = chrono::Utc::now().timestamp();
                    let _ = writeln!(file, "{pid}:{now}");
                    return Ok(Self {
                        path,
                        name: name.to_string(),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Check for stale lock
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some((pid_str, ts_str)) = content.trim().split_once(':') {
                            if let (Ok(pid), Ok(ts)) = (pid_str.parse::<u32>(), ts_str.parse::<i64>()) {
                                let now = chrono::Utc::now().timestamp();
                                // Stale if older than 5 minutes or if the locking process is dead
                                if now - ts > 300 || !is_process_running(pid) {
                                    let _ = fs::remove_file(&path);
                                    continue;
                                }
                            }
                        }
                    }

                    if start.elapsed() >= timeout {
                        return Err(format!(
                            "Target resource '{name}' is currently locked by another active TZ Debloater instance. Please wait for the current operation to finish."
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(75));
                }
                Err(e) => {
                    return Err(format!("Failed to create lock file for '{name}': {e}"));
                }
            }
        }
    }

    /// Convenience lock for target Android device serial
    pub fn lock_device(serial: &str) -> Result<Self, String> {
        Self::acquire(&format!("target_{serial}"), 4000)
    }

    /// Convenience lock for journal mutations
    pub fn lock_journal() -> Result<Self, String> {
        Self::acquire("journal", 3000)
    }

    /// Convenience lock for configuration updates
    pub fn lock_config() -> Result<Self, String> {
        Self::acquire("config", 3000)
    }

    /// Resource name being locked
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for CrossProcessLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
