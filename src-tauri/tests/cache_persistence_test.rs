use vora_lib::forge::cache::{CachedSymbolEntry, SemanticCacheManager};

#[test]
fn test_cache_lifecycle_and_binary_vector_persistence() {
    let temp_dir = std::env::temp_dir().join(format!("vora_cache_test_{}", uuid::Uuid::now_v7()));

    let file_path = "src/neural_engine.rs";
    let code_content = "pub fn execute_tensor() -> bool { true }";
    let vector_data = vec![0.123f32, -0.456f32, 0.789f32, 1.0f32];

    // ۱. مقداردهی اولیه و ذخیره وضعیت در کش
    {
        let mut manager =
            SemanticCacheManager::new(&temp_dir).expect("Failed to create cache manager");

        let symbols = vec![CachedSymbolEntry {
            symbol_id: "sym_tensor_01".to_string(),
            name: "execute_tensor".to_string(),
            kind: "function".to_string(),
            signature: "pub fn execute_tensor() -> bool".to_string(),
            start_line: 1,
            end_line: 1,
        }];

        manager.update_file_cache(file_path, code_content, 1700000000, symbols);
        manager
            .append_vector(file_path, &vector_data)
            .expect("Failed to write vector");
        manager.flush_to_disk().expect("Failed to flush manifest");
    }

    // ۲. شبیه‌سازی Cold Start (بازگشایی مجدد و خواندن از دیسک بدون پردازش مجدد)
    {
        let manager = SemanticCacheManager::new(&temp_dir).expect("Failed to reopen cache manager");

        // بررسی همگام بودن محتوا
        assert!(manager.is_file_up_to_date(file_path, 1700000000, code_content));

        // بررسی بازخوانی نمادها
        let cached_symbols = manager
            .get_cached_symbols(file_path)
            .expect("Symbols must exist");
        assert_eq!(cached_symbols.len(), 1);
        assert_eq!(cached_symbols[0].name, "execute_tensor");

        // بررسی بازیابی دقیق باینری بردارها
        let loaded_vector = manager
            .read_vector(file_path)
            .expect("Vector read failed")
            .expect("Vector must exist");
        assert_eq!(loaded_vector, vector_data);
    }

    std::fs::remove_dir_all(temp_dir).ok();
}
