use super::command::AdbExecutor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidUser {
    pub id: u32,
    pub name: String,
    pub flags: u32,
    pub is_running: bool,
    pub is_primary: bool,
    pub profile_type: String, // "Owner", "Work Profile", "Cloned App Profile", "Secondary User", "Guest"
    pub display_label: String,
}

pub struct UserManager;

impl UserManager {
    /// List all Android users/profiles on the connected device
    pub fn list_users(executor: &AdbExecutor, serial: &str) -> Result<Vec<AndroidUser>, String> {
        let res = executor.run_shell(serial, &["pm", "list", "users"])?;
        if !res.success {
            return Err(format!("Failed to list users on device: {}", res.stderr));
        }

        Self::parse_users_output(&res.stdout)
    }

    pub fn parse_users_output(output: &str) -> Result<Vec<AndroidUser>, String> {
        let mut users = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim();
            // Typical line format: "UserInfo{0:Owner:13} running" or "UserInfo{10:Work profile:30}"
            if !trimmed.starts_with("UserInfo{") {
                continue;
            }

            let is_running = trimmed.ends_with("running");
            let inner_start = trimmed.find('{').unwrap_or(0) + 1;
            let inner_end = trimmed.find('}').unwrap_or(trimmed.len());
            if inner_start >= inner_end {
                continue;
            }

            let inner = &trimmed[inner_start..inner_end];
            let parts: Vec<&str> = inner.split(':').collect();
            if parts.len() < 3 {
                continue;
            }

            let id = match parts[0].parse::<u32>() {
                Ok(parsed_id) => parsed_id,
                Err(_) => continue,
            };
            let name = parts[1].to_string();
            let flags = u32::from_str_radix(parts[2], 16).unwrap_or(0);

            let is_primary = id == 0;
            let profile_type = if is_primary {
                "Owner / Personal".to_string()
            } else if name.to_lowercase().contains("work") || (flags & 0x00000020 != 0) {
                "Work Profile".to_string()
            } else if name.to_lowercase().contains("clone") || name.to_lowercase().contains("dual")
            {
                "Dual / Cloned App Profile".to_string()
            } else if name.to_lowercase().contains("guest") || (flags & 0x00000004 != 0) {
                "Guest Profile".to_string()
            } else {
                "Secondary User".to_string()
            };

            let display_label = format!("User {} — {} ({})", id, name, profile_type);

            users.push(AndroidUser {
                id,
                name,
                flags,
                is_running,
                is_primary,
                profile_type,
                display_label,
            });
        }

        if users.is_empty() {
            return Err("No Android user profiles could be enumerated from device output. Check device permissions.".into());
        }

        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multi_user_output() {
        let sample = "\
Users:
\tUserInfo{0:Owner:13} running
\tUserInfo{10:Work profile:30} running
\tUserInfo{999:Dual App:0}
";
        let users = UserManager::parse_users_output(sample).unwrap();
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].id, 0);
        assert!(users[0].is_primary);
        assert!(users[0].is_running);
        assert_eq!(users[1].id, 10);
        assert_eq!(users[1].profile_type, "Work Profile");
        assert_eq!(users[2].id, 999);
        assert_eq!(users[2].profile_type, "Dual / Cloned App Profile");
    }

    #[test]
    fn test_empty_users_output_fails_honestly() {
        let sample = "Error: command failed or empty output";
        assert!(UserManager::parse_users_output(sample).is_err());
    }
}
