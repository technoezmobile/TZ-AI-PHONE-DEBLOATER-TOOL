use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let joined = args.join(" ");
    let serial = args
        .windows(2)
        .find(|w| w[0] == "-s")
        .map(|w| w[1].as_str())
        .unwrap_or("none");
    let user = args
        .windows(2)
        .find(|w| w[0] == "--user")
        .map(|w| w[1].as_str())
        .unwrap_or("0");
    let state_dir = std::env::var_os("TZ_DEBLOATER_TEST_DATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("mock-adb-state");
    let _ = std::fs::create_dir_all(&state_dir);
    let state = state_dir.join(format!("{serial}-{user}.disabled"));
    if args.first().map(String::as_str) == Some("stderr_flood") {
        eprintln!("{}", "x".repeat(512 * 1024));
        println!("drained");
        return;
    }
    if joined.contains("resolve-activity") {
        println!("com.android.launcher/.Home");
        return;
    }
    if joined.contains("default_input_method") {
        println!("com.android.inputmethod.latin/.Keyboard");
        return;
    }

    if args.first().map(|s| s.as_str()) == Some("version") {
        println!("Android Debug Bridge version 1.0.41\nVersion 34.0.5-10724011");
        return;
    }

    if joined.contains("devices -l") {
        println!("List of devices attached\nMOCK_SERIAL_123        device product:shiba model:Pixel_8 device:shiba transport_id:1");
        return;
    }

    if joined.contains("devices") {
        println!("List of devices attached\nMOCK_SERIAL_123        device");
        return;
    }

    if joined.contains("getprop ro.product.model") {
        println!("Pixel 8");
        return;
    }

    if joined.contains("getprop ro.product.brand") {
        println!("Google");
        return;
    }

    if joined.contains("getprop ro.build.version.release") {
        println!("14");
        return;
    }

    if joined.contains("getprop ro.build.version.sdk") {
        println!("34");
        return;
    }

    if joined.contains("pm list users") {
        if joined.contains("MALFORMED") {
            println!("Corrupted output without users");
            return;
        }
        println!("Users:\n\tUserInfo{{0:Owner:13}} running\n\tUserInfo{{10:Work:30}} running");
        return;
    }

    if joined.contains("pm list packages") {
        if joined.contains("SECURITY_ERROR") {
            println!("java.lang.SecurityException: Permission Denial: getInstalledPackages for user 10 requires android.permission.INTERACT_ACROSS_USERS");
            return;
        }
        if joined.contains("MANAGER_ERROR") {
            println!("Error: Could not access package manager");
            return;
        }
        if joined.contains("MALFORMED_PACKAGES") {
            println!("random corrupted garbage output line 1\nrandom corrupted line 2");
            return;
        }
        if args.iter().any(|a| a == "-d") {
            if state.exists() {
                println!("package:com.facebook.katana");
            }
        } else if args.iter().any(|a| a == "-e") {
            println!("package:com.android.settings\npackage:com.android.systemui");
            if !state.exists() {
                println!("package:com.facebook.katana");
            }
        } else {
            println!("package:com.facebook.katana\npackage:com.android.settings\npackage:com.android.systemui");
        }
        return;
    }

    if joined.contains("pm path") {
        if joined.contains("com.facebook.katana") {
            println!("package:/data/app/com.facebook.katana/base.apk");
        } else {
            // Not installed
        }
        return;
    }

    if joined.contains("pm uninstall -k") {
        // Mock failure with -k to verify no silent fallback occurs
        println!("Failure [DELETE_FAILED_INTERNAL_ERROR]");
        std::process::exit(1);
    }

    if joined.contains("pm uninstall") && !joined.contains("-k") {
        // If standard uninstall is called without -k, flag this
        println!("UNINSTALL_WITHOUT_K_CALLED");
        return;
    }

    if joined.contains("pm disable-user") {
        let _ = std::fs::write(&state, "disabled");
        println!("Package com.facebook.katana new state: disabled-user");
        return;
    }

    if joined.contains("pm enable") {
        let _ = std::fs::remove_file(&state);
        println!("Package com.facebook.katana new state: enabled");
        return;
    }

    if joined.contains("cmd package install-existing") {
        println!("Package com.facebook.katana installed for user: 0");
        return;
    }

    if joined.contains("am force-stop") {
        if joined.contains("--user") {
            println!("Force stopped with profile target");
        } else {
            eprintln!("ERROR: am force-stop called without --user profile targeting");
            std::process::exit(2);
        }
        return;
    }

    if joined.contains("sleep_long") {
        // Sleep to test timeout and child process termination
        thread::sleep(Duration::from_secs(10));
        println!("Finished sleep");
        return;
    }

    println!("Mock ADB executed: {}", joined);
}
