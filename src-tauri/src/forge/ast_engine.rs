use super::model::{
    ContentHash, FileId, ImportKind, ReferenceKind, ScopeContext, ScopeId, SymbolId,
    SymbolIdentity, SymbolKind, VisibilityKind,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub scope_path: String,
    pub signature: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawImport {
    pub source_module: String,
    pub imported_name: String,
    pub alias: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawReference {
    pub caller_scope: String,
    pub callee_name: String,
    pub line: usize,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstRelation {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstGraphPayload {
    pub file_path: String,
    pub symbols: Vec<SymbolNode>,
    pub relations: Vec<AstRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawParsedFile {
    pub file_path: String,
    pub symbols: Vec<SymbolNode>,
    pub imports: Vec<RawImport>,
    pub references: Vec<RawReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFilePayload {
    pub file_id: FileId,
    pub path: String,
    pub symbols: Vec<SymbolIdentity>,
    pub scopes: Vec<ScopeContext>,
    pub raw_references: Vec<super::model::RawReference>,
}

pub struct AstEngine {
    parser: Parser,
    language_name: &'static str,
}

impl AstEngine {
    pub fn new_rust() -> Self {
        let language = tree_sitter_rust::language();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("Failed to load Rust grammar");
        Self {
            parser,
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

    pub fn parse_file_tree(&mut self, content: &str) -> Option<tree_sitter::Tree> {
        self.parser.parse(content.as_bytes(), None)
    }

    pub fn extract_symbols(&mut self, source: &[u8]) -> Vec<SymbolNode> {
        self.extract_symbols_with_path("unknown", source)
    }

    pub fn extract_symbols_with_path(&mut self, file_path: &str, source: &[u8]) -> Vec<SymbolNode> {
        let content = String::from_utf8_lossy(source);
        let parsed = Self::parse_file_syntax(file_path, &content);
        parsed.symbols
    }

    pub fn extract_module_namespace(file_path: &str) -> String {
        let normalized = file_path.replace('\\', "/");
        let path = Path::new(&normalized);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");

        let parent_components: Vec<String> = path
            .parent()
            .map(|p| {
                p.iter()
                    .filter_map(|seg| seg.to_str())
                    .filter(|seg| *seg != "." && *seg != "src")
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        if parent_components.is_empty() {
            format!("crate::{}", stem)
        } else {
            format!("crate::{}::{}", parent_components.join("::"), stem)
        }
    }

    pub fn compute_canonical_symbol_id(
        language: &str,
        module_namespace: &str,
        semantic_scope: &str,
        kind: SymbolKind,
        canonical_signature: &str,
    ) -> SymbolId {
        SymbolId::compute_canonical(
            language,
            module_namespace,
            semantic_scope,
            kind,
            canonical_signature,
        )
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

    pub fn parse_file_semantics(
        file_id: FileId,
        file_path: &str,
        content: &str,
    ) -> ParsedFilePayload {
        let mut engine = Self::for_file(file_path);
        let source = content.as_bytes();

        let tree = match engine.parser.parse(source, None) {
            Some(t) => t,
            None => {
                return ParsedFilePayload {
                    file_id,
                    path: file_path.to_string(),
                    symbols: Vec::new(),
                    scopes: Vec::new(),
                    raw_references: Vec::new(),
                }
            }
        };

        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        let mut scopes = Vec::new();
        let mut raw_references = Vec::new();

        let module_namespace = Self::extract_module_namespace(file_path);
        let root_scope_id = ScopeId::compute_scoped(&file_id, &module_namespace, 0, "root");
        let root_scope = ScopeContext {
            scope_id: root_scope_id.clone(),
            file_id: file_id.clone(),
            parent_scope_id: None,
            visible_bindings: HashMap::new(),
        };
        scopes.push(root_scope);

        engine.traverse_semantic_node(
            root_node,
            &file_id,
            &module_namespace,
            source,
            &root_scope_id,
            "crate",
            1,
            None,
            &mut symbols,
            &mut scopes,
            &mut raw_references,
        );

        ParsedFilePayload {
            file_id,
            path: file_path.to_string(),
            symbols,
            scopes,
            raw_references,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn traverse_semantic_node(
        &self,
        node: Node,
        file_id: &FileId,
        module_namespace: &str,
        source: &[u8],
        current_scope_id: &ScopeId,
        current_scope_path: &str,
        current_depth: usize,
        current_caller_symbol: Option<&SymbolId>,
        symbols: &mut Vec<SymbolIdentity>,
        scopes: &mut Vec<ScopeContext>,
        references: &mut Vec<super::model::RawReference>,
    ) {
        let kind = node.kind();
        let mut next_scope_id = current_scope_id.clone();
        let mut next_scope_path = current_scope_path.to_string();
        let mut next_depth = current_depth;
        let mut next_caller_symbol = current_caller_symbol.cloned();

        match kind {
            "use_declaration" => {
                if let Ok(import_text) = node.utf8_text(source) {
                    let cleaned = import_text
                        .trim_start_matches("use ")
                        .trim_end_matches(';')
                        .trim();

                    let (path_part, alias_part) = if cleaned.contains(" as ") {
                        let parts: Vec<&str> = cleaned.split(" as ").collect();
                        (parts[0].trim(), Some(parts[1].trim()))
                    } else {
                        (cleaned, None)
                    };

                    let path_segments: Vec<&str> = path_part.split("::").collect();
                    let original_name = path_segments.last().unwrap_or(&path_part).to_string();
                    let binding_name = alias_part.unwrap_or(&original_name).to_string();

                    if let Some(current_scope) =
                        scopes.iter_mut().find(|s| &s.scope_id == current_scope_id)
                    {
                        current_scope.visible_bindings.insert(
                            binding_name.clone(),
                            if let Some(alias) = alias_part {
                                ImportKind::Alias {
                                    source_module: path_part.to_string(),
                                    alias: alias.to_string(),
                                }
                            } else {
                                ImportKind::Imported {
                                    source_module: path_part.to_string(),
                                    original_name,
                                }
                            },
                        );
                    }
                }
            }

            "function_item"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "impl_item"
            | "function_declaration"
            | "method_definition"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration" => {
                let name = if let Some(name_node) = node.child_by_field_name("name") {
                    name_node
                        .utf8_text(source)
                        .unwrap_or("anonymous")
                        .to_string()
                } else if kind == "impl_item" {
                    if let Some(type_node) = node.child_by_field_name("type") {
                        format!("impl_{}", type_node.utf8_text(source).unwrap_or("unknown"))
                    } else {
                        "anonymous_impl".to_string()
                    }
                } else {
                    "anonymous".to_string()
                };

                let canonical_signature = Self::extract_canonical_signature(&node, source);

                let body_node = node.child_by_field_name("body");
                let body_text = body_node
                    .and_then(|b| b.utf8_text(source).ok())
                    .unwrap_or("");
                let body_hash = Self::compute_hash(body_text);

                let symbol_kind = match kind {
                    "function_item" | "function_declaration" => SymbolKind::Function,
                    "struct_item" | "class_declaration" => SymbolKind::Struct,
                    "enum_item" => SymbolKind::Enum,
                    "trait_item" | "interface_declaration" => SymbolKind::Trait,
                    "type_alias_declaration" => SymbolKind::TypeAlias,
                    _ => SymbolKind::Method,
                };

                let qualified_scope =
                    if current_scope_path.is_empty() || current_scope_path == "crate" {
                        name.clone()
                    } else {
                        format!("{}::{}", current_scope_path, name)
                    };

                let symbol_id = Self::compute_canonical_symbol_id(
                    self.language_name,
                    module_namespace,
                    &qualified_scope,
                    symbol_kind,
                    &canonical_signature,
                );

                symbols.push(SymbolIdentity {
                    id: symbol_id.clone(),
                    file_id: file_id.clone(),
                    module_namespace: module_namespace.to_string(),
                    name: name.clone(),
                    qualified_scope: qualified_scope.clone(),
                    kind: symbol_kind,
                    canonical_signature,
                    body_hash,
                    visibility: VisibilityKind::Public,
                    lineage_id: None,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });

                let child_scope_id =
                    ScopeId::compute_scoped(file_id, module_namespace, current_depth, &name);
                scopes.push(ScopeContext {
                    scope_id: child_scope_id.clone(),
                    file_id: file_id.clone(),
                    parent_scope_id: Some(current_scope_id.clone()),
                    visible_bindings: HashMap::new(),
                });

                next_scope_id = child_scope_id;
                next_scope_path = qualified_scope;
                next_depth = current_depth + 1;
                next_caller_symbol = Some(symbol_id);
            }

            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declarator" {
                        if let (Some(name_node), Some(value_node)) = (
                            child.child_by_field_name("name"),
                            child.child_by_field_name("value"),
                        ) {
                            if value_node.kind() == "arrow_function"
                                || value_node.kind() == "function"
                            {
                                let name = name_node
                                    .utf8_text(source)
                                    .unwrap_or("anonymous")
                                    .to_string();
                                let canonical_signature =
                                    Self::extract_canonical_signature(&node, source);

                                let body_text = value_node
                                    .child_by_field_name("body")
                                    .and_then(|b| b.utf8_text(source).ok())
                                    .unwrap_or("");
                                let body_hash = Self::compute_hash(body_text);

                                let qualified_scope = if current_scope_path.is_empty()
                                    || current_scope_path == "crate"
                                {
                                    name.clone()
                                } else {
                                    format!("{}::{}", current_scope_path, name)
                                };

                                let symbol_id = Self::compute_canonical_symbol_id(
                                    self.language_name,
                                    module_namespace,
                                    &qualified_scope,
                                    SymbolKind::Function,
                                    &canonical_signature,
                                );

                                symbols.push(SymbolIdentity {
                                    id: symbol_id.clone(),
                                    file_id: file_id.clone(),
                                    module_namespace: module_namespace.to_string(),
                                    name: name.clone(),
                                    qualified_scope: qualified_scope.clone(),
                                    kind: SymbolKind::Function,
                                    canonical_signature,
                                    body_hash,
                                    visibility: VisibilityKind::Public,
                                    lineage_id: None,
                                    start_line: node.start_position().row + 1,
                                    end_line: node.end_position().row + 1,
                                    start_byte: node.start_byte(),
                                    end_byte: node.end_byte(),
                                });
                            }
                        }
                    }
                }
            }

            "call_expression" => {
                let mut callee_raw_name = String::new();
                let mut receiver_name = None;
                let mut is_member_access = false;
                let mut access_chain = Vec::new();

                if let Some(function_node) = node.child_by_field_name("function") {
                    let fn_text = function_node.utf8_text(source).unwrap_or("");
                    if fn_text.contains('.') {
                        is_member_access = true;
                        let parts: Vec<&str> = fn_text.split('.').collect();
                        if let Some(first) = parts.first() {
                            receiver_name = Some(first.to_string());
                        }
                        if let Some(last) = parts.last() {
                            callee_raw_name = last.to_string();
                        }
                        access_chain = parts.iter().map(|s| s.to_string()).collect();
                    } else if fn_text.contains("::") {
                        let parts: Vec<&str> = fn_text.split("::").collect();
                        if let Some(last) = parts.last() {
                            callee_raw_name = last.to_string();
                        }
                        access_chain = parts.iter().map(|s| s.to_string()).collect();
                    } else {
                        callee_raw_name = fn_text.to_string();
                        access_chain = vec![fn_text.to_string()];
                    }
                } else if let Some(first_child) = node.child(0) {
                    let txt = first_child.utf8_text(source).unwrap_or("");
                    callee_raw_name = txt.to_string();
                    access_chain = vec![txt.to_string()];
                }

                if !callee_raw_name.is_empty() {
                    references.push(super::model::RawReference {
                        file_id: file_id.clone(),
                        scope_id: current_scope_id.clone(),
                        caller_symbol_id: current_caller_symbol.cloned(),
                        callee_raw_name,
                        receiver_name,
                        is_member_access,
                        access_chain,
                        ref_kind: ReferenceKind::Call,
                        byte_range: (node.start_byte(), node.end_byte()),
                        line: node.start_position().row + 1,
                    });
                }
            }

            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_semantic_node(
                child,
                file_id,
                module_namespace,
                source,
                &next_scope_id,
                &next_scope_path,
                next_depth,
                next_caller_symbol.as_ref(),
                symbols,
                scopes,
                references,
            );
        }
    }

    fn compute_hash(text: &str) -> String {
        ContentHash::from_str(text).0
    }

    pub fn parse_file_syntax(file_path: &str, content: &str) -> RawParsedFile {
        let mut engine = Self::for_file(file_path);
        let source = content.as_bytes();

        let tree = match engine.parser.parse(source, None) {
            Some(t) => t,
            None => {
                return RawParsedFile {
                    file_path: file_path.to_string(),
                    symbols: Vec::new(),
                    imports: Vec::new(),
                    references: Vec::new(),
                }
            }
        };

        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        let mut references = Vec::new();

        engine.traverse_ast_node(
            root_node,
            file_path,
            source,
            "",
            &mut symbols,
            &mut imports,
            &mut references,
        );

        RawParsedFile {
            file_path: file_path.to_string(),
            symbols,
            imports,
            references,
        }
    }

    pub fn parse_source_code(file_path: &str, content: &str) -> AstGraphPayload {
        let parsed = Self::parse_file_syntax(file_path, content);
        let mut relations = Vec::new();

        for ref_item in &parsed.references {
            if let Some(target_sym) = parsed
                .symbols
                .iter()
                .find(|s| s.name == ref_item.callee_name)
            {
                let source_id = if ref_item.caller_scope.is_empty() {
                    format!("{}:root", file_path)
                } else {
                    format!("{}:{}", file_path, ref_item.caller_scope)
                };

                relations.push(AstRelation {
                    source_id,
                    target_id: target_sym.id.clone(),
                    relation_type: if target_sym.kind.contains("struct")
                        || target_sym.kind.contains("interface")
                        || target_sym.kind.contains("type")
                    {
                        "references_type".to_string()
                    } else {
                        "calls_function".to_string()
                    },
                });
            }
        }

        AstGraphPayload {
            file_path: file_path.to_string(),
            symbols: parsed.symbols,
            relations,
        }
    }

    pub fn scan_workspace_directory(root_dir: &str) -> AstGraphPayload {
        let mut all_symbols = Vec::new();
        let mut all_relations = Vec::new();
        let mut files_to_scan = Vec::new();

        Self::collect_source_files(Path::new(root_dir), &mut files_to_scan);

        let mut parsed_files = Vec::new();
        for file_path in &files_to_scan {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                let path_str = file_path.to_string_lossy().to_string();
                let parsed = Self::parse_file_syntax(&path_str, &content);
                parsed_files.push(parsed);
            }
        }

        for pf in &parsed_files {
            all_symbols.extend(pf.symbols.clone());
        }

        for pf in &parsed_files {
            for ref_item in &pf.references {
                if let Some(target_sym) =
                    all_symbols.iter().find(|s| s.name == ref_item.callee_name)
                {
                    let source_id = format!("{}:{}", pf.file_path, ref_item.caller_scope);
                    if source_id != target_sym.id {
                        all_relations.push(AstRelation {
                            source_id,
                            target_id: target_sym.id.clone(),
                            relation_type: if target_sym.kind.contains("struct")
                                || target_sym.kind.contains("interface")
                                || target_sym.kind.contains("type")
                            {
                                "references_type".to_string()
                            } else {
                                "calls_function".to_string()
                            },
                        });
                    }
                }
            }
        }

        AstGraphPayload {
            file_path: root_dir.to_string(),
            symbols: all_symbols,
            relations: all_relations,
        }
    }

    fn collect_source_files(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                    if dir_name != "target"
                        && dir_name != "node_modules"
                        && dir_name != "dist"
                        && !dir_name.starts_with('.')
                    {
                        Self::collect_source_files(&path, files);
                    }
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" || ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
                        files.push(path);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn traverse_ast_node(
        &self,
        node: Node,
        file_path: &str,
        source: &[u8],
        current_scope: &str,
        symbols: &mut Vec<SymbolNode>,
        imports: &mut Vec<RawImport>,
        references: &mut Vec<RawReference>,
    ) {
        let kind = node.kind();
        let mut next_scope = current_scope.to_string();

        match kind {
            "use_declaration" => {
                if let Ok(import_text) = node.utf8_text(source) {
                    let cleaned = import_text
                        .trim_start_matches("use ")
                        .trim_end_matches(';')
                        .trim();
                    let (path_part, alias_part) = if cleaned.contains(" as ") {
                        let parts: Vec<&str> = cleaned.split(" as ").collect();
                        (parts[0].trim(), Some(parts[1].trim().to_string()))
                    } else {
                        (cleaned, None)
                    };

                    let parts: Vec<&str> = path_part.split("::").collect();
                    let imported_name = parts.last().unwrap_or(&path_part).to_string();
                    imports.push(RawImport {
                        source_module: path_part.to_string(),
                        imported_name,
                        alias: alias_part,
                        line: node.start_position().row + 1,
                    });
                }
            }

            "import_statement" => {
                if let Ok(import_text) = node.utf8_text(source) {
                    imports.push(RawImport {
                        source_module: import_text.trim().to_string(),
                        imported_name: "module_import".to_string(),
                        alias: None,
                        line: node.start_position().row + 1,
                    });
                }
            }

            "function_item" | "struct_item" | "enum_item" | "trait_item" | "impl_item" => {
                let name = if let Some(name_node) = node.child_by_field_name("name") {
                    name_node
                        .utf8_text(source)
                        .unwrap_or("anonymous")
                        .to_string()
                } else if kind == "impl_item" {
                    if let Some(type_node) = node.child_by_field_name("type") {
                        format!("impl_{}", type_node.utf8_text(source).unwrap_or("unknown"))
                    } else {
                        "anonymous_impl".to_string()
                    }
                } else {
                    "anonymous".to_string()
                };

                let signature = Self::extract_canonical_signature(&node, source);

                let scope_qualified = if current_scope.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", current_scope, name)
                };

                let symbol_kind = match kind {
                    "function_item" => SymbolKind::Function,
                    "struct_item" => SymbolKind::Struct,
                    "enum_item" => SymbolKind::Enum,
                    "trait_item" => SymbolKind::Trait,
                    _ => SymbolKind::Method,
                };

                let module_namespace = Self::extract_module_namespace(file_path);
                let stable_id = Self::compute_canonical_symbol_id(
                    self.language_name,
                    &module_namespace,
                    &scope_qualified,
                    symbol_kind,
                    &signature,
                )
                .0;

                symbols.push(SymbolNode {
                    id: stable_id,
                    name: name.clone(),
                    kind: kind.to_string(),
                    file_path: file_path.to_string(),
                    scope_path: scope_qualified.clone(),
                    signature,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });

                next_scope = scope_qualified;
            }

            "function_declaration"
            | "method_definition"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source)
                        .unwrap_or("anonymous")
                        .to_string();
                    let signature = Self::extract_canonical_signature(&node, source);

                    let scope_qualified = if current_scope.is_empty() {
                        name.clone()
                    } else {
                        format!("{}.{}", current_scope, name)
                    };

                    let symbol_kind = match kind {
                        "function_declaration" => SymbolKind::Function,
                        "method_definition" => SymbolKind::Method,
                        "class_declaration" => SymbolKind::Struct,
                        "interface_declaration" => SymbolKind::Trait,
                        "type_alias_declaration" => SymbolKind::TypeAlias,
                        _ => SymbolKind::Function,
                    };

                    let module_namespace = Self::extract_module_namespace(file_path);
                    let stable_id = Self::compute_canonical_symbol_id(
                        self.language_name,
                        &module_namespace,
                        &scope_qualified,
                        symbol_kind,
                        &signature,
                    )
                    .0;

                    symbols.push(SymbolNode {
                        id: stable_id,
                        name: name.clone(),
                        kind: kind.to_string(),
                        file_path: file_path.to_string(),
                        scope_path: scope_qualified.clone(),
                        signature,
                        start_line: node.start_position().row + 1,
                        end_line: node.end_position().row + 1,
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                    });

                    next_scope = scope_qualified;
                }
            }

            "call_expression" => {
                let callee_name = if let Some(function_node) = node.child_by_field_name("function")
                {
                    function_node.utf8_text(source).unwrap_or("")
                } else if let Some(first_child) = node.child(0) {
                    first_child.utf8_text(source).unwrap_or("")
                } else {
                    ""
                };

                if !callee_name.is_empty() {
                    let clean_callee = callee_name
                        .rsplit("::")
                        .next()
                        .unwrap_or(callee_name)
                        .rsplit('.')
                        .next()
                        .unwrap_or(callee_name);
                    references.push(RawReference {
                        caller_scope: current_scope.to_string(),
                        callee_name: clean_callee.to_string(),
                        line: node.start_position().row + 1,
                        byte_offset: node.start_byte(),
                    });
                }
            }

            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_ast_node(
                child,
                file_path,
                source,
                &next_scope,
                symbols,
                imports,
                references,
            );
        }
    }
}
