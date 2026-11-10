//! Standalone Terminal CLI for TZ AI DEBLOATER TOOL (`tz-debloater`).
//!
//! Provides full terminal-driven Android package inspection, policy validation,
//! debloating, safety checks, audit journaling, recovery, and configuration
//! without requiring a running GUI or Tauri session.

use std::io::{self, Write};
use std::process::ExitCode;

use tz_ai_debloater_lib::adb::{
    ActionType, AdbExecutor, AdbLocator, AndroidPackageType, DeviceInfo, DeviceManager,
    DevicePackage, OperationOutcome, PackageManager, PackageState, RemovalEligibility,
    UserManager,
};
use tz_ai_debloater_lib::ai::{ActionPlan, ProposedAction, ShortcutIntent};
use tz_ai_debloater_lib::execution;
use tz_ai_debloater_lib::journal::{JournalEntry, JournalManager};
use tz_ai_debloater_lib::policy::SafetyPolicy;
use tz_ai_debloater_lib::storage::credentials::{ConfigStore, CredentialStore};

const EXIT_SUCCESS: u8 = 0;
const EXIT_RUNTIME_ERROR: u8 = 1;
const EXIT_USAGE_ERROR: u8 = 2;
const EXIT_CONFIRMATION_DECLINED: u8 = 3;
const EXIT_POLICY_VIOLATION: u8 = 4;
const EXIT_DEVICE_UNAVAILABLE: u8 = 5;

const VERSION: &str = "1.1.0";

fn print_version() {
    println!("tz-debloater v{VERSION} (TZ Labs - Intelligent Android Maintenance)");
}

fn print_help() {
    println!(
        r#"tz-debloater v{VERSION} - Intelligent Android Maintenance & Package Console

USAGE:
    tz-debloater <COMMAND> [OPTIONS]

COMMANDS:
    doctor                       Inspect environment, ADB status, OS keyring, and journal
    devices                      List attached Android devices and connection states
    users    [-d <serial>]       List Android user profiles for the selected device
    scan     [OPTIONS]           Scan and list packages for target device and user profile
    packages [OPTIONS]           Alias for 'scan'
    candidates [OPTIONS]         Shorthand for 'scan --view candidates'
    plan     [OPTIONS]           Generate a debloating action plan with safety validation
    apply    [OPTIONS]           Execute approved package modifications with audit logging
    history  [OPTIONS]           View audit journal history of package modifications
    restore  [OPTIONS]           Reinstall or re-enable packages back to prior verified states
    config   <SUBCOMMAND>        Manage connection profiles, secure credentials, and endpoints
    models   [OPTIONS]           Discover available LLM models for the active connection profile
    version                      Print version information
    help                         Print this help message

GLOBAL OPTIONS:
    -d, --device <serial>        Target device serial number
    -u, --user <id>              Target Android user profile ID (default: 0)
    -y, --yes                    Skip interactive confirmation prompts
    --json                       Output machine-readable JSON
    -v, --version                Print version information
    -h, --help                   Print help information

EXIT CODES:
    0    Success
    1    Runtime failure / execution error
    2    Invalid command arguments or syntax error
    3    Operation cancelled / user confirmation declined
    4    Safety policy violation (attempting to remove critical/protected package)
    5    Device offline, device not found, or ADB unavailable

Run 'tz-debloater <COMMAND> --help' for command-specific options and examples.
"#
    );
}

#[derive(Clone)]
struct ParsedArgs {
    command: String,
    device: Option<String>,
    user: u32,
    view: Option<String>,
    search: Option<String>,
    rating: Option<String>,
    pkg_type: Option<String>,
    pkg_state: Option<String>,
    intent: Option<String>,
    packages: Vec<String>,
    action: Option<ActionType>,
    plan_file: Option<String>,
    #[allow(dead_code)]
    prompt: Option<String>,
    limit: usize,
    latest: bool,
    yes: bool,
    json: bool,
    subargs: Vec<String>,
}

fn parse_cli_args(raw: &[String]) -> Result<ParsedArgs, String> {
    let mut command = String::new();
    let mut device = None;
    let mut user = 0;
    let mut view = None;
    let mut search = None;
    let mut rating = None;
    let mut pkg_type = None;
    let mut pkg_state = None;
    let mut intent = None;
    let mut packages = Vec::new();
    let mut action = None;
    let mut plan_file = None;
    let mut prompt = None;
    let mut limit = 20;
    let mut latest = false;
    let mut yes = false;
    let mut json = false;
    let mut subargs = Vec::new();

    let mut i = 0;
    while i < raw.len() {
        let arg = &raw[i];
        if arg == "-h" || arg == "--help" || arg == "help" {
            if command.is_empty() {
                command = "help".into();
            } else {
                subargs.push("--help".into());
            }
            i += 1;
            continue;
        }
        if arg == "-v" || arg == "--version" || arg == "version" {
            if command.is_empty() {
                command = "version".into();
            } else {
                subargs.push("--version".into());
            }
            i += 1;
            continue;
        }
        if arg == "-y" || arg == "--yes" {
            yes = true;
            i += 1;
            continue;
        }
        if arg == "--json" {
            json = true;
            i += 1;
            continue;
        }
        if arg == "-d" || arg == "--device" {
            if i + 1 < raw.len() {
                device = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --device".into());
            }
        }
        if arg == "-u" || arg == "--user" {
            if i + 1 < raw.len() {
                user = raw[i + 1]
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid user ID '{}'", raw[i + 1]))?;
                i += 2;
                continue;
            } else {
                return Err("Missing value for --user".into());
            }
        }
        if arg == "--view" {
            if i + 1 < raw.len() {
                view = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --view".into());
            }
        }
        if arg == "--search" {
            if i + 1 < raw.len() {
                search = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --search".into());
            }
        }
        if arg == "--rating" {
            if i + 1 < raw.len() {
                rating = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --rating".into());
            }
        }
        if arg == "--type" {
            if i + 1 < raw.len() {
                pkg_type = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --type".into());
            }
        }
        if arg == "--state" {
            if i + 1 < raw.len() {
                pkg_state = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --state".into());
            }
        }
        if arg == "--intent" {
            if i + 1 < raw.len() {
                intent = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --intent".into());
            }
        }
        if arg == "--packages" {
            if i + 1 < raw.len() {
                packages = raw[i + 1]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 2;
                continue;
            } else {
                return Err("Missing value for --packages".into());
            }
        }
        if arg == "--action" {
            if i + 1 < raw.len() {
                let act_str = raw[i + 1].to_lowercase();
                let act = match act_str.as_str() {
                    "uninstall" | "uninstall-user" | "remove" => ActionType::UninstallUser,
                    "disable" => ActionType::Disable,
                    "enable" => ActionType::Enable,
                    "restore" | "install-existing" => ActionType::Restore,
                    _ => return Err(format!("Unknown action '{act_str}'. Use uninstall, disable, enable, or restore.")),
                };
                action = Some(act);
                i += 2;
                continue;
            } else {
                return Err("Missing value for --action".into());
            }
        }
        if arg == "--plan" {
            if i + 1 < raw.len() {
                plan_file = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --plan".into());
            }
        }
        if arg == "--prompt" {
            if i + 1 < raw.len() {
                prompt = Some(raw[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("Missing value for --prompt".into());
            }
        }
        if arg == "--limit" {
            if i + 1 < raw.len() {
                limit = raw[i + 1]
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit '{}'", raw[i + 1]))?;
                i += 2;
                continue;
            } else {
                return Err("Missing value for --limit".into());
            }
        }
        if arg == "--latest" {
            latest = true;
            i += 1;
            continue;
        }

        if command.is_empty() && !arg.starts_with('-') {
            command = arg.clone();
            i += 1;
            continue;
        }

        subargs.push(arg.clone());
        i += 1;
    }

    Ok(ParsedArgs {
        command,
        device,
        user,
        view,
        search,
        rating,
        pkg_type,
        pkg_state,
        intent,
        packages,
        action,
        plan_file,
        prompt,
        limit,
        latest,
        yes,
        json,
        subargs,
    })
}

fn get_executor() -> Result<AdbExecutor, (String, u8)> {
    let cfg = ConfigStore::load_config();
    let status = AdbLocator::locate(cfg.custom_adb_path.as_deref());
    if let Some(path) = status.path {
        Ok(AdbExecutor::new(path))
    } else {
        Err((
            status.error.unwrap_or_else(|| {
                "ADB binary not found. Install Android SDK platform-tools or run 'doctor'."
                    .to_string()
            }),
            EXIT_DEVICE_UNAVAILABLE,
        ))
    }
}

fn resolve_target_device(
    executor: &AdbExecutor,
    specified: Option<&str>,
) -> Result<DeviceInfo, (String, u8)> {
    let devices = DeviceManager::list_devices(executor)
        .map_err(|e| (format!("Failed to list ADB devices: {e}"), EXIT_DEVICE_UNAVAILABLE))?;

    if let Some(serial) = specified {
        devices
            .into_iter()
            .find(|d| d.serial == serial)
            .ok_or_else(|| {
                (
                    format!("Device with serial '{serial}' was not found among connected ADB devices."),
                    EXIT_DEVICE_UNAVAILABLE,
                )
            })
    } else {
        let ready: Vec<DeviceInfo> = devices.into_iter().filter(|d| d.state.is_ready()).collect();
        match ready.len() {
            0 => Err((
                "No ready Android devices connected. Connect via USB or wireless ADB.".into(),
                EXIT_DEVICE_UNAVAILABLE,
            )),
            1 => Ok(ready.into_iter().next().unwrap()),
            _ => Err((
                "Multiple Android devices connected. Please specify the target with -d <serial>."
                    .into(),
                EXIT_USAGE_ERROR,
            )),
        }
    }
}

// -----------------------------------------------------------------------------
// Subcommand Implementations
// -----------------------------------------------------------------------------

fn cmd_doctor(json: bool) -> Result<(), (String, u8)> {
    let cfg = ConfigStore::load_config();
    let adb_status = AdbLocator::locate(cfg.custom_adb_path.as_deref());
    let keyring_backend = CredentialStore::backend_name();

    let mut devices_count = 0;
    let mut ready_count = 0;
    let mut devices_list = Vec::new();

    if let Some(ref path) = adb_status.path {
        let exec = AdbExecutor::new(path.clone());
        if let Ok(devs) = DeviceManager::list_devices(&exec) {
            devices_count = devs.len();
            ready_count = devs.iter().filter(|d| d.state.is_ready()).count();
            devices_list = devs;
        }
    }

    let journal_entries = JournalManager::read_journal().unwrap_or_default();
    let snapshots = JournalManager::list_snapshots().unwrap_or_default();

    if json {
        let output = serde_json::json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "version": VERSION,
            "adb": {
                "found": adb_status.found,
                "path": adb_status.path,
                "version": adb_status.version,
                "source": adb_status.source,
                "error": adb_status.error,
            },
            "security": {
                "keyring_backend": keyring_backend,
                "ai_consent_given": cfg.ai_consent_given,
                "approved_destinations_count": cfg.approved_destinations.len(),
            },
            "storage": {
                "journal_entries_count": journal_entries.len(),
                "snapshots_count": snapshots.len(),
            },
            "devices": {
                "total": devices_count,
                "ready": ready_count,
                "list": devices_list,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("TZ AI DEBLOATER TOOL - System Doctor (v{VERSION})");
        println!("==================================================");
        println!("Host OS:             {} ({})", std::env::consts::OS, std::env::consts::ARCH);
        println!("Keyring Store:       {}", keyring_backend);
        println!("AI Data Consent:     {}", if cfg.ai_consent_given { "Granted" } else { "Not accepted" });
        println!("Active AI Profile:   {}", cfg.active_connection_id);
        println!("Approved Endpoints:  {} host(s)", cfg.approved_destinations.len());
        println!("Audit Journal:       {} entries recorded", journal_entries.len());
        println!("Safety Snapshots:    {} saved", snapshots.len());
        println!("--------------------------------------------------");
        if adb_status.found {
            println!("ADB Status:          Available");
            println!("ADB Path:            {}", adb_status.path.unwrap_or_default());
            println!("ADB Version:         {}", adb_status.version.unwrap_or_else(|| "Unknown".into()));
            println!("ADB Detection:       {}", adb_status.source.unwrap_or_else(|| "Standard".into()));
        } else {
            println!("ADB Status:          NOT FOUND");
            println!("Error:               {}", adb_status.error.unwrap_or_else(|| "Binary missing".into()));
            println!("Suggestion:          Install Android SDK platform-tools or set custom path in config.");
        }
        println!("--------------------------------------------------");
        println!("Connected Devices:   {} total ({} ready)", devices_count, ready_count);
        for dev in &devices_list {
            println!(
                "  - {:<18} {:<12} {:<15} Android {:<4} [{}]",
                dev.serial, dev.brand, dev.model, dev.android_version, dev.state.as_str()
            );
        }
        println!("==================================================");
    }
    Ok(())
}

fn cmd_devices(json: bool) -> Result<(), (String, u8)> {
    let exec = get_executor()?;
    let devices = DeviceManager::list_devices(&exec)
        .map_err(|e| (format!("Failed to list devices: {e}"), EXIT_DEVICE_UNAVAILABLE))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&devices).unwrap());
    } else {
        if devices.is_empty() {
            println!("No Android devices detected via ADB.");
            println!("Ensure USB debugging or Wireless ADB is enabled on your device.");
            return Ok(());
        }
        println!(
            "{:<20} {:<12} {:<16} {:<10} {:<12} {:<10}",
            "SERIAL", "BRAND", "MODEL", "ANDROID", "STATE", "CONNECTION"
        );
        println!("{}", "-".repeat(84));
        for d in devices {
            println!(
                "{:<20} {:<12} {:<16} {:<10} {:<12} {:<10}",
                d.serial,
                if d.brand.is_empty() { "—" } else { &d.brand },
                if d.model.is_empty() { "—" } else { &d.model },
                if d.android_version.is_empty() { "—" } else { &d.android_version },
                d.state.as_str(),
                d.connection_type
            );
        }
    }
    Ok(())
}

fn cmd_users(device: Option<&str>, json: bool) -> Result<(), (String, u8)> {
    let exec = get_executor()?;
    let target = resolve_target_device(&exec, device)?;
    let users = UserManager::list_users(&exec, &target.serial)
        .map_err(|e| (format!("Failed to list Android users: {e}"), EXIT_RUNTIME_ERROR))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&users).unwrap());
    } else {
        println!("Android User Profiles for {} ({})", target.serial, target.model);
        println!("{:<8} {:<24} {:<12} {:<10}", "USER ID", "NAME", "FLAGS", "RUNNING");
        println!("{}", "-".repeat(58));
        for u in users {
            println!(
                "{:<8} {:<24} {:<12} {:<10}",
                u.id,
                u.name,
                format!("0x{:04x}", u.flags),
                if u.is_running { "Yes" } else { "No" }
            );
        }
    }
    Ok(())
}

fn cmd_scan(args: &ParsedArgs) -> Result<(), (String, u8)> {
    let exec = get_executor()?;
    let target = resolve_target_device(&exec, args.device.as_deref())?;
    let all_packages = PackageManager::scan_device_packages(&exec, &target.serial, args.user)
        .map_err(|e| (format!("Failed to scan packages: {e}"), EXIT_RUNTIME_ERROR))?;

    let view = args.view.as_deref().unwrap_or("candidates").to_lowercase();
    let search_lower = args.search.as_deref().map(|s| s.to_lowercase());
    let rating_filter = args.rating.as_deref().map(|s| s.to_lowercase());
    let type_filter = args.pkg_type.as_deref().map(|s| s.to_lowercase());
    let state_filter = args.pkg_state.as_deref().map(|s| s.to_lowercase());

    let filtered: Vec<&DevicePackage> = all_packages
        .iter()
        .filter(|p| {
            // View filtering
            match view.as_str() {
                "candidates" => {
                    p.removal_eligibility == RemovalEligibility::Eligible
                        && p.state != PackageState::Uninstalled
                }
                "system" => {
                    p.package_type == AndroidPackageType::System
                        || p.package_type == AndroidPackageType::UpdatedSystem
                }
                "removed" => p.state == PackageState::Uninstalled,
                "all" => true,
                _ => true,
            }
        })
        .filter(|p| {
            if let Some(ref q) = search_lower {
                let id_match = p.package_id.to_lowercase().contains(q);
                let desc_match = p.description.to_lowercase().contains(q);
                id_match || desc_match
            } else {
                true
            }
        })
        .filter(|p| {
            if let Some(ref r) = rating_filter {
                p.removal_recommendation.to_lowercase() == *r
            } else {
                true
            }
        })
        .filter(|p| {
            if let Some(ref t) = type_filter {
                match t.as_str() {
                    "system" => p.package_type == AndroidPackageType::System,
                    "updated" | "updated-system" => p.package_type == AndroidPackageType::UpdatedSystem,
                    "user" => p.package_type == AndroidPackageType::User,
                    _ => true,
                }
            } else {
                true
            }
        })
        .filter(|p| {
            if let Some(ref s) = state_filter {
                match s.as_str() {
                    "enabled" => p.state == PackageState::Enabled,
                    "disabled" => p.state == PackageState::Disabled,
                    "uninstalled" => p.state == PackageState::Uninstalled,
                    _ => true,
                }
            } else {
                true
            }
        })
        .collect();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&filtered).unwrap());
    } else {
        println!(
            "Package Inventory: {} (User {}) · View: '{}' ({} matching out of {} total)",
            target.serial, args.user, view, filtered.len(), all_packages.len()
        );
        println!(
            "{:<42} {:<14} {:<12} {:<15} {:<12} {:<12}",
            "PACKAGE ID", "CATEGORY", "RATING", "TYPE", "STATE", "ELIGIBILITY"
        );
        println!("{}", "-".repeat(110));
        for p in &filtered {
            println!(
                "{:<42} {:<14} {:<12} {:<15} {:<12} {:<12}",
                truncate_str(&p.package_id, 40),
                truncate_str(&p.list_category, 13),
                p.removal_recommendation,
                p.package_type.display_name(),
                p.state.as_str(),
                p.removal_eligibility.display_name(),
            );
        }
        println!("{}", "-".repeat(110));
        println!(
            "Showing {} package(s). Use 'tz-debloater plan' or 'apply' to modify states.",
            filtered.len()
        );
    }
    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

fn cmd_plan(args: &ParsedArgs) -> Result<(), (String, u8)> {
    let exec = get_executor()?;
    let target = resolve_target_device(&exec, args.device.as_deref())?;
    let packages = PackageManager::scan_device_packages(&exec, &target.serial, args.user)
        .map_err(|e| (format!("Failed to scan packages: {e}"), EXIT_RUNTIME_ERROR))?;

    let launcher = DeviceManager::get_active_launcher_for_user(&exec, &target.serial, args.user);
    let ime = DeviceManager::get_active_ime_for_user(&exec, &target.serial, args.user);

    let mut policy = SafetyPolicy::new();
    policy.set_active_roles(launcher.clone(), ime.clone());

    let mut proposed = Vec::new();

    if !args.packages.is_empty() {
        let action = args.action.unwrap_or(ActionType::Disable);
        for pkg_id in &args.packages {
            let pkg_info = packages.iter().find(|p| &p.package_id == pkg_id);
            let is_allowed = policy.is_action_allowed(pkg_id, action);
            let (is_prot, warning) = match is_allowed {
                Ok(_) => (false, None),
                Err(e) => (true, Some(e)),
            };

            proposed.push(ProposedAction {
                package_id: pkg_id.clone(),
                package_name: None,
                proposed_action: action,
                reason: format!("User requested direct {} for this package", action.display_name()),
                evidence: "Manual command specification".into(),
                expected_side_effects: None,
                uncertainty: "Low".into(),
                removal_category: pkg_info.map(|p| p.list_category.clone()).unwrap_or_else(|| "Manual".into()),
                is_protected: is_prot,
                protection_warning: warning,
                selected: true,
                is_executable: !is_prot,
            });
        }
    } else if let Some(ref intent_str) = args.intent {
        let intent = match intent_str.to_lowercase().as_str() {
            "bloatware" => ShortcutIntent::ReviewBloatware,
            "trackers" | "tracker" => ShortcutIntent::ReviewTrackers,
            "promos" | "promo" => ShortcutIntent::UninstallPromos,
            _ => return Err((format!("Unknown intent '{intent_str}'. Use bloatware, trackers, or promos."), EXIT_USAGE_ERROR)),
        };

        for p in &packages {
            if p.state == PackageState::Uninstalled {
                continue;
            }
            let is_match = match intent {
                ShortcutIntent::ReviewTrackers => {
                    tz_ai_debloater_lib::ai::AiClient::is_tracker_or_analytics(&p.package_id)
                }
                ShortcutIntent::UninstallPromos => {
                    tz_ai_debloater_lib::ai::AiClient::is_promotional(&p.package_id)
                }
                ShortcutIntent::ReviewBloatware => {
                    p.removal_eligibility == RemovalEligibility::Eligible
                }
            };

            if is_match {
                let action = match intent {
                    ShortcutIntent::UninstallPromos => ActionType::UninstallUser,
                    _ => ActionType::Disable,
                };
                let check = policy.is_action_allowed(&p.package_id, action);
                let (is_prot, warn) = match check {
                    Ok(_) => (false, None),
                    Err(e) => (true, Some(e)),
                };

                proposed.push(ProposedAction {
                    package_id: p.package_id.clone(),
                    package_name: None,
                    proposed_action: action,
                    reason: format!("Matched catalog heuristic for {:?}", intent),
                    evidence: format!("Catalog rating: {}, category: {}", p.removal_recommendation, p.list_category),
                    expected_side_effects: None,
                    uncertainty: "Low".into(),
                    removal_category: p.list_category.clone(),
                    is_protected: is_prot,
                    protection_warning: warn,
                    selected: true,
                    is_executable: !is_prot,
                });
            }
        }
    } else {
        return Err((
            "Please specify packages with --packages <id,id,...> or an intent with --intent <bloatware|trackers|promos>."
                .into(),
            EXIT_USAGE_ERROR,
        ));
    }

    let plan = ActionPlan {
        id: uuid::Uuid::new_v4().to_string(),
        revision: 1,
        target_device_serial: target.serial.clone(),
        target_user_id: args.user,
        explanation: format!("Plan generated for device {} (User {})", target.serial, args.user),
        preserved_functionality: vec![
            format!("Active launcher preserved: {:?}", launcher.unwrap_or_else(|| "None".into())),
            format!("Active keyboard preserved: {:?}", ime.unwrap_or_else(|| "None".into())),
        ],
        actions: proposed,
        created_at: chrono::Utc::now().timestamp(),
        approved_by_user: false,
        is_valid: true,
        invalidation_reason: None,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&plan).unwrap());
    } else {
        println!("Proposed Debloat Plan ({})", plan.id);
        println!("Target Device: {} · User Profile: {}", plan.target_device_serial, plan.target_user_id);
        println!("{:<40} {:<16} {:<12} {:<12} {:<24}", "PACKAGE ID", "ACTION", "PROTECTED", "EXECUTABLE", "REASON");
        println!("{}", "-".repeat(108));
        for a in &plan.actions {
            println!(
                "{:<40} {:<16} {:<12} {:<12} {:<24}",
                truncate_str(&a.package_id, 38),
                a.proposed_action.display_name(),
                if a.is_protected { "YES (BLOCKED)" } else { "No" },
                if a.is_executable { "Yes" } else { "No" },
                truncate_str(&a.reason, 22)
            );
        }
        println!("{}", "-".repeat(108));
        let executable_count = plan.actions.iter().filter(|a| a.is_executable).count();
        let blocked_count = plan.actions.iter().filter(|a| a.is_protected).count();
        println!(
            "Total proposed: {} action(s) ({} executable, {} blocked by policy).",
            plan.actions.len(), executable_count, blocked_count
        );
        println!("To apply this plan, run: tz-debloater apply --packages <pkg1,pkg2> --action <action>");
    }
    Ok(())
}

fn cmd_apply(args: &ParsedArgs) -> Result<(), (String, u8)> {
    let exec = get_executor()?;
    let target = resolve_target_device(&exec, args.device.as_deref())?;

    let launcher = DeviceManager::get_active_launcher_for_user(&exec, &target.serial, args.user);
    let ime = DeviceManager::get_active_ime_for_user(&exec, &target.serial, args.user);

    let mut policy = SafetyPolicy::new();
    policy.set_active_roles(launcher.clone(), ime.clone());

    // Collect actions to execute: either from --plan file or --packages + --action
    let mut actions_to_run: Vec<(String, ActionType)> = Vec::new();

    if let Some(ref path) = args.plan_file {
        let content = std::fs::read_to_string(path)
            .map_err(|e| (format!("Failed to read plan file '{path}': {e}"), EXIT_USAGE_ERROR))?;
        let plan: ActionPlan = serde_json::from_str(&content)
            .map_err(|e| (format!("Invalid plan file JSON format: {e}"), EXIT_USAGE_ERROR))?;
        for a in plan.actions {
            if a.selected && a.is_executable {
                actions_to_run.push((a.package_id, a.proposed_action));
            }
        }
    } else if !args.packages.is_empty() {
        let action = args.action.unwrap_or(ActionType::Disable);
        for p in &args.packages {
            actions_to_run.push((p.clone(), action));
        }
    } else {
        return Err((
            "No packages specified to apply. Provide --packages <id,id,...> or --plan <file.json>."
                .into(),
            EXIT_USAGE_ERROR,
        ));
    }

    if actions_to_run.is_empty() {
        return Err(("No executable actions in the plan/request.".into(), EXIT_USAGE_ERROR));
    }

    // Safety policy preflight validation
    for (pkg, act) in &actions_to_run {
        if let Err(reason) = policy.is_action_allowed(pkg, *act) {
            eprintln!("[POLICY VIOLATION] Action '{}' on package '{}' is prohibited: {}", act.display_name(), pkg, reason);
            return Err((
                format!("Safety policy blocked operation on '{pkg}': {reason}"),
                EXIT_POLICY_VIOLATION,
            ));
        }
    }

    // Interactive confirmation prompt unless -y / --yes
    if !args.yes {
        println!("==================================================");
        println!("PRE-EXECUTION CONFIRMATION");
        println!("Target Device:   {} ({})", target.serial, target.model);
        println!("Target User:     {}", args.user);
        println!("Planned Actions: {} item(s)", actions_to_run.len());
        println!("--------------------------------------------------");
        for (pkg, act) in &actions_to_run {
            println!("  • {:<16} {}", act.display_name(), pkg);
        }
        println!("==================================================");
        print!("Proceed with executing these changes on the connected device? [y/N]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            println!("Operation cancelled by user.");
            return Err(("Confirmation declined by user.".into(), EXIT_CONFIRMATION_DECLINED));
        }
    }

    println!("Applying {} operation(s) to {} (User {})...", actions_to_run.len(), target.serial, args.user);

    let mut outcomes = Vec::new();
    let mut any_failed = false;

    for (pkg, act) in actions_to_run {
        match execution::run_recorded(&exec, &target.serial, args.user, &pkg, act) {
            Ok(outcome) => {
                if !outcome.success {
                    any_failed = true;
                }
                outcomes.push(outcome);
            }
            Err(e) => {
                any_failed = true;
                outcomes.push(OperationOutcome {
                    package_id: pkg.clone(),
                    action: act,
                    success: false,
                    verified: false,
                    exit_code: Some(1),
                    previous_state: None,
                    resulting_state: None,
                    message: format!("Execution failed: {e}"),
                    classified_cause: tz_ai_debloater_lib::adb::ClassifiedCause::None,
                    execution_backend: "DirectAdbPm".into(),
                    command_args: Vec::new(),
                    target_device: target.serial.clone(),
                    target_user_id: args.user,
                    raw_stdout: None,
                    raw_stderr: Some(e),
                });
            }
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&outcomes).unwrap());
    } else {
        println!("\nExecution Summary:");
        println!("{:<38} {:<14} {:<10} {:<10} {:<10} {:<24}", "PACKAGE ID", "ACTION", "SUCCESS", "BEFORE", "AFTER", "MESSAGE");
        println!("{}", "-".repeat(110));
        for o in &outcomes {
            println!(
                "{:<38} {:<14} {:<10} {:<10} {:<10} {:<24}",
                truncate_str(&o.package_id, 36),
                o.action.display_name(),
                if o.success { "Yes" } else { "FAILED" },
                o.previous_state.map(|s| s.as_str()).unwrap_or("—"),
                o.resulting_state.map(|s| s.as_str()).unwrap_or("—"),
                truncate_str(&o.message, 22)
            );
        }
        println!("{}", "-".repeat(110));
    }

    if any_failed {
        Err(("One or more operations failed. Inspect the output above for diagnostics.".into(), EXIT_RUNTIME_ERROR))
    } else {
        Ok(())
    }
}

fn cmd_history(args: &ParsedArgs) -> Result<(), (String, u8)> {
    let entries = JournalManager::read_journal()
        .map_err(|e| (format!("Failed to read journal: {e}"), EXIT_RUNTIME_ERROR))?;

    let filtered: Vec<&JournalEntry> = entries
        .iter()
        .filter(|e| {
            if let Some(ref d) = args.device {
                &e.device_serial == d
            } else {
                true
            }
        })
        .rev()
        .take(args.limit)
        .collect();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&filtered).unwrap());
    } else {
        println!("Audit Journal History (showing up to {} entries)", args.limit);
        println!("{:<20} {:<16} {:<6} {:<34} {:<14} {:<10}", "TIMESTAMP", "DEVICE", "USER", "PACKAGE ID", "ACTION", "RESULT");
        println!("{}", "-".repeat(104));
        for e in filtered {
            let dt = chrono::DateTime::from_timestamp(e.timestamp, 0)
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| e.timestamp.to_string());
            println!(
                "{:<20} {:<16} {:<6} {:<34} {:<14} {:<10}",
                dt,
                truncate_str(&e.device_serial, 14),
                e.user_id,
                truncate_str(&e.package_id, 32),
                e.action.display_name(),
                if e.success { "Success" } else { "Failed" }
            );
        }
    }
    Ok(())
}

fn cmd_restore(args: &ParsedArgs) -> Result<(), (String, u8)> {
    let exec = get_executor()?;
    let target = resolve_target_device(&exec, args.device.as_deref())?;
    let packages = PackageManager::scan_device_packages(&exec, &target.serial, args.user)
        .map_err(|e| (format!("Failed to scan packages: {e}"), EXIT_RUNTIME_ERROR))?;

    let mut to_restore = Vec::new();

    if args.latest {
        let entries = JournalManager::read_journal()
            .map_err(|e| (format!("Failed to read journal: {e}"), EXIT_RUNTIME_ERROR))?;
        let matching: Vec<&JournalEntry> = entries
            .iter()
            .filter(|e| e.device_serial == target.serial && e.user_id == args.user && e.success)
            .collect();
        if let Some(last) = matching.last() {
            let current = packages.iter().find(|p| p.package_id == last.package_id);
            let action = match current.map(|p| p.state) {
                Some(PackageState::Uninstalled) => ActionType::Restore,
                _ => ActionType::Enable,
            };
            to_restore.push((last.package_id.clone(), action));
        } else {
            return Err(("No journal entries found to restore for this device/user.".into(), EXIT_USAGE_ERROR));
        }
    } else if !args.packages.is_empty() {
        for pkg_id in &args.packages {
            let current = packages.iter().find(|p| &p.package_id == pkg_id);
            let action = match current.map(|p| p.state) {
                Some(PackageState::Uninstalled) => ActionType::Restore,
                _ => ActionType::Enable,
            };
            to_restore.push((pkg_id.clone(), action));
        }
    } else {
        return Err(("Specify packages with --packages <id,id,...> or use --latest to restore the last modified package.".into(), EXIT_USAGE_ERROR));
    }

    if !args.yes {
        println!("Restoration Preview:");
        for (pkg, act) in &to_restore {
            println!("  • {:<16} {}", act.display_name(), pkg);
        }
        print!("Restore these packages on device {} (User {})? [y/N]: ", target.serial, args.user);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            println!("Restoration cancelled by user.");
            return Err(("Confirmation declined by user.".into(), EXIT_CONFIRMATION_DECLINED));
        }
    }

    println!("Restoring {} package(s)...", to_restore.len());
    let mut any_failed = false;
    let mut outcomes = Vec::new();

    for (pkg, act) in to_restore {
        match execution::run_recorded(&exec, &target.serial, args.user, &pkg, act) {
            Ok(outcome) => {
                if !outcome.success {
                    any_failed = true;
                }
                outcomes.push(outcome);
            }
            Err(e) => {
                any_failed = true;
                eprintln!("Failed to restore '{pkg}': {e}");
            }
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&outcomes).unwrap());
    } else {
        println!("\nRestoration Results:");
        for o in outcomes {
            println!(
                "  [{}] {} -> {}: {}",
                if o.success { "RESTORED" } else { "FAILED" },
                o.package_id,
                o.action.display_name(),
                o.message
            );
        }
    }

    if any_failed {
        Err(("One or more packages could not be restored.".into(), EXIT_RUNTIME_ERROR))
    } else {
        Ok(())
    }
}

fn cmd_config(args: &ParsedArgs) -> Result<(), (String, u8)> {
    let sub = args.subargs.first().map(|s| s.as_str()).unwrap_or("list");
    let mut cfg = ConfigStore::load_config();

    match sub {
        "list" => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&cfg).unwrap());
            } else {
                println!("TZ Debloater Configuration");
                println!("==================================================");
                println!("Active Profile:      {}", cfg.active_connection_id);
                println!("Custom ADB Path:     {}", cfg.custom_adb_path.as_deref().unwrap_or("None (auto-discover)"));
                println!("AI Consent Given:    {}", cfg.ai_consent_given);
                println!("Approved Hosts:      {}", cfg.approved_destinations.join(", "));
                println!("--------------------------------------------------");
                println!("Configured Connection Profiles ({} total):", cfg.connections.len());
                for c in &cfg.connections {
                    let has_key = CredentialStore::has_credential(&c.credential_id);
                    println!(
                        "  - {:<20} {:<12} {:<30} [Key: {}]",
                        c.id,
                        c.protocol,
                        c.base_url,
                        if has_key { "Stored" } else { "Missing" }
                    );
                }
                println!("==================================================");
            }
        }
        "set-active" => {
            let conn_id = args.subargs.get(1).ok_or_else(|| {
                ("Missing connection ID. Usage: tz-debloater config set-active <id>".into(), EXIT_USAGE_ERROR)
            })?;
            if !cfg.connections.iter().any(|c| &c.id == conn_id) {
                return Err((format!("Connection profile '{conn_id}' does not exist."), EXIT_USAGE_ERROR));
            }
            cfg.active_connection_id = conn_id.clone();
            ConfigStore::save_config(&cfg)
                .map_err(|e| (format!("Failed to save config: {e}"), EXIT_RUNTIME_ERROR))?;
            println!("Active connection profile set to '{conn_id}'.");
        }
        "set-key" => {
            let conn_id = args.subargs.get(1).ok_or_else(|| {
                ("Missing connection ID. Usage: tz-debloater config set-key <id> <secret>".into(), EXIT_USAGE_ERROR)
            })?;
            let secret = args.subargs.get(2).ok_or_else(|| {
                ("Missing API key secret. Usage: tz-debloater config set-key <id> <secret>".into(), EXIT_USAGE_ERROR)
            })?;
            CredentialStore::save_credential(conn_id, secret)
                .map_err(|e| (format!("Failed to store credential securely: {e}"), EXIT_RUNTIME_ERROR))?;
            println!("API key for connection '{conn_id}' saved to secure OS keyring.");
        }
        "remove-key" => {
            let conn_id = args.subargs.get(1).ok_or_else(|| {
                ("Missing connection ID. Usage: tz-debloater config remove-key <id>".into(), EXIT_USAGE_ERROR)
            })?;
            CredentialStore::delete_credential(conn_id)
                .map_err(|e| (format!("Failed to remove credential: {e}"), EXIT_RUNTIME_ERROR))?;
            println!("Credential for '{conn_id}' removed from secure store.");
        }
        "approve-host" => {
            let host = args.subargs.get(1).ok_or_else(|| {
                ("Missing host. Usage: tz-debloater config approve-host <hostname>".into(), EXIT_USAGE_ERROR)
            })?;
            let clean = host.trim().trim_end_matches('/').to_string();
            if !cfg.approved_destinations.contains(&clean) {
                cfg.approved_destinations.push(clean.clone());
                ConfigStore::save_config(&cfg)
                    .map_err(|e| (format!("Failed to save config: {e}"), EXIT_RUNTIME_ERROR))?;
                println!("Destination '{clean}' approved for device telemetry transmission.");
            } else {
                println!("Destination '{clean}' is already approved.");
            }
        }
        _ => {
            return Err((
                format!("Unknown config subcommand '{sub}'. Available: list, set-active, set-key, remove-key, approve-host"),
                EXIT_USAGE_ERROR,
            ));
        }
    }
    Ok(())
}

fn cmd_models(args: &ParsedArgs) -> Result<(), (String, u8)> {
    let cfg = ConfigStore::load_config();
    let conn_id = args.subargs.first().unwrap_or(&cfg.active_connection_id);
    let profile = cfg
        .connections
        .iter()
        .find(|c| &c.id == conn_id)
        .ok_or_else(|| (format!("Connection profile '{conn_id}' not found."), EXIT_USAGE_ERROR))?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| (format!("Failed to initialize async runtime: {e}"), EXIT_RUNTIME_ERROR))?;

    let cred = CredentialStore::get_credential(&profile.credential_id).unwrap_or(None);

    let res = rt.block_on(async {
        tz_ai_debloater_lib::ai::AiClient::discover_models(profile, cred.as_deref(), None).await
    });

    match res {
        Ok(discovery) => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&discovery).unwrap());
            } else {
                println!("Discovered Models for Profile '{}':", profile.id);
                println!("{:<32} {:<36}", "MODEL ID", "DISPLAY NAME");
                println!("{}", "-".repeat(70));
                for m in discovery.models {
                    println!("{:<32} {:<36}", m.id, m.display_name);
                }
            }
            Ok(())
        }
        Err(e) => Err((format!("Model discovery failed: {}", e.message), EXIT_RUNTIME_ERROR)),
    }
}

// -----------------------------------------------------------------------------
// Main Entrypoint
// -----------------------------------------------------------------------------

fn main() -> ExitCode {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    let args = match parse_cli_args(&raw_args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Run 'tz-debloater --help' for usage instructions.");
            return ExitCode::from(EXIT_USAGE_ERROR);
        }
    };

    if args.command.is_empty() || args.command == "help" {
        print_help();
        return ExitCode::from(EXIT_SUCCESS);
    }

    if args.command == "version" {
        print_version();
        return ExitCode::from(EXIT_SUCCESS);
    }

    let result = match args.command.as_str() {
        "doctor" => cmd_doctor(args.json),
        "devices" => cmd_devices(args.json),
        "users" => cmd_users(args.device.as_deref(), args.json),
        "scan" | "packages" => cmd_scan(&args),
        "candidates" => {
            let mut cand_args = args.clone();
            cand_args.view = Some("candidates".into());
            cmd_scan(&cand_args)
        }
        "plan" => cmd_plan(&args),
        "apply" => cmd_apply(&args),
        "history" => cmd_history(&args),
        "restore" => cmd_restore(&args),
        "config" => cmd_config(&args),
        "models" => cmd_models(&args),
        unknown => {
            eprintln!("Unknown command '{unknown}'.");
            eprintln!("Run 'tz-debloater --help' for available commands.");
            return ExitCode::from(EXIT_USAGE_ERROR);
        }
    };

    match result {
        Ok(()) => ExitCode::from(EXIT_SUCCESS),
        Err((msg, code)) => {
            if !args.json {
                eprintln!("Error (code {code}): {msg}");
            } else {
                let err_obj = serde_json::json!({
                    "error": true,
                    "exit_code": code,
                    "message": msg
                });
                eprintln!("{}", serde_json::to_string_pretty(&err_obj).unwrap());
            }
            ExitCode::from(code)
        }
    }
}
