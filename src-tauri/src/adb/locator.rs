use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdbStatus {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}

pub struct AdbLocator;

impl AdbLocator {
    /// Attempt to find adb executable from various locations
    pub fn locate(custom_path: Option<&str>) -> AdbStatus {
        // 1. Check custom path if provided
        if let Some(path_str) = custom_path {
            let path = PathBuf::from(path_str);
            if path.exists() {
                if let Ok(ver) = Self::verify_adb(&path) {
                    return AdbStatus {
                        found: true,
                        path: Some(path.to_string_lossy().to_string()),
                        version: Some(ver),
                        source: Some("Custom Configuration".into()),
                        error: None,
                    };
                }
            }
        }

        // 2. Check application's local bin folder in %LOCALAPPDATA%\TZ_AI_Debloater\platform-tools\adb.exe
        if let Some(local_app_data) = dirs::data_local_dir() {
            let local_bin = local_app_data
                .join("TZ_AI_Debloater")
                .join("platform-tools")
                .join(if cfg!(windows) { "adb.exe" } else { "adb" });

            if local_bin.exists() {
                if let Ok(ver) = Self::verify_adb(&local_bin) {
                    return AdbStatus {
                        found: true,
                        path: Some(local_bin.to_string_lossy().to_string()),
                        version: Some(ver),
                        source: Some("Application Managed Platform Tools".into()),
                        error: None,
                    };
                }
            }
        }

        // 3. Check system PATH
        if let Ok(path) = which::which("adb").or_else(|_| Self::check_where_adb()) {
            if let Ok(ver) = Self::verify_adb(&path) {
                return AdbStatus {
                    found: true,
                    path: Some(path.to_string_lossy().to_string()),
                    version: Some(ver),
                    source: Some("System PATH".into()),
                    error: None,
                };
            }
        }

        // 4. Check standard Android SDK locations on Windows
        let standard_paths = Self::get_standard_paths();
        for path in standard_paths {
            if path.exists() {
                if let Ok(ver) = Self::verify_adb(&path) {
                    return AdbStatus {
                        found: true,
                        path: Some(path.to_string_lossy().to_string()),
                        version: Some(ver),
                        source: Some("Android SDK".into()),
                        error: None,
                    };
                }
            }
        }

        AdbStatus {
            found: false,
            path: None,
            version: None,
            source: None,
            error: Some("ADB (Android Debug Bridge) executable not found. Please install Android Platform Tools, add adb to PATH, or use the setup tool.".into()),
        }
    }

    /// Verifies that the path points to a functioning ADB by invoking `version`
    pub fn verify_adb(path: &Path) -> Result<String, String> {
        let result = super::command::AdbExecutor::new(path)
            .with_timeout(std::time::Duration::from_secs(5))
            .run_raw(&["version"])?;
        if result.success && result.stdout.contains("Android Debug Bridge") {
            Ok(result.stdout.lines().next().unwrap_or("").to_string())
        } else {
            Err("The selected executable is not a functioning ADB binary".into())
        }
    }

    fn check_where_adb() -> Result<PathBuf, ()> {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("where");
            cmd.arg("adb");
            cmd.creation_flags(CREATE_NO_WINDOW);
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = stdout.lines().next() {
                        let p = PathBuf::from(first_line.trim());
                        if p.exists() {
                            return Ok(p);
                        }
                    }
                }
            }
        }
        Err(())
    }

    fn get_standard_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let bin_name = if cfg!(windows) { "adb.exe" } else { "adb" };

        #[cfg(target_os = "windows")]
        {
            if let Some(local) = dirs::data_local_dir() {
                paths.push(
                    local
                        .join("Android")
                        .join("Sdk")
                        .join("platform-tools")
                        .join(bin_name),
                );
                paths.push(
                    local
                        .join("Android")
                        .join("sdk")
                        .join("platform-tools")
                        .join(bin_name),
                );
            }

            if let Ok(prog_files) = std::env::var("ProgramFiles") {
                paths.push(
                    PathBuf::from(prog_files)
                        .join("Android")
                        .join("platform-tools")
                        .join(bin_name),
                );
            }
            if let Ok(prog_files_x86) = std::env::var("ProgramFiles(x86)") {
                paths.push(
                    PathBuf::from(prog_files_x86)
                        .join("Android")
                        .join("platform-tools")
                        .join(bin_name),
                );
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Library/Android/sdk/platform-tools").join(bin_name));
            }
            paths.push(PathBuf::from("/opt/homebrew/bin").join(bin_name));
            paths.push(PathBuf::from("/usr/local/bin").join(bin_name));
            paths.push(PathBuf::from("/opt/local/bin").join(bin_name));
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Android/Sdk/platform-tools").join(bin_name));
                paths.push(home.join(".local/share/platform-tools").join(bin_name));
            }
            paths.push(PathBuf::from("/usr/bin").join(bin_name));
            paths.push(PathBuf::from("/usr/local/bin").join(bin_name));
            paths.push(PathBuf::from("/snap/bin").join(bin_name));
        }

        paths
    }

    /// Download official Google Android Platform-Tools zip and unpack to user local app data
    pub async fn download_and_setup_platform_tools<F>(
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(f32, &str) + Send + 'static,
    {
        let local_app_data = dirs::data_local_dir()
            .ok_or_else(|| "Could not determine local app data directory".to_string())?;

        let target_dir = local_app_data.join("TZ_AI_Debloater");
        tokio::fs::create_dir_all(&target_dir)
            .await
            .map_err(|e| format!("Failed to create destination folder: {e}"))?;

        let url = if cfg!(target_os = "windows") {
            "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
        } else if cfg!(target_os = "macos") {
            "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
        } else {
            "https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
        };
        progress_callback(0.1, "Connecting to official Google Android repository...");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to download platform-tools: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Google repository returned HTTP status {}",
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(12 * 1024 * 1024);
        let mut downloaded: u64 = 0;
        let mut bytes_stream = response.bytes_stream();

        let zip_path = target_dir.join("platform-tools-temp.zip");
        let mut file = tokio::fs::File::create(&zip_path)
            .await
            .map_err(|e| format!("Failed to create temp zip file: {e}"))?;

        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        while let Some(chunk_res) = bytes_stream.next().await {
            let chunk = chunk_res.map_err(|e| format!("Download error: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {e}"))?;

            downloaded += chunk.len() as u64;
            let ratio = (downloaded as f32 / total_size as f32).min(0.95);
            progress_callback(
                0.1 + (ratio * 0.7),
                &format!(
                    "Downloading: {:.1} MB / {:.1} MB",
                    downloaded as f32 / 1_048_576.0,
                    total_size as f32 / 1_048_576.0
                ),
            );
        }

        file.flush()
            .await
            .map_err(|e| format!("Flush error: {e}"))?;
        drop(file);

        progress_callback(0.85, "Extracting platform tools...");

        // Extract zip file synchronously in a blocking thread
        let zip_path_clone = zip_path.clone();
        let target_dir_clone = target_dir.clone();

        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&zip_path_clone)
                .map_err(|e| format!("Failed to open downloaded zip: {e}"))?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {e}"))?;

            archive
                .extract(&target_dir_clone)
                .map_err(|e| format!("Failed to extract zip archive: {e}"))?;

            let _ = std::fs::remove_file(&zip_path_clone);
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Extraction task panicked: {e}"))??;

        let adb_binary =
            target_dir
                .join("platform-tools")
                .join(if cfg!(windows) { "adb.exe" } else { "adb" });
        if !adb_binary.exists() {
            return Err("Extracted archive did not contain expected adb executable".into());
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&adb_binary) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&adb_binary, perms);
            }
        }

        progress_callback(1.0, "Platform tools setup complete!");
        Ok(adb_binary)
    }
}
