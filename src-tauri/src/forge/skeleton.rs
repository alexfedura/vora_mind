use super::ast_engine::AstEngine;
use super::model::{CanonicalEntityId, LineageId, SymbolId, SymbolKind};
use serde::Serialize;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SymbolDefinition {
    pub name: String,
    pub kind: String,
    pub signature: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    /// All function calls or type references made inside this symbol's body
    pub outbound_references: Vec<String>,
    pub canonical_entity_id: Option<CanonicalEntityId>,
    pub symbol_id: Option<SymbolId>,
    pub lineage_id: Option<LineageId>,
}

pub struct SkeletonExtractor {
    parser: Parser,
    language: Language,
    language_name: &'static str,
}

impl SkeletonExtractor {
    pub fn new_rust() -> Self {
        let language = tree_sitter_rust::language();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("Failed to load Rust grammar");
        Self {
            parser,
            language,
            language_name: "rust",
        }
    }

    pub fn new_typescript() -> Self {
        let language = tree_sitter_typescript::language_typescript();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("Failed to load TypeScript grammar");
        Self {
            parser,
            language,
            language_name: "typescript",
        }
    }

    pub fn new_tsx() -> Self {
        let language = tree_sitter_typescript::language_tsx();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("Failed to load TSX grammar");
        Self {
            parser,
            language,
            language_name: "tsx",
        }
    }

    pub fn for_file(file_path: &str) -> Self {
        if file_path.ends_with(".tsx") {
            Self::new_tsx()
        } else if file_path.ends_with(".ts")
            || file_path.ends_with(".js")
            || file_path.ends_with(".jsx")
        {
            Self::new_typescript()
        } else {
            Self::new_rust()
        }
    }

    fn map_kind_str(kind: &str) -> SymbolKind {
        match kind {
            "struct_item" | "class_declaration" => SymbolKind::Struct,
            "trait_item" | "interface_declaration" => SymbolKind::Trait,
            "enum_item" => SymbolKind::Enum,
            "type_alias_declaration" => SymbolKind::TypeAlias,
            "method_definition" => SymbolKind::Method,
            _ => SymbolKind::Function,
        }
    }

    fn extract_canonical_signature(node: &Node, source: &[u8]) -> String {
        let body_start = node
            .child_by_field_name("body")
            .map(|b| b.start_byte())
            .unwrap_or_else(|| node.end_byte());

        let sig_bytes = if body_start >= node.start_byte() && body_start <= node.end_byte() {
            &source[node.start_byte()..body_start]
        } else {
            &source[node.start_byte()..node.end_byte()]
        };

        let raw_sig = String::from_utf8_lossy(sig_bytes);
        raw_sig
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
            .trim_end_matches('{')
            .trim_end_matches(';')
            .trim()
            .to_string()
    }

    /// Extracts all symbol definitions and their internal calls from source code across Rust and TypeScript
    pub fn analyze_file(&mut self, file_path: &str, source: &[u8]) -> Vec<SymbolDefinition> {
        let tree = match self.parser.parse(source, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let query_str = if self.language_name == "rust" {
            r#"
                (function_item name: (identifier) @name) @item
                (struct_item name: (type_identifier) @name) @item
                (trait_item name: (type_identifier) @name) @item
                (enum_item name: (type_identifier) @name) @item
            "#
        } else {
            r#"
                (function_declaration name: (identifier) @name) @item
                (class_declaration name: (type_identifier) @name) @item
                (interface_declaration name: (type_identifier) @name) @item
                (type_alias_declaration name: (type_identifier) @name) @item
                (method_definition name: (property_identifier) @name) @item
            "#
        };

        let query = match Query::new(&self.language, query_str) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source);

        let mut definitions = Vec::new();
        let module_namespace = AstEngine::extract_module_namespace(file_path);

        for m in matches {
            let mut name = String::new();
            let mut item_node: Option<Node> = None;

            for capture in m.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                if *capture_name == "name" {
                    name = capture.node.utf8_text(source).unwrap_or("").to_string();
                } else if *capture_name == "item" {
                    item_node = Some(capture.node);
                }
            }

            if let Some(node) = item_node {
                let signature = Self::extract_canonical_signature(&node, source);
                let outbound_references = self.extract_calls_in_node(&node, source);
                let kind_str = node.kind().to_string();
                let sym_kind = Self::map_kind_str(&kind_str);

                let symbol_id = Some(SymbolId::compute_canonical(
                    self.language_name,
                    &module_namespace,
                    &name,
                    sym_kind,
                    &signature,
                ));

                definitions.push(SymbolDefinition {
                    name,
                    kind: kind_str,
                    signature,
                    file_path: file_path.to_string(),
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    outbound_references,
                    canonical_entity_id: None,
                    symbol_id,
                    lineage_id: None,
                });
            }
        }

        definitions
    }

    /// Scans a node subtree to find any call expressions or identifier references
    fn extract_calls_in_node(&self, node: &Node, source: &[u8]) -> Vec<String> {
        let call_query_str = if self.language_name == "rust" {
            r#"
                (call_expression function: (identifier) @callee)
                (call_expression function: (field_expression field: (field_identifier) @callee))
                (type_identifier) @type_ref
            "#
        } else {
            r#"
                (call_expression function: (identifier) @callee)
                (call_expression function: (member_expression property: (property_identifier) @callee))
                (type_identifier) @type_ref
            "#
        };

        let query = match Query::new(&self.language, call_query_str) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *node, source);

        let mut references = Vec::new();
        for m in matches {
            for capture in m.captures {
                if let Ok(text) = capture.node.utf8_text(source) {
                    let ref_name = text.to_string();
                    if !references.contains(&ref_name) {
                        references.push(ref_name);
                    }
                }
            }
        }
        references
    }
}
