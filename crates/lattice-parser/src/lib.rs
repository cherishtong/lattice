//! Source document parsing and syntax highlighting.
//!
//! C highlighting uses the official query shipped with `tree-sitter-c`.
//! Other source extensions are initially rendered as plain UTF-8 text.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

const DOCUMENT_CACHE_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub path: String,
    pub language: &'static str,
    pub lines: Vec<Line>,
    pub symbols: Vec<Symbol>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    Definition,
    Call,
    Type,
    Macro,
    Field,
    Other,
}

impl ReferenceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Definition => "定义",
            Self::Call => "调用",
            Self::Type => "类型",
            Self::Macro => "宏",
            Self::Field => "字段",
            Self::Other => "引用",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceOccurrence {
    pub line: u32,
    pub kind: ReferenceKind,
    pub conditional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub number: u32,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub class: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: u32,
    pub end_line: u32,
    pub signature: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Struct,
    Union,
    Enum,
    Type,
    Macro,
    Class,
    Interface,
    Method,
    Constructor,
    Record,
    Annotation,
    Trait,
    Module,
    Constant,
    Impl,
}

impl SymbolKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Function => "函数",
            Self::Struct => "结构体",
            Self::Union => "联合体",
            Self::Enum => "枚举",
            Self::Type => "类型",
            Self::Macro => "宏",
            Self::Class => "类",
            Self::Interface => "接口",
            Self::Method => "方法",
            Self::Constructor => "构造器",
            Self::Record => "记录",
            Self::Annotation => "注解",
            Self::Trait => "Trait",
            Self::Module => "模块",
            Self::Constant => "常量",
            Self::Impl => "实现",
        }
    }

    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Function => "ƒ",
            Self::Struct => "S",
            Self::Union => "U",
            Self::Enum => "E",
            Self::Type => "T",
            Self::Macro => "#",
            Self::Class => "C",
            Self::Interface => "I",
            Self::Method => "m",
            Self::Constructor => "c",
            Self::Record => "R",
            Self::Annotation => "@",
            Self::Trait => "τ",
            Self::Module => "M",
            Self::Constant => "K",
            Self::Impl => "⊞",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceLanguage {
    C,
    Cpp,
    Java,
    Rust,
    TypeScript,
    Tsx,
    Text,
}

impl SourceLanguage {
    fn parser_language(self) -> tree_sitter::Language {
        match self {
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Text => unreachable!("text is not parsed as source"),
        }
    }

    fn highlights(self) -> &'static str {
        match self {
            Self::C => tree_sitter_c::HIGHLIGHT_QUERY,
            Self::Cpp => tree_sitter_cpp::HIGHLIGHT_QUERY,
            Self::Java => tree_sitter_java::HIGHLIGHTS_QUERY,
            Self::Rust => tree_sitter_rust::HIGHLIGHTS_QUERY,
            Self::TypeScript | Self::Tsx => tree_sitter_typescript::HIGHLIGHTS_QUERY,
            Self::Text => "",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Java => "Java",
            Self::Rust => "Rust",
            Self::TypeScript | Self::Tsx => "TypeScript",
            Self::Text => "Text",
        }
    }
}

fn detect_language(path: &str) -> SourceLanguage {
    match std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("c" | "h" | "m") => SourceLanguage::C,
        Some("cc" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "ixx") => SourceLanguage::Cpp,
        Some("java") => SourceLanguage::Java,
        Some("rs") => SourceLanguage::Rust,
        Some("tsx") => SourceLanguage::Tsx,
        Some("ts" | "mts" | "cts") => SourceLanguage::TypeScript,
        _ => SourceLanguage::Text,
    }
}

pub fn parse(path: &str, source: &str) -> Result<Document, String> {
    let normalized = source.replace("\r\n", "\n");
    let kind = detect_language(path);
    if kind == SourceLanguage::Text {
        return Ok(plain_document(path, &normalized));
    }

    let language = kind.parser_language();
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|error| format!("无法加载语法: {error}"))?;
    let tree = parser
        .parse(&normalized, None)
        .ok_or_else(|| "语法解析失败".to_owned())?;

    let query = Query::new(&language, kind.highlights())
        .map_err(|error| format!("高亮查询失败: {error}"))?;

    let mut ranges = vec![Vec::<HighlightRange>::new(); normalized.split('\n').count().max(1)];
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, tree.root_node(), normalized.as_bytes());
    while let Some((query_match, capture_index)) = captures.next() {
        let capture = query_match.captures[*capture_index];
        let capture_name = &query.capture_names()[capture.index as usize];
        if let Some((class, priority)) = capture_class(capture_name) {
            add_capture_ranges(
                &normalized,
                capture.node.byte_range(),
                class,
                priority,
                &mut ranges,
            );
        }
    }

    let source_lines: Vec<&str> = normalized.split('\n').collect();
    let mut lines = Vec::with_capacity(source_lines.len());
    for (index, text) in source_lines.iter().enumerate() {
        lines.push(Line {
            number: index as u32 + 1,
            spans: build_spans(text, &mut ranges[index]),
        });
    }

    let mut symbols = Vec::new();
    let mut seen = HashSet::new();
    match kind {
        SourceLanguage::Java => collect_java_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::Rust => collect_rust_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::TypeScript | SourceLanguage::Tsx => collect_typescript_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::C | SourceLanguage::Cpp => collect_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::Text => {}
    }
    symbols.sort_by_key(|symbol| (symbol.start_line, symbol.name.clone()));

    Ok(Document {
        path: path.to_owned(),
        language: kind.display_name(),
        lines,
        symbols,
    })
}

/// 提取真实标识符引用，自动排除注释与字符串。
pub fn references(
    path: &str,
    source: &str,
    name: &str,
) -> Result<Vec<ReferenceOccurrence>, String> {
    let normalized = source.replace("\r\n", "\n");
    let kind = detect_language(path);
    if kind == SourceLanguage::Text {
        return Ok(Vec::new());
    }
    let language = kind.parser_language();
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|error| format!("无法加载语法: {error}"))?;
    let tree = parser
        .parse(&normalized, None)
        .ok_or_else(|| "语法解析失败".to_owned())?;

    let mut stack = vec![tree.root_node()];
    let mut seen = HashSet::new();
    let mut occurrences = Vec::new();
    while let Some(node) = stack.pop() {
        if is_identifier_node(node.kind())
            && let Ok(text) = node.utf8_text(normalized.as_bytes())
            && text == name
            && seen.insert(node.start_byte())
        {
            occurrences.push(ReferenceOccurrence {
                line: node.start_position().row as u32 + 1,
                kind: classify_reference(node, kind),
                conditional: inside_conditional(node, normalized.as_bytes()),
            });
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    occurrences.sort_by_key(|occurrence| occurrence.line);
    Ok(occurrences)
}

fn is_identifier_node(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "type_identifier"
            | "field_identifier"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "namespace_identifier"
            | "statement_identifier"
    )
}

fn same_node(node: Node<'_>, other: Option<Node<'_>>) -> bool {
    other.is_some_and(|other| {
        other.start_byte() == node.start_byte() && other.end_byte() == node.end_byte()
    })
}

fn field_of(node: Node<'_>, fields: &[&str]) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    fields
        .iter()
        .any(|field| same_node(node, parent.child_by_field_name(field)))
}

fn classify_reference(node: Node<'_>, language: SourceLanguage) -> ReferenceKind {
    let Some(parent) = node.parent() else {
        return ReferenceKind::Other;
    };
    let parent_kind = parent.kind();

    if field_of(node, &["name", "declarator"])
        && matches!(
            parent_kind,
            "function_declarator"
                | "pointer_declarator"
                | "struct_specifier"
                | "union_specifier"
                | "enum_specifier"
                | "type_definition"
                | "preproc_def"
                | "preproc_function_def"
                | "function_definition"
                | "function_item"
                | "struct_item"
                | "enum_item"
                | "union_item"
                | "trait_item"
                | "type_item"
                | "mod_item"
                | "const_item"
                | "static_item"
                | "macro_definition"
                | "method_declaration"
                | "constructor_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "record_declaration"
                | "annotation_type_declaration"
                | "field_declaration"
                | "variable_declarator"
                | "function_declaration"
                | "method_definition"
                | "type_alias_declaration"
                | "public_field_definition"
                | "property_signature"
                | "lexical_declaration"
        )
    {
        return ReferenceKind::Definition;
    }

    if matches!(parent_kind, "macro_invocation" | "preproc_function_def")
        && field_of(node, &["macro", "name"])
    {
        return ReferenceKind::Macro;
    }
    if matches!(
        parent_kind,
        "call_expression"
            | "call"
            | "method_invocation"
            | "object_creation_expression"
            | "new_expression"
            | "function_call_expression"
    ) && field_of(node, &["function", "name", "constructor"])
    {
        return ReferenceKind::Call;
    }
    if matches!(
        parent_kind,
        "field_expression" | "member_expression" | "field_access" | "selector_expression"
    ) && field_of(node, &["field", "property", "name"])
    {
        if parent.parent().is_some_and(|grandparent| {
            matches!(
                grandparent.kind(),
                "call_expression" | "call" | "method_invocation"
            )
        }) {
            return ReferenceKind::Call;
        }
        return ReferenceKind::Field;
    }
    if node.kind() == "type_identifier"
        || matches!(
            parent_kind,
            "type_identifier"
                | "scoped_type_identifier"
                | "type_annotation"
                | "type_arguments"
                | "superclass"
                | "super_interfaces"
                | "extends_clause"
                | "implements_clause"
                | "class_heritage"
                | "trait_bounds"
                | "bounded_type"
        )
    {
        return ReferenceKind::Type;
    }
    if matches!(language, SourceLanguage::C)
        && matches!(parent_kind, "preproc_include" | "preproc_call")
    {
        return ReferenceKind::Macro;
    }
    ReferenceKind::Other
}

fn inside_conditional(node: Node<'_>, source: &[u8]) -> bool {
    let mut current = Some(node);
    while let Some(current_node) = current {
        if matches!(
            current_node.kind(),
            "preproc_if" | "preproc_ifdef" | "preproc_elif" | "preproc_else" | "preproc_elifdef"
        ) || has_cfg_attribute(current_node, source)
        {
            return true;
        }
        current = current_node.parent();
    }
    false
}

fn has_cfg_attribute(node: Node<'_>, source: &[u8]) -> bool {
    let mut previous = node.prev_named_sibling();
    while let Some(sibling) = previous {
        let kind = sibling.kind();
        if matches!(
            kind,
            "line_comment" | "block_comment" | "doc_comment" | "comment"
        ) {
            previous = sibling.prev_named_sibling();
            continue;
        }
        if kind != "attribute_item" {
            return false;
        }
        let Ok(text) = sibling.utf8_text(source) else {
            return false;
        };
        if text.contains("cfg(") || text.contains("cfg_attr(") {
            return true;
        }
        previous = sibling.prev_named_sibling();
    }
    false
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSite {
    pub name: String,
    pub line: u32,
}

/// 提取一段行范围内的函数调用（含方法调用与构造），用于调用链。
pub fn call_sites(
    path: &str,
    source: &str,
    start_line: u32,
    end_line: u32,
) -> Result<Vec<CallSite>, String> {
    let normalized = source.replace("\r\n", "\n");
    let kind = detect_language(path);
    if kind == SourceLanguage::Text {
        return Ok(Vec::new());
    }
    let language = kind.parser_language();
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|error| format!("无法加载语法: {error}"))?;
    let tree = parser
        .parse(&normalized, None)
        .ok_or_else(|| "语法解析失败".to_owned())?;
    let bytes = normalized.as_bytes();
    let first = start_line.saturating_sub(1) as usize;
    let last = end_line as usize;

    let mut stack = vec![tree.root_node()];
    let mut seen = HashSet::new();
    let mut sites = Vec::new();
    while let Some(node) = stack.pop() {
        let row = node.start_position().row;
        if row >= first
            && row < last
            && let Some(target) = call_target_node(node)
            && let Some(name) = call_name_text(target, bytes)
        {
            let line = row as u32 + 1;
            if seen.insert((name.clone(), line)) {
                sites.push(CallSite { name, line });
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    sites.sort_by_key(|site| (site.line, site.name.clone()));
    sites.truncate(300);
    Ok(sites)
}

fn call_target_node(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "call_expression" | "call" => node.child_by_field_name("function"),
        "method_invocation" => node.child_by_field_name("name"),
        "object_creation_expression" => node.child_by_field_name("type"),
        "new_expression" => node
            .child_by_field_name("constructor")
            .or_else(|| node.child_by_field_name("type")),
        _ => None,
    }
}

fn call_name_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier"
        | "field_identifier"
        | "type_identifier"
        | "property_identifier"
        | "namespace_identifier"
        | "this"
        | "super" => node.utf8_text(source).ok().map(str::to_owned),
        "field_expression" | "member_expression" | "field_access" | "selector_expression" => node
            .child_by_field_name("field")
            .or_else(|| node.child_by_field_name("property"))
            .or_else(|| node.child_by_field_name("name"))
            .or_else(|| node.named_child(node.named_child_count().saturating_sub(1)))
            .and_then(|child| call_name_text(child, source)),
        "qualified_identifier" | "scoped_identifier" => node
            .child_by_field_name("name")
            .or_else(|| node.named_child(node.named_child_count().saturating_sub(1)))
            .and_then(|child| call_name_text(child, source)),
        "generic_function" | "parenthesized_expression" | "pointer_expression" => node
            .named_child(0)
            .and_then(|child| call_name_text(child, source)),
        _ => None,
    }
}

pub fn outline(path: &str, source: &str) -> Result<Vec<Symbol>, String> {
    let normalized = source.replace("\r\n", "\n");
    let kind = detect_language(path);
    if kind == SourceLanguage::Text {
        return Ok(Vec::new());
    }
    let language = kind.parser_language();
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|error| format!("无法加载语法: {error}"))?;
    let tree = parser
        .parse(&normalized, None)
        .ok_or_else(|| "语法解析失败".to_owned())?;

    let source_lines: Vec<&str> = normalized.split('\n').collect();
    let mut symbols = Vec::new();
    let mut seen = HashSet::new();
    match kind {
        SourceLanguage::Java => collect_java_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::Rust => collect_rust_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::TypeScript | SourceLanguage::Tsx => collect_typescript_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::C | SourceLanguage::Cpp => collect_symbols(
            tree.root_node(),
            normalized.as_bytes(),
            &source_lines,
            &mut symbols,
            &mut seen,
        ),
        SourceLanguage::Text => {}
    }
    symbols.sort_by_key(|symbol| (symbol.start_line, symbol.name.clone()));
    Ok(symbols)
}

#[derive(Serialize, Deserialize)]
struct CachedDocument {
    version: u32,
    fingerprint: u64,
    lines: Vec<Vec<CachedSpan>>,
    symbols: Vec<CachedSymbol>,
}

#[derive(Serialize, Deserialize)]
struct CachedSpan {
    start: u32,
    end: u32,
    class: u8,
}

#[derive(Serialize, Deserialize)]
struct CachedSymbol {
    name: String,
    kind: u8,
    start_line: u32,
    end_line: u32,
    signature: String,
}

#[derive(Debug)]
pub struct LoadedDocument {
    pub document: Document,
    pub from_cache: bool,
}

/// Load a cached parsed document, or parse and persist it for next time.
pub fn load_or_parse_cached(
    path: &str,
    source: &str,
    cache_path: &Path,
) -> Result<LoadedDocument, String> {
    if detect_language(path) == SourceLanguage::Text {
        return parse(path, source).map(|document| LoadedDocument {
            document,
            from_cache: false,
        });
    }

    let fingerprint = content_fingerprint(source);
    if let Some(document) = load_document_cache(path, source, cache_path, fingerprint) {
        return Ok(LoadedDocument {
            document,
            from_cache: true,
        });
    }

    let document = parse(path, source)?;
    let _ = write_document_cache(&document, cache_path, fingerprint);
    Ok(LoadedDocument {
        document,
        from_cache: false,
    })
}

/// Build a document cache only when it is missing or stale. Returns whether it parsed.
pub fn ensure_document_cache(path: &str, source: &str, cache_path: &Path) -> Result<bool, String> {
    if detect_language(path) == SourceLanguage::Text {
        return Ok(false);
    }
    let fingerprint = content_fingerprint(source);
    if load_document_cache(path, source, cache_path, fingerprint).is_some() {
        return Ok(false);
    }
    let document = parse(path, source)?;
    write_document_cache(&document, cache_path, fingerprint)?;
    Ok(true)
}

fn content_fingerprint(source: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn load_document_cache(
    path: &str,
    source: &str,
    cache_path: &Path,
    fingerprint: u64,
) -> Option<Document> {
    let bytes = fs::read(cache_path).ok()?;
    let cached: CachedDocument = postcard::from_bytes(&bytes).ok()?;
    if cached.version != DOCUMENT_CACHE_VERSION || cached.fingerprint != fingerprint {
        return None;
    }

    let normalized = source.replace("\r\n", "\n");
    let source_lines: Vec<&str> = normalized.split('\n').collect();
    if source_lines.len() != cached.lines.len() {
        return None;
    }

    let mut lines = Vec::with_capacity(source_lines.len());
    for (index, (text, hits)) in source_lines.iter().zip(cached.lines.iter()).enumerate() {
        lines.push(Line {
            number: index as u32 + 1,
            spans: rebuild_spans(text, hits)?,
        });
    }

    let symbols = cached
        .symbols
        .into_iter()
        .filter_map(|symbol| {
            symbol_kind_from_code(symbol.kind).map(|kind| Symbol {
                name: symbol.name,
                kind,
                start_line: symbol.start_line,
                end_line: symbol.end_line,
                signature: symbol.signature,
            })
        })
        .collect();

    Some(Document {
        path: path.to_owned(),
        language: detect_language(path).display_name(),
        lines,
        symbols,
    })
}

fn rebuild_spans(text: &str, hits: &[CachedSpan]) -> Option<Vec<Span>> {
    if hits.is_empty() {
        return Some(if text.is_empty() {
            Vec::new()
        } else {
            vec![Span {
                text: text.to_owned(),
                class: "plain",
            }]
        });
    }

    let mut spans = Vec::new();
    let mut cursor = 0;
    for hit in hits {
        let start = hit.start as usize;
        let end = hit.end as usize;
        if start < cursor
            || start >= end
            || end > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            return None;
        }
        if start > cursor {
            spans.push(Span {
                text: text[cursor..start].to_owned(),
                class: "plain",
            });
        }
        spans.push(Span {
            text: text[start..end].to_owned(),
            class: span_class_name(hit.class)?,
        });
        cursor = end;
    }
    if cursor < text.len() {
        spans.push(Span {
            text: text[cursor..].to_owned(),
            class: "plain",
        });
    }
    Some(spans)
}

fn write_document_cache(
    document: &Document,
    cache_path: &Path,
    fingerprint: u64,
) -> Result<(), String> {
    let mut cached_lines = Vec::with_capacity(document.lines.len());
    for line in &document.lines {
        let mut offset = 0u32;
        let mut spans = Vec::new();
        for span in &line.spans {
            let start = offset;
            offset += span.text.len() as u32;
            if let Some(class) = span_class_code(span.class)
                && span.class != "plain"
            {
                spans.push(CachedSpan {
                    start,
                    end: offset,
                    class,
                });
            }
        }
        cached_lines.push(spans);
    }

    let cached = CachedDocument {
        version: DOCUMENT_CACHE_VERSION,
        fingerprint,
        lines: cached_lines,
        symbols: document
            .symbols
            .iter()
            .map(|symbol| CachedSymbol {
                name: symbol.name.clone(),
                kind: symbol_kind_code(symbol.kind),
                start_line: symbol.start_line,
                end_line: symbol.end_line,
                signature: symbol.signature.clone(),
            })
            .collect(),
    };

    let bytes =
        postcard::to_stdvec(&cached).map_err(|error| format!("序列化文档缓存失败: {error}"))?;
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建文档缓存目录失败: {error}"))?;
    }
    let temporary = cache_path.with_extension("postcard.tmp");
    fs::write(&temporary, bytes).map_err(|error| format!("写入文档缓存失败: {error}"))?;
    if cache_path.exists() {
        fs::remove_file(cache_path).map_err(|error| format!("替换文档缓存失败: {error}"))?;
    }
    fs::rename(&temporary, cache_path).map_err(|error| format!("提交文档缓存失败: {error}"))
}

fn span_class_code(class: &str) -> Option<u8> {
    match class {
        "comment" => Some(0),
        "string" => Some(1),
        "keyword" => Some(2),
        "type" => Some(3),
        "function" => Some(4),
        "constant" => Some(5),
        "number" => Some(6),
        "property" => Some(7),
        "operator" => Some(8),
        _ => None,
    }
}

fn span_class_name(code: u8) -> Option<&'static str> {
    match code {
        0 => Some("comment"),
        1 => Some("string"),
        2 => Some("keyword"),
        3 => Some("type"),
        4 => Some("function"),
        5 => Some("constant"),
        6 => Some("number"),
        7 => Some("property"),
        8 => Some("operator"),
        _ => None,
    }
}

fn symbol_kind_code(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Function => 0,
        SymbolKind::Struct => 1,
        SymbolKind::Union => 2,
        SymbolKind::Enum => 3,
        SymbolKind::Type => 4,
        SymbolKind::Macro => 5,
        SymbolKind::Class => 6,
        SymbolKind::Interface => 7,
        SymbolKind::Method => 8,
        SymbolKind::Constructor => 9,
        SymbolKind::Record => 10,
        SymbolKind::Annotation => 11,
        SymbolKind::Trait => 12,
        SymbolKind::Module => 13,
        SymbolKind::Constant => 14,
        SymbolKind::Impl => 15,
    }
}

fn symbol_kind_from_code(code: u8) -> Option<SymbolKind> {
    match code {
        0 => Some(SymbolKind::Function),
        1 => Some(SymbolKind::Struct),
        2 => Some(SymbolKind::Union),
        3 => Some(SymbolKind::Enum),
        4 => Some(SymbolKind::Type),
        5 => Some(SymbolKind::Macro),
        6 => Some(SymbolKind::Class),
        7 => Some(SymbolKind::Interface),
        8 => Some(SymbolKind::Method),
        9 => Some(SymbolKind::Constructor),
        10 => Some(SymbolKind::Record),
        11 => Some(SymbolKind::Annotation),
        12 => Some(SymbolKind::Trait),
        13 => Some(SymbolKind::Module),
        14 => Some(SymbolKind::Constant),
        15 => Some(SymbolKind::Impl),
        _ => None,
    }
}

fn plain_document(path: &str, source: &str) -> Document {
    let lines = source
        .split('\n')
        .enumerate()
        .map(|(index, text)| Line {
            number: index as u32 + 1,
            spans: if text.is_empty() {
                Vec::new()
            } else {
                vec![Span {
                    text: text.to_owned(),
                    class: "plain",
                }]
            },
        })
        .collect();

    Document {
        path: path.to_owned(),
        language: "Text",
        lines,
        symbols: Vec::new(),
    }
}

#[derive(Debug, Clone)]
struct HighlightRange {
    start: usize,
    end: usize,
    class: &'static str,
    priority: u8,
}

fn capture_class(capture_name: &str) -> Option<(&'static str, u8)> {
    let class = if capture_name.starts_with("comment") {
        ("comment", 100)
    } else if capture_name.starts_with("string") {
        ("string", 90)
    } else if capture_name.starts_with("keyword") {
        ("keyword", 85)
    } else if capture_name.starts_with("type") || capture_name.starts_with("annotation") {
        ("type", 75)
    } else if capture_name.starts_with("function")
        || capture_name.starts_with("method")
        || capture_name.starts_with("constructor")
    {
        ("function", 70)
    } else if capture_name.starts_with("constant") {
        ("constant", 65)
    } else if capture_name.starts_with("number") {
        ("number", 65)
    } else if capture_name.starts_with("property") || capture_name.starts_with("field") {
        ("property", 60)
    } else if capture_name.starts_with("label") {
        ("constant", 60)
    } else if capture_name.starts_with("operator") {
        ("operator", 50)
    } else {
        return None;
    };
    Some(class)
}

fn add_capture_ranges(
    source: &str,
    range: std::ops::Range<usize>,
    class: &'static str,
    priority: u8,
    lines: &mut [Vec<HighlightRange>],
) {
    if range.start >= range.end
        || range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return;
    }

    let bytes = source.as_bytes();
    let start_line = bytes[..range.start]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count();
    let end_line = bytes[..range.end.saturating_sub(1)]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count();

    let mut line_start = source[..range.start]
        .rfind('\n')
        .map_or(0, |index| index + 1);

    for line_index in start_line..=end_line.min(lines.len().saturating_sub(1)) {
        let line_end = source[line_start..]
            .find('\n')
            .map_or(source.len(), |offset| line_start + offset);
        let local_start = range.start.max(line_start) - line_start;
        let local_end = range.end.min(line_end) - line_start;
        if local_start < local_end {
            lines[line_index].push(HighlightRange {
                start: local_start,
                end: local_end,
                class,
                priority,
            });
        }
        line_start = line_end + 1;
    }
}

fn build_spans(text: &str, ranges: &mut [HighlightRange]) -> Vec<Span> {
    if text.is_empty() {
        return Vec::new();
    }

    ranges.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| b.end.cmp(&a.end))
    });

    let mut spans = Vec::new();
    let mut cursor = 0;
    for range in ranges.iter() {
        let start = range.start.min(text.len());
        let end = range.end.min(text.len());
        if start < cursor || start >= end {
            continue;
        }
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            continue;
        }
        if start > cursor && text.is_char_boundary(cursor) {
            spans.push(Span {
                text: text[cursor..start].to_owned(),
                class: "plain",
            });
        }
        spans.push(Span {
            text: text[start..end].to_owned(),
            class: range.class,
        });
        cursor = end;
    }

    if cursor < text.len() && text.is_char_boundary(cursor) {
        spans.push(Span {
            text: text[cursor..].to_owned(),
            class: "plain",
        });
    }
    spans
}

fn collect_symbols(
    root: Node<'_>,
    source: &[u8],
    source_lines: &[&str],
    symbols: &mut Vec<Symbol>,
    seen: &mut HashSet<(String, u32)>,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let kind = match node.kind() {
            "function_definition" => Some(
                if inside_cpp_container(node) || has_qualified_declarator(node) {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                },
            ),
            "field_declaration" if has_function_declarator(node) => Some(SymbolKind::Method),
            "class_specifier" => Some(SymbolKind::Class),
            "namespace_definition" => Some(SymbolKind::Module),
            "struct_specifier" => Some(SymbolKind::Struct),
            "union_specifier" => Some(SymbolKind::Union),
            "enum_specifier" => Some(SymbolKind::Enum),
            "type_definition" => Some(SymbolKind::Type),
            "alias_declaration" => Some(SymbolKind::Type),
            "preproc_def" | "preproc_function_def" => Some(SymbolKind::Macro),
            _ => None,
        };

        if let Some(kind) = kind
            && let Some(name_node) = symbol_name_node(node, kind)
            && let Ok(name) = name_node.utf8_text(source)
        {
            let name = name.to_owned();
            let start_line = node.start_position().row as u32 + 1;
            let end_line = node.end_position().row as u32 + 1;
            if seen.insert((name.clone(), start_line)) {
                let signature = source_lines
                    .get(node.start_position().row)
                    .map(|line| line.trim())
                    .unwrap_or_default();
                symbols.push(Symbol {
                    name,
                    kind,
                    start_line,
                    end_line,
                    signature: truncate(signature, 96),
                });
            }
        }

        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
}

fn symbol_name_node<'tree>(node: Node<'tree>, kind: SymbolKind) -> Option<Node<'tree>> {
    match kind {
        SymbolKind::Function | SymbolKind::Method | SymbolKind::Constructor => {
            let declarator = node.child_by_field_name("declarator")?;
            function_name_node(declarator)
        }
        SymbolKind::Struct
        | SymbolKind::Union
        | SymbolKind::Enum
        | SymbolKind::Class
        | SymbolKind::Module => node.child_by_field_name("name"),
        SymbolKind::Macro => node.child_by_field_name("name"),
        SymbolKind::Type => node
            .child_by_field_name("declarator")
            .and_then(find_identifier)
            .or_else(|| node.child_by_field_name("name")),
        _ => None,
    }
}

fn has_qualified_declarator(node: Node<'_>) -> bool {
    node.child_by_field_name("declarator")
        .and_then(|declarator| declarator.child_by_field_name("declarator"))
        .is_some_and(|name| name.kind() == "qualified_identifier")
}

fn has_function_declarator(node: Node<'_>) -> bool {
    let Some(declarator) = node.child_by_field_name("declarator") else {
        return false;
    };
    declarator.kind() == "function_declarator"
        || declarator
            .named_child(0)
            .is_some_and(|child| child.kind() == "function_declarator")
}

fn function_name_node(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "function_declarator" => node
            .child_by_field_name("declarator")
            .and_then(function_name_node),
        "qualified_identifier" => node
            .child_by_field_name("name")
            .and_then(function_name_node)
            .or_else(|| node.named_child(node.named_child_count().saturating_sub(1))),
        "identifier" | "field_identifier" | "destructor_name" | "operator_name" => Some(node),
        _ => find_identifier(node),
    }
}

fn inside_cpp_container(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "class_specifier" | "struct_specifier" => return true,
            "function_definition" | "translation_unit" => return false,
            _ => current = parent.parent(),
        }
    }
    false
}

fn collect_java_symbols(
    root: Node<'_>,
    source: &[u8],
    source_lines: &[&str],
    symbols: &mut Vec<Symbol>,
    seen: &mut HashSet<(String, u32)>,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let kind = match node.kind() {
            "class_declaration" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            "enum_declaration" => Some(SymbolKind::Enum),
            "record_declaration" => Some(SymbolKind::Record),
            "annotation_type_declaration" => Some(SymbolKind::Annotation),
            "method_declaration" => Some(SymbolKind::Method),
            "constructor_declaration" => Some(SymbolKind::Constructor),
            _ => None,
        };

        if let Some(kind) = kind
            && let Some(name_node) = node.child_by_field_name("name")
            && let Ok(name) = name_node.utf8_text(source)
        {
            let name = name.to_owned();
            let start_line = node.start_position().row as u32 + 1;
            let end_line = node.end_position().row as u32 + 1;
            if seen.insert((name.clone(), start_line)) {
                let signature = source_lines
                    .get(node.start_position().row)
                    .map(|line| line.trim())
                    .unwrap_or_default();
                symbols.push(Symbol {
                    name,
                    kind,
                    start_line,
                    end_line,
                    signature: truncate(signature, 96),
                });
            }
        }

        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
}

fn collect_rust_symbols(
    root: Node<'_>,
    source: &[u8],
    source_lines: &[&str],
    symbols: &mut Vec<Symbol>,
    seen: &mut HashSet<(String, u32)>,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let kind = match node.kind() {
            "function_item" | "function_signature_item" => Some(if inside_impl_or_trait(node) {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            }),
            "struct_item" => Some(SymbolKind::Struct),
            "enum_item" => Some(SymbolKind::Enum),
            "union_item" => Some(SymbolKind::Union),
            "trait_item" => Some(SymbolKind::Trait),
            "mod_item" => Some(SymbolKind::Module),
            "macro_definition" => Some(SymbolKind::Macro),
            "type_item" => Some(SymbolKind::Type),
            "const_item" | "static_item" => Some(SymbolKind::Constant),
            _ => None,
        };
        if let Some(kind) = kind {
            push_node_symbol(node, kind, source, source_lines, symbols, seen);
        } else if node.kind() == "impl_item" {
            let type_name = node
                .child_by_field_name("type")
                .and_then(|node| node.utf8_text(source).ok())
                .unwrap_or("…");
            let name = match node
                .child_by_field_name("trait")
                .and_then(|node| node.utf8_text(source).ok())
            {
                Some(trait_name) => format!("impl {trait_name} for {type_name}"),
                None => format!("impl {type_name}"),
            };
            push_named_symbol(node, name, SymbolKind::Impl, source_lines, symbols, seen);
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
}

fn collect_typescript_symbols(
    root: Node<'_>,
    source: &[u8],
    source_lines: &[&str],
    symbols: &mut Vec<Symbol>,
    seen: &mut HashSet<(String, u32)>,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let kind = match node.kind() {
            "class_declaration" | "abstract_class_declaration" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            "enum_declaration" => Some(SymbolKind::Enum),
            "type_alias_declaration" => Some(SymbolKind::Type),
            "function_declaration" => Some(SymbolKind::Function),
            "method_definition" => {
                let name = node
                    .child_by_field_name("name")
                    .and_then(|name| name.utf8_text(source).ok())
                    .unwrap_or_default();
                Some(if name == "constructor" {
                    SymbolKind::Constructor
                } else {
                    SymbolKind::Method
                })
            }
            "module" | "internal_module" => Some(SymbolKind::Module),
            "variable_declarator" => {
                let value_kind = node
                    .child_by_field_name("value")
                    .map(|value| value.kind())
                    .unwrap_or_default();
                if matches!(
                    value_kind,
                    "arrow_function" | "function_expression" | "function"
                ) {
                    Some(SymbolKind::Function)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(kind) = kind {
            push_node_symbol(node, kind, source, source_lines, symbols, seen);
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
}

fn inside_impl_or_trait(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "impl_item" | "trait_item" => return true,
            "function_item" | "source_file" => return false,
            _ => current = parent.parent(),
        }
    }
    false
}

fn push_node_symbol(
    node: Node<'_>,
    kind: SymbolKind,
    source: &[u8],
    source_lines: &[&str],
    symbols: &mut Vec<Symbol>,
    seen: &mut HashSet<(String, u32)>,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    let Ok(name) = name_node.utf8_text(source) else {
        return;
    };
    push_named_symbol(node, name.to_owned(), kind, source_lines, symbols, seen);
}

fn push_named_symbol(
    node: Node<'_>,
    name: String,
    kind: SymbolKind,
    source_lines: &[&str],
    symbols: &mut Vec<Symbol>,
    seen: &mut HashSet<(String, u32)>,
) {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    if !seen.insert((name.clone(), start_line)) {
        return;
    }
    let signature = source_lines
        .get(node.start_position().row)
        .map(|line| line.trim())
        .unwrap_or_default();
    symbols.push(Symbol {
        name,
        kind,
        start_line,
        end_line,
        signature: truncate(signature, 96),
    });
}

fn find_identifier(root: Node<'_>) -> Option<Node<'_>> {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if matches!(
            node.kind(),
            "identifier" | "field_identifier" | "type_identifier"
        ) {
            return Some(node);
        }

        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
        if let Some(declarator) = node.child_by_field_name("declarator") {
            stack.push(declarator);
        }
    }
    None
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let mut truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlines_c_functions_and_structs() {
        let source = r#"
struct Point {
    int x;
    int y;
};

static int add(int a, int b) {
    return a + b;
}
"#;
        let document = parse("point.c", source).expect("parse");
        assert!(document.symbols.iter().any(|symbol| symbol.name == "Point"));
        assert!(document.symbols.iter().any(|symbol| symbol.name == "add"));
        assert_eq!(document.lines.len(), source.split('\n').count());
    }

    #[test]
    fn highlights_comments_and_keywords() {
        let document = parse("sample.c", "/* note */\nstatic int value = 1;\n").expect("parse");
        let classes: Vec<&str> = document.lines[0]
            .spans
            .iter()
            .map(|span| span.class)
            .collect();
        assert!(classes.contains(&"comment"));
        let classes: Vec<&str> = document.lines[1]
            .spans
            .iter()
            .map(|span| span.class)
            .collect();
        assert!(classes.contains(&"keyword"));
        assert!(classes.contains(&"type"));
    }

    #[test]
    fn outlines_java_types_methods_and_constructors() {
        let source = r#"
package demo;

public final class Box {
    private int value;

    public Box(int value) {
        this.value = value;
    }

    public int value() {
        return value;
    }

    interface Marker {}
}
"#;
        let document = parse("Box.java", source).expect("parse Java");
        assert_eq!(document.language, "Java");
        assert!(
            document
                .symbols
                .iter()
                .any(|symbol| { symbol.name == "Box" && symbol.kind == SymbolKind::Class })
        );
        assert!(
            document
                .symbols
                .iter()
                .any(|symbol| { symbol.name == "Box" && symbol.kind == SymbolKind::Constructor })
        );
        assert!(
            document
                .symbols
                .iter()
                .any(|symbol| { symbol.name == "value" && symbol.kind == SymbolKind::Method })
        );
        assert!(
            document
                .symbols
                .iter()
                .any(|symbol| { symbol.name == "Marker" && symbol.kind == SymbolKind::Interface })
        );
        assert!(
            document
                .lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| { span.class == "keyword" && span.text.contains("class") })
        );
    }

    #[test]
    fn outlines_rust_items_and_methods() {
        let source = r#"
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new() -> Self {
        Self { x: 0, y: 0 }
    }
}

pub trait Draw {
    fn draw(&self);
}

pub mod inner {
    pub fn helper() {}
}

macro_rules! demo {
    () => {};
}
"#;
        let document = parse("point.rs", source).expect("parse Rust");
        assert_eq!(document.language, "Rust");
        for (name, kind) in [
            ("Point", SymbolKind::Struct),
            ("new", SymbolKind::Method),
            ("Draw", SymbolKind::Trait),
            ("draw", SymbolKind::Method),
            ("inner", SymbolKind::Module),
            ("helper", SymbolKind::Function),
            ("demo", SymbolKind::Macro),
        ] {
            assert!(
                document
                    .symbols
                    .iter()
                    .any(|symbol| symbol.name == name && symbol.kind == kind),
                "missing {name} as {kind:?}"
            );
        }
        assert!(
            document
                .symbols
                .iter()
                .any(|symbol| symbol.name == "impl Point" && symbol.kind == SymbolKind::Impl),
            "missing impl block symbol"
        );
        assert!(
            document
                .lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.class == "keyword" && span.text.contains("struct"))
        );
    }

    #[test]
    fn outlines_cpp_classes_namespaces_and_methods() {
        let source = r#"
namespace demo {
class Shape {
public:
    virtual double area() const = 0;
    void reset() { value = 0; }
private:
    int value;
};
double Shape::scale(double factor) { return factor; }
}
"#;
        let document = parse("shape.cpp", source).expect("parse C++");
        assert_eq!(document.language, "C++");
        for (name, kind) in [
            ("demo", SymbolKind::Module),
            ("Shape", SymbolKind::Class),
            ("area", SymbolKind::Method),
            ("reset", SymbolKind::Method),
            ("scale", SymbolKind::Method),
        ] {
            assert!(
                document
                    .symbols
                    .iter()
                    .any(|symbol| symbol.name == name && symbol.kind == kind),
                "missing {name} as {kind:?}"
            );
        }
    }

    #[test]
    fn outlines_typescript_declarations_and_arrow_functions() {
        let source = r#"
export interface Shape {
    area(): number;
}

export class Circle implements Shape {
    constructor(private radius: number) {}
    area(): number {
        return Math.PI * this.radius ** 2;
    }
}

export type Id = string;
export enum Kind { A, B }
export function make(): Circle {
    return new Circle(1);
}
export const helper = () => 1;

namespace NS {
    export function inner() {}
}
"#;
        let document = parse("shape.ts", source).expect("parse TypeScript");
        assert_eq!(document.language, "TypeScript");
        for (name, kind) in [
            ("Shape", SymbolKind::Interface),
            ("Circle", SymbolKind::Class),
            ("constructor", SymbolKind::Constructor),
            ("area", SymbolKind::Method),
            ("Id", SymbolKind::Type),
            ("Kind", SymbolKind::Enum),
            ("make", SymbolKind::Function),
            ("helper", SymbolKind::Function),
            ("NS", SymbolKind::Module),
            ("inner", SymbolKind::Function),
        ] {
            assert!(
                document
                    .symbols
                    .iter()
                    .any(|symbol| symbol.name == name && symbol.kind == kind),
                "missing {name} as {kind:?}"
            );
        }
    }

    #[test]
    fn references_skip_comments_and_strings() {
        let source = r#"
int parse(int value) { return value; }
void run(void) {
    parse(1);
    // parse should stay out of the results
    const char *label = "parse";
}
"#;
        let occurrences = references("sample.c", source, "parse").expect("references");
        assert_eq!(
            occurrences.iter().map(|item| item.line).collect::<Vec<_>>(),
            vec![2, 4]
        );
        assert_eq!(occurrences[0].kind, ReferenceKind::Definition);
        assert_eq!(occurrences[1].kind, ReferenceKind::Call);
    }

    #[test]
    fn extracts_call_sites_from_a_line_range() {
        let source = r#"
static int helper(int value) { return value; }
static int run(int value) {
    int first = helper(value);
    int second = helper(first);
    return second;
}
static int other(void) { return helper(1); }
"#;
        let sites = call_sites("sample.c", source, 3, 6).expect("call sites");
        assert_eq!(sites.len(), 2);
        assert!(sites.iter().all(|site| site.name == "helper"));
        assert!(sites.iter().all(|site| site.line >= 4 && site.line <= 5));
    }

    #[test]
    fn extracts_java_and_typescript_method_calls() {
        let java = r#"
class Demo {
    void run() {
        list.add(1);
        helper();
    }
}
"#;
        let sites = call_sites("Demo.java", java, 3, 6).expect("java calls");
        assert!(sites.iter().any(|site| site.name == "add"));
        assert!(sites.iter().any(|site| site.name == "helper"));

        let typescript = r#"
function run(shape: Shape) {
    shape.area();
    parse(1);
}
"#;
        let sites = call_sites("run.ts", typescript, 2, 5).expect("ts calls");
        assert!(sites.iter().any(|site| site.name == "area"));
        assert!(sites.iter().any(|site| site.name == "parse"));
    }

    #[test]
    fn references_mark_conditional_branches() {
        let source = r#"
fn parse() {}
#[cfg(feature = "extra")]
fn extra() { parse(); }
fn plain() { parse(); }
"#;
        let occurrences = references("sample.rs", source, "parse").expect("references");
        let conditional: Vec<bool> = occurrences.iter().map(|item| item.conditional).collect();
        assert_eq!(conditional, vec![false, true, false]);
    }

    #[test]
    fn references_classify_typescript_calls_and_types() {
        let source = r#"
interface Shape { area(): number }
function parse(shape: Shape): number {
    return shape.area();
}
parse({ area: () => 1 });
const label = "parse";
"#;
        let occurrences = references("shape.ts", source, "parse").expect("references");
        let kinds: Vec<ReferenceKind> = occurrences.iter().map(|item| item.kind).collect();
        assert!(kinds.contains(&ReferenceKind::Definition));
        assert!(kinds.contains(&ReferenceKind::Call));
        assert!(occurrences.iter().all(|item| item.line != 7));
    }
}
