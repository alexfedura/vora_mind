use super::model::{
    CanonicalKind, CanonicalNamespace, CanonicalParameter, CanonicalSignature, CanonicalSymbol,
    FileId, LineageId, SnapshotId, SymbolId, VisibilityKind, WorkspaceId,
};
use tree_sitter::Node;

pub trait SemanticAdapter {
    fn language_name(&self) -> &'static str;
    fn canonicalize_namespace(&self, file_path: &str) -> CanonicalNamespace;
    fn canonicalize_kind(&self, syntax_kind: &str) -> CanonicalKind;
    fn parse_structured_signature(&self, node: &Node, source: &[u8]) -> CanonicalSignature;
    fn normalize_type_representation(&self, raw_type: &str) -> String;
}

pub struct RustSemanticAdapter;

impl SemanticAdapter for RustSemanticAdapter {
    fn language_name(&self) -> &'static str {
        "rust"
    }

    fn canonicalize_namespace(&self, file_path: &str) -> CanonicalNamespace {
        let normalized = file_path.replace('\\', "/");
        let path = std::path::Path::new(&normalized);
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
            CanonicalNamespace::from_rust(stem, "")
        } else {
            CanonicalNamespace::from_rust(&parent_components[0], &parent_components[1..].join("/"))
        }
    }

    fn canonicalize_kind(&self, syntax_kind: &str) -> CanonicalKind {
        match syntax_kind {
            "function_item" => CanonicalKind::Function,
            "struct_item" => CanonicalKind::Struct,
            "trait_item" => CanonicalKind::Trait,
            "enum_item" => CanonicalKind::Enum,
            "enum_variant" => CanonicalKind::EnumVariant,
            "type_item" => CanonicalKind::TypeAlias,
            "field_declaration" => CanonicalKind::Field,
            "const_item" => CanonicalKind::Constant,
            "static_item" => CanonicalKind::Variable,
            "mod_item" => CanonicalKind::Module,
            "impl_item" => CanonicalKind::Impl,
            _ => CanonicalKind::Function,
        }
    }

    fn normalize_type_representation(&self, raw_type: &str) -> String {
        let trimmed = raw_type.trim();
        match trimmed {
            "f64" | "f32" | "i64" | "i32" | "u64" | "u32" | "usize" | "isize" => {
                "number".to_string()
            }
            "bool" => "boolean".to_string(),
            "String" | "&str" => "string".to_string(),
            t if t.starts_with("Vec<") && t.ends_with('>') => {
                let inner = &t[4..t.len() - 1];
                format!("Array<{}>", self.normalize_type_representation(inner))
            }
            other => other.to_string(),
        }
    }

    fn parse_structured_signature(&self, node: &Node, source: &[u8]) -> CanonicalSignature {
        let mut parameters = Vec::new();
        let mut return_type = None;
        let mut generic_parameters = Vec::new();
        let mut is_async = false;

        let raw_text = node.utf8_text(source).unwrap_or("");
        if raw_text.starts_with("async ") || raw_text.contains(" async fn ") {
            is_async = true;
        }

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter" {
                    let param_name = child
                        .child_by_field_name("pattern")
                        .and_then(|n| n.utf8_text(source).ok())
                        .unwrap_or("arg")
                        .to_string();
                    let raw_type = child
                        .child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source).ok())
                        .unwrap_or("unknown");

                    parameters.push(CanonicalParameter {
                        name: param_name,
                        param_type: self.normalize_type_representation(raw_type),
                        is_optional: false,
                    });
                }
            }
        }

        if let Some(ret_node) = node.child_by_field_name("return_type") {
            if let Ok(ret_str) = ret_node.utf8_text(source) {
                let clean = ret_str.trim_start_matches("->").trim();
                return_type = Some(self.normalize_type_representation(clean));
            }
        }

        if let Some(generics_node) = node.child_by_field_name("type_parameters") {
            let mut cursor = generics_node.walk();
            for child in generics_node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    if let Ok(gen_str) = child.utf8_text(source) {
                        generic_parameters.push(gen_str.to_string());
                    }
                }
            }
        }

        let body_start = node
            .child_by_field_name("body")
            .map(|b| b.start_byte())
            .unwrap_or_else(|| node.end_byte());

        let sig_bytes = if body_start >= node.start_byte() && body_start <= node.end_byte() {
            &source[node.start_byte()..body_start]
        } else {
            &source[node.start_byte()..node.end_byte()]
        };

        let raw_signature = String::from_utf8_lossy(sig_bytes).trim().to_string();
        let normalized_text = raw_signature
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        CanonicalSignature {
            raw_signature,
            normalized_text,
            parameters,
            return_type,
            generic_parameters,
            is_async,
        }
    }
}

pub struct TypeScriptSemanticAdapter;

impl SemanticAdapter for TypeScriptSemanticAdapter {
    fn language_name(&self) -> &'static str {
        "typescript"
    }

    fn canonicalize_namespace(&self, file_path: &str) -> CanonicalNamespace {
        let normalized = file_path.replace('\\', "/");
        CanonicalNamespace::from_typescript("app", &normalized)
    }

    fn canonicalize_kind(&self, syntax_kind: &str) -> CanonicalKind {
        match syntax_kind {
            "function_declaration" | "arrow_function" => CanonicalKind::Function,
            "method_definition" => CanonicalKind::Method,
            "class_declaration" => CanonicalKind::Class,
            "interface_declaration" => CanonicalKind::Interface,
            "type_alias_declaration" => CanonicalKind::TypeAlias,
            "enum_declaration" => CanonicalKind::Enum,
            "enum_assignment" => CanonicalKind::EnumVariant,
            "property_signature" | "public_field_definition" => CanonicalKind::Field,
            _ => CanonicalKind::Variable,
        }
    }

    fn normalize_type_representation(&self, raw_type: &str) -> String {
        let trimmed = raw_type.trim();
        match trimmed {
            "number" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "string" => "string".to_string(),
            t if t.ends_with("[]") => {
                let inner = &t[..t.len() - 2];
                format!("Array<{}>", self.normalize_type_representation(inner))
            }
            t if t.starts_with("Array<") && t.ends_with('>') => {
                let inner = &t[6..t.len() - 1];
                format!("Array<{}>", self.normalize_type_representation(inner))
            }
            other => other.to_string(),
        }
    }

    fn parse_structured_signature(&self, node: &Node, source: &[u8]) -> CanonicalSignature {
        let mut parameters = Vec::new();
        let mut return_type = None;
        let mut generic_parameters = Vec::new();
        let mut is_async = false;

        let raw_text = node.utf8_text(source).unwrap_or("");
        if raw_text.starts_with("async ") || raw_text.contains(" async ") {
            is_async = true;
        }

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
                    let is_optional = child.kind() == "optional_parameter";
                    let param_name = child
                        .child_by_field_name("pattern")
                        .and_then(|n| n.utf8_text(source).ok())
                        .unwrap_or("arg")
                        .to_string();
                    let raw_type = child
                        .child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source).ok())
                        .unwrap_or("any")
                        .trim_start_matches(':')
                        .trim();

                    parameters.push(CanonicalParameter {
                        name: param_name,
                        param_type: self.normalize_type_representation(raw_type),
                        is_optional,
                    });
                }
            }
        }

        if let Some(ret_node) = node.child_by_field_name("return_type") {
            if let Ok(ret_str) = ret_node.utf8_text(source) {
                let clean = ret_str.trim_start_matches(':').trim();
                return_type = Some(self.normalize_type_representation(clean));
            }
        }

        if let Some(generics_node) = node.child_by_field_name("type_parameters") {
            let mut cursor = generics_node.walk();
            for child in generics_node.children(&mut cursor) {
                if child.kind() == "type_parameter" {
                    if let Some(gen_str) = child
                        .child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source).ok())
                        .or_else(|| child.utf8_text(source).ok())
                    {
                        generic_parameters.push(gen_str.to_string());
                    }
                }
            }
        }

        let body_start = node
            .child_by_field_name("body")
            .map(|b| b.start_byte())
            .unwrap_or_else(|| node.end_byte());

        let sig_bytes = if body_start >= node.start_byte() && body_start <= node.end_byte() {
            &source[node.start_byte()..body_start]
        } else {
            &source[node.start_byte()..node.end_byte()]
        };

        let raw_signature = String::from_utf8_lossy(sig_bytes).trim().to_string();
        let normalized_text = raw_signature
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        CanonicalSignature {
            raw_signature,
            normalized_text,
            parameters,
            return_type,
            generic_parameters,
            is_async,
        }
    }
}

pub struct TSXSemanticAdapter {
    ts_adapter: TypeScriptSemanticAdapter,
}

impl Default for TSXSemanticAdapter {
    fn default() -> Self {
        Self {
            ts_adapter: TypeScriptSemanticAdapter,
        }
    }
}

impl SemanticAdapter for TSXSemanticAdapter {
    fn language_name(&self) -> &'static str {
        "tsx"
    }

    fn canonicalize_namespace(&self, file_path: &str) -> CanonicalNamespace {
        self.ts_adapter.canonicalize_namespace(file_path)
    }

    fn canonicalize_kind(&self, syntax_kind: &str) -> CanonicalKind {
        self.ts_adapter.canonicalize_kind(syntax_kind)
    }

    fn normalize_type_representation(&self, raw_type: &str) -> String {
        self.ts_adapter.normalize_type_representation(raw_type)
    }

    fn parse_structured_signature(&self, node: &Node, source: &[u8]) -> CanonicalSignature {
        self.ts_adapter.parse_structured_signature(node, source)
    }
}

pub fn get_adapter_for_file(file_path: &str) -> Box<dyn SemanticAdapter> {
    if file_path.ends_with(".tsx") {
        Box::new(TSXSemanticAdapter::default())
    } else if file_path.ends_with(".ts")
        || file_path.ends_with(".js")
        || file_path.ends_with(".jsx")
    {
        Box::new(TypeScriptSemanticAdapter)
    } else {
        Box::new(RustSemanticAdapter)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn canonicalize_ast_node(
    adapter: &dyn SemanticAdapter,
    node: &Node,
    source: &[u8],
    workspace_id: &WorkspaceId,
    snapshot_id: &SnapshotId,
    file_id: &FileId,
    symbol_id: &SymbolId,
    lineage_id: &LineageId,
    file_path: &str,
    symbol_name: &str,
    visibility: VisibilityKind,
) -> CanonicalSymbol {
    let canonical_namespace = adapter.canonicalize_namespace(file_path);
    let canonical_kind = adapter.canonicalize_kind(node.kind());
    let canonical_signature = adapter.parse_structured_signature(node, source);
    let entity_id = super::model::CanonicalEntityId::from_lineage(workspace_id, lineage_id);

    CanonicalSymbol {
        entity_id,
        lineage_id: lineage_id.clone(),
        workspace_id: workspace_id.clone(),
        snapshot_id: snapshot_id.clone(),
        language: adapter.language_name().to_string(),
        canonical_namespace,
        canonical_name: symbol_name.to_string(),
        canonical_kind,
        canonical_signature,
        visibility,
        file_id: file_id.clone(),
        symbol_id: symbol_id.clone(),
    }
}
