use super::command::{is_valid_package_name, AdbExecutor};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageState {
    Enabled,
    Disabled,
    Uninstalled,
}

impl PackageState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Uninstalled => "uninstalled",
        }
    }
}

impl std::fmt::Display for PackageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidPackageType {
    System,
    UpdatedSystem,
    User,
}

impl AndroidPackageType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::UpdatedSystem => "UpdatedSystem",
            Self::User => "User",
        }
    }
}

impl std::fmt::Display for AndroidPackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemovalEligibility {
    Eligible,
    ReviewOnly,
    Protected,
    AlreadyRemoved,
}

impl RemovalEligibility {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Eligible => "Eligible",
            Self::ReviewOnly => "ReviewOnly",
            Self::Protected => "Protected",
            Self::AlreadyRemoved => "AlreadyRemoved",
        }
    }
}

impl std::fmt::Display for RemovalEligibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePackage {
    pub package_id: String,
    pub state: PackageState,
    pub is_system: bool,
    pub package_type: AndroidPackageType,
    pub user_id: u32,
    pub list_category: String, // "Oem", "Aosp", "Google", "Carrier", "Misc", "Unlisted"
    pub removal_recommendation: String, // "Recommended", "Advanced", "Expert", "Unsafe", "Unlisted"
    pub removal_eligibility: RemovalEligibility,
    pub description: String,
    pub dependencies: Vec<String>,
    pub needed_by: Vec<String>,
    pub is_protected: bool,
    pub protection_reason: Option<String>,
}

pub struct PackageManager;

impl PackageManager {
    /// Parse `package:<id>` lines from `pm list packages` output
    pub fn parse_package_list(output: &str) -> HashSet<String> {
        let mut packages = HashSet::new();
        const PREFIX: &str = "package:";

        for line in output.lines() {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix(PREFIX) {
                let pkg_name = stripped.trim_end_matches('\r').trim();
                if is_valid_package_name(pkg_name) {
                    packages.insert(pkg_name.to_string());
                }
            }
        }

        packages
    }

    /// Parse `package:<path>=<id>` lines from `pm list packages -f` output
    pub fn parse_package_file_paths(output: &str) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        const PREFIX: &str = "package:";

        for line in output.lines() {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix(PREFIX) {
                if let Some((path_str, pkg_id)) = stripped.rsplit_once('=') {
                    let pkg_clean = pkg_id.trim_end_matches('\r').trim();
                    if is_valid_package_name(pkg_clean) {
                        map.insert(pkg_clean.to_string(), path_str.trim().to_string());
                    }
                }
            }
        }

        map
    }

    /// Fetch raw list of packages with specific pm flags strictly bound to user profile
    pub fn list_packages_with_flags(
        executor: &AdbExecutor,
        serial: &str,
        user_id: u32,
        extra_flag: Option<&str>,
    ) -> Result<HashSet<String>, String> {
        let user_str = user_id.to_string();
        let mut args = vec!["pm", "list", "packages"];

        if let Some(flag) = extra_flag {
            args.push(flag);
        }

        // Target explicit profile strictly; do NOT silently drop --user
        args.push("--user");
        args.push(&user_str);

        let res = executor.run_shell(serial, &args)?;
        if !res.success {
            return Err(format!(
                "Failed to list packages for user {user_id}: {}",
                res.stderr
            ));
        }

        // Even with exit code 0, Android pm can output error messages to stdout/stderr:
        let combined = format!("{}\n{}", res.stdout, res.stderr);
        let lower = combined.to_lowercase();
        if lower.contains("securityexception")
            || lower.contains("permission denial")
            || lower.contains("inaccessible profile")
            || lower.contains("inaccessible user")
            || lower.contains("user not found")
            || lower.contains("could not access package manager")
            || lower.contains("nullpointerexception")
            || lower.contains("parcelableexception")
            || lower.contains("androidruntime:")
            || lower.contains("exception occurred while executing")
            || lower.contains("error:")
            || lower.contains("failure [")
        {
            let first_err = combined
                .lines()
                .map(|l| l.trim())
                .find(|l| {
                    let ll = l.to_lowercase();
                    ll.contains("exception")
                        || ll.contains("error")
                        || ll.contains("denial")
                        || ll.contains("failure")
                        || ll.contains("inaccessible")
                })
                .unwrap_or("Android package manager returned an error");
            return Err(format!(
                "Android package discovery rejected error output for user {user_id}: {first_err}"
            ));
        }

        let non_empty_lines: Vec<&str> = res
            .stdout
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        let has_package_line = non_empty_lines.iter().any(|l| l.starts_with("package:"));

        // If stdout contains non-empty lines but not a single valid package: line, it is malformed output
        if !non_empty_lines.is_empty() && !has_package_line {
            return Err(format!(
                "Malformed package list output for user {user_id}: {}",
                non_empty_lines[0]
            ));
        }

        Ok(Self::parse_package_list(&res.stdout))
    }

    /// Scan all packages for a specific user and categorize them
    pub fn scan_device_packages(
        executor: &AdbExecutor,
        serial: &str,
        user_id: u32,
    ) -> Result<Vec<DevicePackage>, String> {
        // 1. All packages known to device for this user (including uninstalled system packages)
        let all_user_packages =
            Self::list_packages_with_flags(executor, serial, user_id, Some("-u"))?;

        if all_user_packages.is_empty() {
            return Err(format!(
                "Device returned empty package inventory for user {user_id}. Package discovery rejected untrusted empty inventory."
            ));
        }

        // 2. Currently enabled packages for this user
        let enabled_packages =
            Self::list_packages_with_flags(executor, serial, user_id, Some("-e"))?;

        // 3. Currently disabled packages for this user
        let disabled_packages =
            Self::list_packages_with_flags(executor, serial, user_id, Some("-d"))?;

        // 4. System packages (for system vs user-app distinction)
        let system_packages =
            Self::list_packages_with_flags(executor, serial, user_id, Some("-s"))?;

        // 4b. Query system package file paths to detect UpdatedSystem packages from OS metadata
        let user_str = user_id.to_string();
        let system_file_map = match executor.run_shell(
            serial,
            &["pm", "list", "packages", "-s", "-f", "--user", &user_str],
        ) {
            Ok(res) if res.success => Self::parse_package_file_paths(&res.stdout),
            _ => std::collections::HashMap::new(),
        };

        // 5. Query active launcher and input method for dynamic protection
        let active_launcher =
            super::device::DeviceManager::get_active_launcher_for_user(executor, serial, user_id);
        let active_ime =
            super::device::DeviceManager::get_active_ime_for_user(executor, serial, user_id);

        let catalog = crate::dataset::get_catalog();
        let mut policy = crate::policy::SafetyPolicy::new();
        policy.set_active_roles(active_launcher, active_ime);

        let mut device_packages = Vec::with_capacity(all_user_packages.len());

        for pkg in all_user_packages {
            let in_system_set = system_packages.contains(&pkg);

            // Detect Android Package Type: System, UpdatedSystem, or User
            let package_type = if in_system_set {
                if let Some(path) = system_file_map.get(&pkg) {
                    if path.starts_with("/data/") || path.contains("/data/app/") {
                        AndroidPackageType::UpdatedSystem
                    } else {
                        AndroidPackageType::System
                    }
                } else {
                    AndroidPackageType::System
                }
            } else {
                AndroidPackageType::User
            };
            let is_system = package_type != AndroidPackageType::User;

            // Determine accurate state
            let state = if enabled_packages.contains(&pkg) {
                PackageState::Enabled
            } else if disabled_packages.contains(&pkg) {
                PackageState::Disabled
            } else {
                PackageState::Uninstalled
            };

            // Enrich from embedded UAD-ng dataset
            // Keep "Miscellaneous" as a catalog category only; never assign Misc to unlisted packages.
            let (list_cat, removal_rec, desc, deps, needed) = if let Some(meta) = catalog.get(&pkg)
            {
                let d = if meta.description.trim().is_empty() {
                    "No description available in community database.".to_string()
                } else {
                    meta.description.clone()
                };
                (
                    meta.list.clone(),
                    meta.removal.clone(),
                    d,
                    meta.dependencies.clone(),
                    meta.needed_by.clone(),
                )
            } else {
                (
                    "Unlisted".into(),
                    "Unlisted".into(),
                    match package_type {
                        AndroidPackageType::System => {
                            "System application (unlisted in debloat catalog)".into()
                        }
                        AndroidPackageType::UpdatedSystem => {
                            "Updated system application (unlisted in debloat catalog)".into()
                        }
                        AndroidPackageType::User => {
                            "Third-party user application (unlisted in debloat catalog)".into()
                        }
                    },
                    Vec::new(),
                    Vec::new(),
                )
            };

            let prot = policy.check_protection(&pkg);
            let is_utility = crate::policy::SafetyPolicy::is_built_in_utility(&pkg);
            let is_uncertain_vendor = crate::policy::SafetyPolicy::is_uncertain_vendor_package(&pkg);

            // Current removal eligibility:
            // Catalog "Recommended" rating must NEVER override device protection rules or imply safety!
            let removal_eligibility = if state == PackageState::Uninstalled {
                RemovalEligibility::AlreadyRemoved
            } else if prot.is_protected {
                RemovalEligibility::Protected
            } else if is_utility || is_uncertain_vendor {
                RemovalEligibility::ReviewOnly
            } else if removal_rec.eq_ignore_ascii_case("recommended") {
                RemovalEligibility::Eligible
            } else {
                RemovalEligibility::ReviewOnly
            };

            device_packages.push(DevicePackage {
                package_id: pkg,
                state,
                is_system,
                package_type,
                user_id,
                list_category: list_cat,
                removal_recommendation: removal_rec,
                removal_eligibility,
                description: desc,
                dependencies: deps,
                needed_by: needed,
                is_protected: prot.is_protected,
                protection_reason: prot.reason,
            });
        }

        // Sort by package_id alphabetically
        device_packages.sort_by(|a, b| a.package_id.cmp(&b.package_id));
        Ok(device_packages)
    }

    /// Query the single package state on target device
    pub fn query_single_package_state(
        executor: &AdbExecutor,
        serial: &str,
        user_id: u32,
        package_id: &str,
    ) -> Result<Option<PackageState>, String> {
        let enabled = Self::list_packages_with_flags(executor, serial, user_id, Some("-e"))?;
        if enabled.contains(package_id) {
            return Ok(Some(PackageState::Enabled));
        }

        let disabled = Self::list_packages_with_flags(executor, serial, user_id, Some("-d"))?;
        if disabled.contains(package_id) {
            return Ok(Some(PackageState::Disabled));
        }

        let all = Self::list_packages_with_flags(executor, serial, user_id, Some("-u"))?;
        if all.contains(package_id) {
            return Ok(Some(PackageState::Uninstalled));
        }

        Ok(None)
    }

    /// Query the device's currently active default launcher package
    pub fn query_active_launcher(executor: &AdbExecutor, serial: &str) -> Option<String> {
        let res = executor
            .run_shell(
                serial,
                &[
                    "cmd",
                    "package",
                    "resolve-activity",
                    "--brief",
                    "-c",
                    "android.intent.category.HOME",
                ],
            )
            .ok()?;

        if !res.success {
            return None;
        }

        // Example output line: "com.google.android.apps.nexuslauncher/.NexusLauncherActivity"
        for line in res.stdout.lines() {
            let trimmed = line.trim();
            if let Some(slash_idx) = trimmed.find('/') {
                let pkg = &trimmed[..slash_idx];
                if is_valid_package_name(pkg) {
                    return Some(pkg.to_string());
                }
            }
        }
        None
    }

    /// Query the device's currently active default input method (keyboard) package
    pub fn query_active_ime(executor: &AdbExecutor, serial: &str) -> Option<String> {
        let res = executor
            .run_shell(
                serial,
                &["settings", "get", "secure", "default_input_method"],
            )
            .ok()?;

        if !res.success {
            return None;
        }

        let trimmed = res.stdout.trim();
        if let Some(slash_idx) = trimmed.find('/') {
            let pkg = &trimmed[..slash_idx];
            if is_valid_package_name(pkg) {
                return Some(pkg.to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_list_crlf_and_lf() {
        let output = "package:com.android.settings\r\npackage:com.google.android.gms\npackage:com.sec.android.app.myfiles\r\n";
        let parsed = PackageManager::parse_package_list(output);
        assert_eq!(parsed.len(), 3);
        assert!(parsed.contains("com.android.settings"));
        assert!(parsed.contains("com.google.android.gms"));
        assert!(parsed.contains("com.sec.android.app.myfiles"));
    }

    #[test]
    fn test_parse_package_list_rejects_injections() {
        let output = "package:com.android.settings\npackage:com.malicious; rm -rf /\npackage:com.test && echo 1\n";
        let parsed = PackageManager::parse_package_list(output);
        assert_eq!(parsed.len(), 1);
        assert!(parsed.contains("com.android.settings"));
    }

    #[test]
    fn test_parse_package_file_paths() {
        let output = "package:/system/priv-app/Settings/Settings.apk=com.android.settings\r\npackage:/data/app/~~abc/com.example.updated-123/base.apk=com.example.updated\n";
        let map = PackageManager::parse_package_file_paths(output);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("com.android.settings").unwrap(), "/system/priv-app/Settings/Settings.apk");
        assert_eq!(map.get("com.example.updated").unwrap(), "/data/app/~~abc/com.example.updated-123/base.apk");
    }

    #[test]
    fn test_catalog_recommended_never_overrides_device_protection() {
        // Even if UAD-ng recommends removal, if device policy protects the package,
        // removal_eligibility must be Protected, not Eligible!
        let removal_rec = "Recommended";
        let is_protected = true;
        let state = PackageState::Enabled;
        let is_utility = false;
        let is_uncertain_vendor = false;

        let removal_eligibility = if state == PackageState::Uninstalled {
            RemovalEligibility::AlreadyRemoved
        } else if is_protected {
            RemovalEligibility::Protected
        } else if is_utility || is_uncertain_vendor {
            RemovalEligibility::ReviewOnly
        } else if removal_rec.eq_ignore_ascii_case("recommended") {
            RemovalEligibility::Eligible
        } else {
            RemovalEligibility::ReviewOnly
        };

        assert_eq!(removal_eligibility, RemovalEligibility::Protected);
    }
}
