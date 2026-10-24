use std::fs;
use std::process::Command;
use vora_lib::forge::tools::ForgeToolRegistry;

#[test]
fn test_p10_5_h1_candidate_actual_execution_proof() {
    let base_temp = std::env::temp_dir().join(format!("vora_h1_proof_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base_temp);
    fs::create_dir_all(base_temp.join("src")).expect("Failed to create temp project");

    // ۱. ایجاد ساختار واقعی یک پکیج Rust با Cargo.toml و تست واحد
    let cargo_toml = r#"
[package]
name = "h1_proof_pkg"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#;
    fs::write(base_temp.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    let original_lib_code = r#"
pub fn get_value() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_value() {
        assert_eq!(get_value(), 1);
    }
}
"#;
    fs::write(base_temp.join("src/lib.rs"), original_lib_code).expect("Failed to write src/lib.rs");

    // ۲. تایید اولیه: اجرای تست در ریپازیتوری اصلی باید مقدار ۱ را پاس کند
    let base_cargo_check = Command::new("cargo")
        .args(["test", "--", "test_value"])
        .current_dir(&base_temp)
        .output();
    assert!(
        base_cargo_check.is_ok() && base_cargo_check.unwrap().status.success(),
        "Original repo test suite must pass for value = 1"
    );

    // ۳. آماده‌سازی پچ کاندیدا که هم مقدار تابع و هم شرط تست را به ۹۹۹ تغییر می‌دهد
    let candidate_patched_code = r#"
pub fn get_value() -> i32 {
    999
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_value() {
        assert_eq!(get_value(), 999);
    }
}
"#;

    // ۴. اجرای پچ کاندیدا در Candidate Sandbox با اجرای تست واقعی cargo test
    let sandbox_result = ForgeToolRegistry::forge_test_candidate_patch(
        base_temp.to_str().unwrap(),
        "src/lib.rs",
        candidate_patched_code,
        Some("cargo test -- --nocapture"),
    );

    // ۵. اعتبارسنجی سطح ۱: تست سندباکس باید با موفقیت کامپایل و تست ۹۹۹ را تایید کرده باشد
    assert!(
        sandbox_result.is_ok(),
        "Sandbox candidate execution failed to run"
    );
    let test_output = sandbox_result.unwrap();
    assert!(
        test_output.success,
        "Candidate test failed in sandbox! Stdout: {}\nStderr: {}",
        test_output.stdout, test_output.stderr
    );
    assert!(
        test_output.stdout.contains("test tests::test_value ... ok"),
        "Candidate test runner did not execute the modified test suite!"
    );

    // ۶. اعتبارسنجی سطح ۲ (Zero Contamination): ریپازیتوری اصلی باید بدون تغییر مانده و تست آن همچنان روی ۱ پاس شود
    let original_read_back = fs::read_to_string(base_temp.join("src/lib.rs")).unwrap();
    assert_eq!(
        original_read_back, original_lib_code,
        "Contamination detected! Original code was mutated."
    );

    let base_cargo_recheck = Command::new("cargo")
        .args(["test", "--", "test_value"])
        .current_dir(&base_temp)
        .output()
        .expect("Failed to re-run test on base repository");

    assert!(
        base_cargo_recheck.status.success(),
        "Base repository test failed after candidate sandbox run!"
    );

    // پاکسازی دایرکتوری موقت
    let _ = fs::remove_dir_all(&base_temp);
}
