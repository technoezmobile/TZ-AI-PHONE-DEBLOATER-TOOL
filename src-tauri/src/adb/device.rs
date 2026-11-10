use super::command::AdbExecutor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceConnectionState {
    Ready,
    Unauthorized,
    Offline,
    NoPermissions,
    Recovery,
    Sideload,
    Unknown(String),
}

impl DeviceConnectionState {
    pub fn from_adb_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "device" => Self::Ready,
            "unauthorized" => Self::Unauthorized,
            "offline" => Self::Offline,
            "no permissions" => Self::NoPermissions,
            "recovery" => Self::Recovery,
            "sideload" => Self::Sideload,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "device",
            Self::Unauthorized => "unauthorized",
            Self::Offline => "offline",
            Self::NoPermissions => "no permissions",
            Self::Recovery => "recovery",
            Self::Sideload => "sideload",
            Self::Unknown(_) => "unknown",
        }
    }

    pub fn display_badge(&self) -> (&'static str, &'static str) {
        match self {
            Self::Ready => (
                "Ready",
                "bg-emerald-500/10 text-emerald-400 border-emerald-500/30",
            ),
            Self::Unauthorized => (
                "Unauthorized",
                "bg-amber-500/10 text-amber-400 border-amber-500/30",
            ),
            Self::Offline => ("Offline", "bg-red-500/10 text-red-400 border-red-500/30"),
            Self::NoPermissions => (
                "No Permissions",
                "bg-amber-500/10 text-amber-400 border-amber-500/30",
            ),
            Self::Recovery => (
                "Recovery",
                "bg-purple-500/10 text-purple-400 border-purple-500/30",
            ),
            Self::Sideload => (
                "Sideload",
                "bg-blue-500/10 text-blue-400 border-blue-500/30",
            ),
            Self::Unknown(_) => ("Unknown", "bg-zinc-500/10 text-zinc-400 border-zinc-500/30"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub serial: String,
    pub state: DeviceConnectionState,
    pub connection_type: String, // "USB" or "Wireless"
    pub model: String,
    pub brand: String,
    pub manufacturer: String,
    pub android_version: String,
    pub android_sdk: u32,
    pub build_id: String,
    pub security_patch: String,
}

pub struct DeviceManager;

impl DeviceManager {
    /// Query list of attached devices and their states from `adb devices -l`
    pub fn list_devices(executor: &AdbExecutor) -> Result<Vec<DeviceInfo>, String> {
        let res = executor.run_raw(&["devices", "-l"])?;
        if !res.success {
            return Err(format!("Failed to query adb devices: {}", res.stderr));
        }

        let mut devices = Vec::new();
        let lines: Vec<&str> = res.stdout.lines().collect();

        // First line is "List of devices attached"
        for line in lines.iter().skip(1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // format is: <serial> <state> [product:...] [model:...] [device:...] [transport_id:...]
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.len() < 2 {
                continue;
            }

            let serial = tokens[0].to_string();
            let state = DeviceConnectionState::from_adb_str(tokens[1]);
            let is_wireless = serial.contains(':');
            let connection_type = if is_wireless {
                "Wireless".to_string()
            } else {
                "USB".to_string()
            };

            if state.is_ready() {
                // Populate detailed device properties
                let model = Self::get_prop(executor, &serial, "ro.product.model")
                    .unwrap_or_else(|_| "Unknown Model".into());
                let brand = Self::get_prop(executor, &serial, "ro.product.brand")
                    .unwrap_or_else(|_| "".into());
                let manufacturer = Self::get_prop(executor, &serial, "ro.product.manufacturer")
                    .unwrap_or_else(|_| "".into());
                let android_version = Self::get_prop(executor, &serial, "ro.build.version.release")
                    .unwrap_or_else(|_| "Unknown".into());
                let android_sdk = Self::get_prop(executor, &serial, "ro.build.version.sdk")
                    .ok()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                let build_id = Self::get_prop(executor, &serial, "ro.build.id").unwrap_or_default();
                let security_patch =
                    Self::get_prop(executor, &serial, "ro.build.version.security_patch")
                        .unwrap_or_default();

                devices.push(DeviceInfo {
                    serial,
                    state,
                    connection_type,
                    model,
                    brand,
                    manufacturer,
                    android_version,
                    android_sdk,
                    build_id,
                    security_patch,
                });
            } else {
                // Device not ready (e.g. unauthorized or offline)
                devices.push(DeviceInfo {
                    serial,
                    state,
                    connection_type,
                    model: "Device locked/unauthorized".into(),
                    brand: "".into(),
                    manufacturer: "".into(),
                    android_version: "".into(),
                    android_sdk: 0,
                    build_id: "".into(),
                    security_patch: "".into(),
                });
            }
        }

        Ok(devices)
    }

    /// Read an Android system property from device
    pub fn get_prop(executor: &AdbExecutor, serial: &str, key: &str) -> Result<String, String> {
        let res = executor.run_shell(serial, &["getprop", key])?;
        if res.success {
            Ok(res.stdout.trim().to_string())
        } else {
            Err(res.stderr)
        }
    }

    /// Discover currently active default launcher package
    pub fn get_active_launcher(executor: &AdbExecutor, serial: &str) -> Option<String> {
        Self::get_active_launcher_for_user(executor, serial, 0)
    }
    pub fn get_active_launcher_for_user(
        executor: &AdbExecutor,
        serial: &str,
        user: u32,
    ) -> Option<String> {
        if let Ok(res) = executor.run_shell(
            serial,
            &[
                "cmd",
                "package",
                "resolve-activity",
                "--brief",
                "--user",
                &user.to_string(),
                "-a",
                "android.intent.action.MAIN",
                "-c",
                "android.intent.category.HOME",
            ],
        ) {
            if res.success {
                for line in res.stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains('/')
                        && !trimmed.starts_with("priority=")
                        && !trimmed.starts_with("match=")
                    {
                        if let Some(pkg) = trimmed.split('/').next() {
                            let clean_pkg = pkg.trim();
                            if super::command::is_valid_package_name(clean_pkg) {
                                return Some(clean_pkg.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Discover currently active default keyboard / IME package
    pub fn get_active_ime(executor: &AdbExecutor, serial: &str) -> Option<String> {
        Self::get_active_ime_for_user(executor, serial, 0)
    }
    pub fn get_active_ime_for_user(
        executor: &AdbExecutor,
        serial: &str,
        user: u32,
    ) -> Option<String> {
        if let Ok(res) = executor.run_shell(
            serial,
            &[
                "settings",
                "--user",
                &user.to_string(),
                "get",
                "secure",
                "default_input_method",
            ],
        ) {
            if res.success {
                let trimmed = res.stdout.trim();
                if let Some(pkg) = trimmed.split('/').next() {
                    let clean_pkg = pkg.trim();
                    if super::command::is_valid_package_name(clean_pkg) {
                        return Some(clean_pkg.to_string());
                    }
                }
            }
        }
        None
    }

    /// Pair with a wireless debugging endpoint (`adb pair <host>:<port> <code>`)
    pub fn pair_wireless(
        executor: &AdbExecutor,
        host: &str,
        pairing_port: u16,
        pairing_code: &str,
    ) -> Result<String, String> {
        let host = host.trim();
        let pairing_code = pairing_code.trim();

        if host.is_empty() {
            return Err("Wireless debugging host/IP address cannot be empty.".into());
        }
        if pairing_port == 0 {
            return Err("Pairing port must be greater than 0.".into());
        }
        if pairing_code.len() != 6 || !pairing_code.chars().all(|c| c.is_ascii_digit()) {
            return Err(
                "Pairing code must be a 6-digit number as shown on your phone's screen.".into(),
            );
        }

        let address = format!("{host}:{pairing_port}");
        let res = executor.run_raw(&["pair", &address, pairing_code])?;

        if res.success && res.stdout.to_lowercase().contains("successfully paired") {
            Ok(format!("Successfully paired with {address}! Now connect using your device's connection port."))
        } else {
            let msg = if !res.stdout.is_empty() {
                res.stdout
            } else {
                res.stderr
            };
            Err(format!("Pairing failed: {msg}"))
        }
    }

    /// Connect to a wireless debugging endpoint (`adb connect <host>:<port>`)
    pub fn connect_wireless(
        executor: &AdbExecutor,
        host: &str,
        connect_port: u16,
    ) -> Result<String, String> {
        let host = host.trim();

        if host.is_empty() {
            return Err("Wireless debugging host/IP address cannot be empty.".into());
        }
        if connect_port == 0 {
            return Err("Connection port must be greater than 0.".into());
        }

        let address = format!("{host}:{connect_port}");
        let res = executor.run_raw(&["connect", &address])?;

        if res.success && res.stdout.to_lowercase().contains("connected to") {
            Ok(format!("Successfully connected to {address}"))
        } else {
            let msg = if !res.stdout.is_empty() {
                res.stdout
            } else {
                res.stderr
            };
            Err(format!("Connection failed: {msg}"))
        }
    }

    /// Disconnect wireless debugging endpoint (`adb disconnect <address>`)
    pub fn disconnect_wireless(executor: &AdbExecutor, address: &str) -> Result<String, String> {
        let res = executor.run_raw(&["disconnect", address])?;
        Ok(res.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_states_parsing() {
        assert_eq!(
            DeviceConnectionState::from_adb_str("device"),
            DeviceConnectionState::Ready
        );
        assert_eq!(
            DeviceConnectionState::from_adb_str("unauthorized"),
            DeviceConnectionState::Unauthorized
        );
        assert_eq!(
            DeviceConnectionState::from_adb_str("offline"),
            DeviceConnectionState::Offline
        );
        assert_eq!(
            DeviceConnectionState::from_adb_str("no permissions"),
            DeviceConnectionState::NoPermissions
        );
        assert_eq!(
            DeviceConnectionState::from_adb_str("recovery"),
            DeviceConnectionState::Recovery
        );
    }
}
