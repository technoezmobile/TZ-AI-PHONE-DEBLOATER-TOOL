//! Genuine cross-process Windows Credential Manager persistence test.
//! Saves a disposable UUID credential in Process 1, exits completely,
//! reads and verifies it in Process 2, and deletes it in Process 3.
//! No memory-store fallback is substituted; calls direct native OS keyring FFI.

#[cfg(windows)]
use tz_ai_debloater_lib::storage::credentials::os_keyring;
#[cfg(windows)]
use uuid::Uuid;

#[cfg(windows)]
#[test]
fn test_cross_process_windows_vault_persistence() {
    let args: Vec<String> = std::env::args().collect();

    // Check if running as a designated child step
    if let Some(pos) = args.iter().position(|a| a == "--child-step") {
        let step = &args[pos + 1];
        let target = &args[pos + 2];
        match step.as_str() {
            "save" => {
                let secret = &args[pos + 3];
                os_keyring::save(target, secret).expect("Child Process 1 (save) failed");
                std::process::exit(0);
            }
            "read" => {
                let expected = &args[pos + 3];
                let val = os_keyring::read(target).expect("Child Process 2 (read) failed");
                if val.as_deref() != Some(expected.as_str()) {
                    eprintln!("Mismatch: expected {:?}, got {:?}", expected, val);
                    std::process::exit(1);
                }
                std::process::exit(0);
            }
            "delete" => {
                os_keyring::delete(target).expect("Child Process 3 (delete) failed");
                let val = os_keyring::read(target).expect("Child Process 3 verify-read failed");
                if val.is_some() {
                    eprintln!("Error: credential still exists after delete");
                    std::process::exit(1);
                }
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown child step: {}", step);
                std::process::exit(2);
            }
        }
    }

    // Running as test parent:
    let current_exe = std::env::current_exe().expect("Failed to locate current test executable");
    let test_uuid = Uuid::new_v4().to_string();
    let disposable_target = format!("TZ_DEBLOATER_CROSSPROC_{test_uuid}");
    let test_secret = format!("sk-crossproc-disposable-secret-{test_uuid}");

    // Pre-flight cleanup
    let _ = os_keyring::delete(&disposable_target);

    // 1. Process 1: Save in child process and wait for it to exit completely
    let status_save = std::process::Command::new(&current_exe)
        .args([
            "test_cross_process_windows_vault_persistence",
            "--exact",
            "--nocapture",
            "--",
            "--child-step",
            "save",
            &disposable_target,
            &test_secret,
        ])
        .status()
        .expect("Failed to spawn Child Process 1 (save)");
    assert!(
        status_save.success(),
        "Child Process 1 (save) failed with status: {:?}",
        status_save
    );

    // 2. Process 2: Read in child process and wait for it to exit completely
    let status_read = std::process::Command::new(&current_exe)
        .args([
            "test_cross_process_windows_vault_persistence",
            "--exact",
            "--nocapture",
            "--",
            "--child-step",
            "read",
            &disposable_target,
            &test_secret,
        ])
        .status()
        .expect("Failed to spawn Child Process 2 (read)");
    assert!(
        status_read.success(),
        "Child Process 2 (read) failed with status: {:?}",
        status_read
    );

    // 3. Process 3: Delete in child process and verify it is gone
    let status_delete = std::process::Command::new(&current_exe)
        .args([
            "test_cross_process_windows_vault_persistence",
            "--exact",
            "--nocapture",
            "--",
            "--child-step",
            "delete",
            &disposable_target,
        ])
        .status()
        .expect("Failed to spawn Child Process 3 (delete)");
    assert!(
        status_delete.success(),
        "Child Process 3 (delete) failed with status: {:?}",
        status_delete
    );

    // 4. Final verification in parent process: ensure credential is deleted
    let final_check = os_keyring::read(&disposable_target).expect("Parent read check");
    assert_eq!(
        final_check, None,
        "Disposable credential persisted after child delete"
    );
}

