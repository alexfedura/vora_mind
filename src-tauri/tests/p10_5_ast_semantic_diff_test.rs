use vora_lib::forge::differ::SemanticDiffer;
use vora_lib::forge::model::SemanticChange;

#[test]
fn test_p10_5_h2_ast_semantic_diff_signature_mutation() {
    let file_path = "src/math/calc.rs";
    let old_code = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
    let new_code = "pub fn add(a: i32, b: i32, c: i32) -> i32 {\n    a + b + c\n}\n";

    let diff = SemanticDiffer::extract_ast_semantic_change(file_path, "add", old_code, new_code);

    match diff {
        SemanticChange::SignatureChanged {
            old_signature,
            new_signature,
            ..
        } => {
            assert!(old_signature.contains("a: i32, b: i32"));
            assert!(new_signature.contains("a: i32, b: i32, c: i32"));
        }
        other => panic!("Expected SignatureChanged, got: {:?}", other),
    }
}

#[test]
fn test_p10_5_h2_ast_semantic_diff_body_mutation() {
    let file_path = "src/math/calc.rs";
    let old_code = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
    let new_code = "pub fn add(a: i32, b: i32) -> i32 {\n    // modernized internal logic\n    a.saturating_add(b)\n}\n";

    let diff = SemanticDiffer::extract_ast_semantic_change(file_path, "add", old_code, new_code);

    match diff {
        SemanticChange::BodyChanged {
            old_body_hash,
            new_body_hash,
            ..
        } => {
            assert_ne!(old_body_hash, new_body_hash);
        }
        other => panic!("Expected BodyChanged, got: {:?}", other),
    }
}

#[test]
fn test_p10_5_h2_ast_semantic_diff_symbol_renamed() {
    let file_path = "src/math/calc.rs";
    let old_code = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
    let new_code = "pub fn add_numbers(a: i32, b: i32) -> i32 {\n    a + b\n}\n";

    let diff = SemanticDiffer::extract_ast_semantic_change(file_path, "add", old_code, new_code);

    match diff {
        SemanticChange::SymbolRenamed {
            old_name, new_name, ..
        } => {
            assert_eq!(old_name, "add");
            assert_eq!(new_name, "add_numbers");
        }
        other => panic!("Expected SymbolRenamed, got: {:?}", other),
    }
}
