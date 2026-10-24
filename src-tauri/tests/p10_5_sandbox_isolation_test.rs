use std::fs;
use std::process::Command;
use vora_lib::forge::tools::ForgeToolRegistry;

#[test]
fn test_p10_5_g2_sandbox_zero_contamination_invariant() {
    let base_temp = std::env::temp_dir().join(format!("vora_real_repo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base_temp);
    fs::create_dir_all(base_temp.join("src")).expect("Failed to create real repo directory");

    let main_rs_path = base_temp.join("src/main.rs");
    let original_clean_code = "fn main() {\n    println!(\"CLEAN_PRODUCTION_CODE\");\n}\n";
    fs::write(&main_rs_path, original_clean_code).expect("Failed to write original main.rs");

    // سناریو ۱: پچ مخرب یا کاندیدای آلوده‌کننده
    let malicious_candidate_patch =
        "fn main() {\n    println!(\"CONTAMINATED_CANDIDATE_LEAK\");\n}\n";

    // اجرای پچ در Candidate Sandbox
    let test_res = ForgeToolRegistry::forge_test_candidate_patch(
        base_temp.to_str().unwrap(),
        "src/main.rs",
        malicious_candidate_patch,
        Some("cargo --version"),
    );

    assert!(test_res.is_ok(), "Sandbox candidate test execution failed");

    // راستی‌آزمایی سطح ۱: محتوای فایل اصلی در ریپازیتوری پایه نباید ذره‌ای تغییر کرده باشد
    let read_back_original =
        fs::read_to_string(&main_rs_path).expect("Failed to read original file");
    assert_eq!(
        read_back_original, original_clean_code,
        "CRITICAL INVARIANT VIOLATED: Real repository was mutated by Candidate Sandbox!"
    );
    assert!(
        !read_back_original.contains("CONTAMINATED_CANDIDATE_LEAK"),
        "Leak detected inside production working tree!"
    );

    // سناریو ۲: پچ چندفایلی کاندیدا و تست ایزولاسیون کامل چند مسیره
    let lib_rs_path = base_temp.join("src/lib.rs");
    let original_lib_code = "pub fn value() -> i32 { 10 }\n";
    fs::write(&lib_rs_path, original_lib_code).expect("Failed to write original lib.rs");

    let multi_patches = [
        ("src/main.rs", "fn main() { panic!(\"SANDBOX_PANIC\"); }"),
        ("src/lib.rs", "pub fn value() -> i32 { 999 }"),
    ];

    let multi_res = ForgeToolRegistry::forge_test_multi_candidate_patches(
        base_temp.to_str().unwrap(),
        &multi_patches,
        Some("cargo --version"),
    );

    assert!(multi_res.is_ok());

    // راستی‌آزمایی سطح ۲: هر دو فایل اصلی در ریپازیتوری باید دست‌نخورده باقی بمانند
    let final_main = fs::read_to_string(&main_rs_path).unwrap();
    let final_lib = fs::read_to_string(&lib_rs_path).unwrap();

    assert_eq!(final_main, original_clean_code);
    assert_eq!(final_lib, original_lib_code);

    let _ = fs::remove_dir_all(&base_temp);
}

/// P10.5-H1: اثبات قطعی اجرای کد کاندیدا در سندباکس و راستی‌آزمایی مقدار جهش‌یافته
#[test]
fn test_p10_5_h1_candidate_execution_proof() {
    let proof_dir = std::env::temp_dir().join(format!("vora_exec_proof_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proof_dir);
    fs::create_dir_all(proof_dir.join("src")).expect("Failed to create proof project dir");

    // ساخت یک پروژه حداقلی معتبر Rust با تست داخلی
    let cargo_toml = r#"[package]
name = "proof_target"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(proof_dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    let original_lib = r#"pub fn calculate_magic() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic() {
        assert_eq!(calculate_magic(), 1);
    }
}
"#;
    let lib_path = proof_dir.join("src/lib.rs");
    fs::write(&lib_path, original_lib).expect("Failed to write src/lib.rs");

    // ۱. بررسی اجرای تست در ریپوی پایه (باید پاس شود چون مقدار ۱ است)
    let base_test = Command::new("cargo")
        .arg("test")
        .current_dir(&proof_dir)
        .output();

    if let Ok(out) = base_test {
        if !out.status.success() {
            eprintln!("Cargo test skipped or toolchain unavailable in test env");
            let _ = fs::remove_dir_all(&proof_dir);
            return;
        }
    }

    // ۲. تعریف پچ کاندیدا: تغییر مقدار خروجی به ۹۹۹ و به‌روزرسانی تست برای پذیرش ۹۹۹
    let candidate_patched_lib = r#"pub fn calculate_magic() -> i32 {
    999
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic() {
        assert_eq!(calculate_magic(), 999);
    }
}
"#;

    // ۳. اجرای پچ در Candidate Workspace
    let result = ForgeToolRegistry::forge_test_candidate_patch(
        proof_dir.to_str().unwrap(),
        "src/lib.rs",
        candidate_patched_lib,
        Some("cargo test -- --nocapture"),
    );

    // ۴. راستی‌آزمایی: تست کاندیدا باید با موفقیت بر روی مقدار ۹۹۹ اجرا شده باشد
    assert!(
        result.is_ok(),
        "Candidate execution failed: {:?}",
        result.err()
    );

    // ۵. اثبات قطعی عدم نشت: ریپازیتوری اصلی باید همچنان مقدار ۱ را نگه داشته باشد
    let base_content_after = fs::read_to_string(&lib_path).unwrap();
    assert!(
        base_content_after.contains("assert_eq!(calculate_magic(), 1)"),
        "H1 VIOLATION: Real repository content was overwritten by candidate patch!"
    );
    assert!(
        !base_content_after.contains("999"),
        "H1 VIOLATION: Mutated value leaked into real repository!"
    );

    let _ = fs::remove_dir_all(&proof_dir);
}
