use super::command::AdbExecutor;
use super::operation::{ActionType, ClassifiedCause, OperationOutcome};
use super::package::PackageState;
use serde::{Deserialize, Serialize};

pub const REMOTE_HELPER_PATH: &str = "/data/local/tmp/tz-helper.apk";
pub const HELPER_MAIN_CLASS: &str = "io.github.samolego.canta.helper.Main";
pub const PROTOCOL_VERSION: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperOperationResult {
    pub package_name: String,
    pub success: bool,
    pub status: i32,
    pub message: String,
}

pub struct TzHelperBackend;

impl TzHelperBackend {
    /// Decode a base64-encoded protobuf line emitted by Canta-compatible helper
    pub fn decode_protocol_line(line: &str) -> Option<HelperOperationResult> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let bytes = match base64_decode(trimmed) {
            Some(b) => b,
            None => return None,
        };

        Self::decode_protobuf_operation_result(&bytes)
    }

    /// Decode raw Protobuf bytes for OperationResult:
    /// Field 1: package_name (string)
    /// Field 2: success (bool)
    /// Field 3: status (int32/varint)
    /// Field 4: message (string)
    pub fn decode_protobuf_operation_result(bytes: &[u8]) -> Option<HelperOperationResult> {
        let mut idx = 0;
        let mut package_name = String::new();
        let mut success = false;
        let mut status = -1000;
        let mut message = String::new();

        while idx < bytes.len() {
            let key = decode_varint(bytes, &mut idx)?;
            let wire_type = key & 0x07;
            let field_num = key >> 3;

            match (field_num, wire_type) {
                (1, 2) => {
                    // String: package_name
                    let len = decode_varint(bytes, &mut idx)? as usize;
                    if idx + len > bytes.len() {
                        return None;
                    }
                    package_name = String::from_utf8_lossy(&bytes[idx..idx + len]).into_owned();
                    idx += len;
                }
                (2, 0) => {
                    // Bool: success
                    let val = decode_varint(bytes, &mut idx)?;
                    success = val != 0;
                }
                (3, 0) => {
                    // Int32: status
                    let val = decode_varint(bytes, &mut idx)?;
                    status = val as i32;
                }
                (4, 2) => {
                    // String: message
                    let len = decode_varint(bytes, &mut idx)? as usize;
                    if idx + len > bytes.len() {
                        return None;
                    }
                    message = String::from_utf8_lossy(&bytes[idx..idx + len]).into_owned();
                    idx += len;
                }
                (_, 0) => {
                    // Skip varint
                    let _ = decode_varint(bytes, &mut idx)?;
                }
                (_, 1) => {
                    // Skip 64-bit
                    idx += 8;
                }
                (_, 2) => {
                    // Skip length-delimited
                    let len = decode_varint(bytes, &mut idx)? as usize;
                    idx += len;
                }
                (_, 5) => {
                    // Skip 32-bit
                    idx += 4;
                }
                _ => return None,
            }
        }

        Some(HelperOperationResult {
            package_name,
            success,
            status,
            message,
        })
    }

    /// Check if the helper is present on the device and verify its protocol version
    pub fn is_helper_ready(executor: &AdbExecutor, serial: &str) -> bool {
        let cmd = format!(
            "CLASSPATH={REMOTE_HELPER_PATH} app_process /system/bin {HELPER_MAIN_CLASS} version"
        );
        let res = match executor.run_shell(serial, &["sh", "-c", &cmd]) {
            Ok(r) => r,
            Err(_) => return false,
        };

        if res.success {
            let last_line = res
                .stdout
                .lines()
                .filter(|l| !l.trim().is_empty())
                .last()
                .unwrap_or("")
                .trim();
            if let Ok(ver) = last_line.parse::<i32>() {
                return ver >= 1;
            }
        }
        false
    }

    /// Execute package action via TZ Helper backend if available
    pub fn try_execute_action(
        executor: &AdbExecutor,
        serial: &str,
        user_id: u32,
        package_id: &str,
        action: ActionType,
    ) -> Option<OperationOutcome> {
        if !Self::is_helper_ready(executor, serial) {
            return None;
        }

        let user_str = user_id.to_string();
        let (sub_cmd, is_uninstall) = match action {
            ActionType::UninstallUser => ("uninstall", true),
            ActionType::Restore => ("reinstall", false),
            // Helper does not implement disable/enable; handled by direct pm
            _ => return None,
        };

        let cmd = format!(
            "CLASSPATH={REMOTE_HELPER_PATH} app_process /system/bin {HELPER_MAIN_CLASS} --user {user_str} {sub_cmd} {package_id}"
        );

        let res = executor.run_shell(serial, &["sh", "-c", &cmd]).ok()?;

        // Decode lines from stdout
        for line in res.stdout.lines().rev() {
            if let Some(decoded) = Self::decode_protocol_line(line) {
                if decoded.package_name == package_id {
                    let verified = decoded.success;
                    let resulting_state = if is_uninstall {
                        if verified {
                            Some(PackageState::Uninstalled)
                        } else {
                            Some(PackageState::Enabled)
                        }
                    } else if verified {
                        Some(PackageState::Enabled)
                    } else {
                        Some(PackageState::Uninstalled)
                    };

                    let message = if verified {
                        if is_uninstall {
                            format!("Verified uninstalled for User {user_id} via TZ Helper.")
                        } else {
                            format!("Verified restored for User {user_id} via TZ Helper.")
                        }
                    } else if !decoded.message.is_empty() {
                        decoded.message.clone()
                    } else {
                        "Helper operation failed".into()
                    };

                    return Some(OperationOutcome {
                        package_id: package_id.to_string(),
                        action,
                        success: decoded.success,
                        exit_code: res.exit_code,
                        previous_state: None,
                        resulting_state,
                        message,
                        verified,
                        classified_cause: if decoded.success {
                            ClassifiedCause::None
                        } else {
                            ClassifiedCause::Unclassified {
                                summary: decoded.message,
                            }
                        },
                        execution_backend: "TzHelper".to_string(),
                        command_args: vec![
                            "app_process".into(),
                            HELPER_MAIN_CLASS.into(),
                            "--user".into(),
                            user_str,
                            sub_cmd.into(),
                            package_id.into(),
                        ],
                        target_device: serial.to_string(),
                        target_user_id: user_id,
                        raw_stdout: Some(res.stdout),
                        raw_stderr: Some(res.stderr),
                    });
                }
            }
        }

        None
    }
}

/// Helper function to decode standard base64 without external crate
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const B64_MAP: [i8; 256] = {
        let mut table = [-1i8; 256];
        let mut i = 0usize;
        while i < 26 {
            table[b'A' as usize + i] = i as i8;
            table[b'a' as usize + i] = (i + 26) as i8;
            if i < 10 {
                table[b'0' as usize + i] = (i + 52) as i8;
            }
            i += 1;
        }
        table[b'+' as usize] = 62;
        table[b'/' as usize] = 63;
        table
    };

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in input.as_bytes() {
        if b == b'=' || b.is_ascii_whitespace() {
            continue;
        }
        let val = B64_MAP[b as usize];
        if val < 0 {
            return None;
        }
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }

    Some(out)
}

fn decode_varint(bytes: &[u8], idx: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    let mut shift = 0;
    while *idx < bytes.len() {
        let byte = bytes[*idx];
        *idx += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            return Some(result);
        }
        shift += 7;
        if shift > 64 {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protobuf_decoding_success() {
        // Construct protobuf bytes for:
        // package_name: "com.test.app" (tag 1: 0x0A, len 12)
        // success: true (tag 2: 0x10, val 1)
        // status: 0 (tag 3: 0x18, val 0)
        // message: "Success" (tag 4: 0x22, len 7)
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x0A, 12]);
        bytes.extend_from_slice(b"com.test.app");
        bytes.extend_from_slice(&[0x10, 0x01]);
        bytes.extend_from_slice(&[0x18, 0x00]);
        bytes.extend_from_slice(&[0x22, 7]);
        bytes.extend_from_slice(b"Success");

        let res = TzHelperBackend::decode_protobuf_operation_result(&bytes).expect("Must decode");
        assert_eq!(res.package_name, "com.test.app");
        assert!(res.success);
        assert_eq!(res.status, 0);
        assert_eq!(res.message, "Success");
    }

    #[test]
    fn test_base64_protocol_line_decoding() {
        // Encode the above bytes to base64
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x0A, 12]);
        bytes.extend_from_slice(b"com.test.app");
        bytes.extend_from_slice(&[0x10, 0x01]);
        bytes.extend_from_slice(&[0x18, 0x00]);
        bytes.extend_from_slice(&[0x22, 7]);
        bytes.extend_from_slice(b"Success");

        // Standard base64 representation of these bytes
        let b64 = "Cgxjb20udGVzdC5hcHAQARgAIgdTdWNjZXNz";
        let res = TzHelperBackend::decode_protocol_line(b64).expect("Must decode line");
        assert_eq!(res.package_name, "com.test.app");
        assert!(res.success);
    }
}

