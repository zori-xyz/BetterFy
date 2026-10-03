//! Compiled Panorama layouts (`.vxml_c`) edited the way Minify edits them.
//!
//! Valve's compiler stores a layout as a KV3 tree in the resource's `LaCo`
//! block: `{ m_AST: { m_pRoot: node } }`, where every node has an `eType`, an
//! optional `name`, its children (`child` when there is one, `vecChildren`
//! when there are several) and a `sourceLineColumn`. Minify decompiles the
//! layout to XML, applies the actions in a mod's `xml.json` with
//! ElementTree and recompiles it with the Workshop Tools. BetterFy applies
//! the same actions to the tree directly, with the same selector rules and
//! the same "first match" order, so no Valve tooling is needed.
//!
//! The compiler leaves `m_ChildResourceList` empty even for layouts that
//! include scripts and styles, so the includes are resolved by path when the
//! layout loads and editing the tree alone matches what the compiler emits.
//! Every other block is kept byte for byte.

use crate::kv3::{self, Value};
use crate::panorama;
use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const MAX_EDITS_BYTES: usize = 256 * 1024;
const MAX_SNIPPET_BYTES: usize = 64 * 1024;
const MAX_NODES: usize = 200_000;
const MAX_DEPTH: usize = 128;

/// The section Minify's Auto Accept Match adds to the game settings for mod
/// menus, copied from its `script_after_decompile.py`.
const MENU_SECTION: &str = r##"<Panel class="SettingsSectionContainer" section="#minify" icon="s2r://panorama/images/control_icons/24px/check.vsvg">
  <Panel class="SettingsSectionTitleContainer LeftRightFlow">
    <Image class="SettingsSectionTitleIcon" texturewidth="48px" textureheight="48px" scaling="stretch-to-fit-preserve-aspect" src="s2r://panorama/images/control_icons/24px/check.vsvg" />
    <Label class="SettingsSectionTitle" text="Minify" />
  </Panel>
</Panel>"##;
pub(crate) const MENU_TARGET: &str = "panorama/layout/popups/popup_settings_reborn.vxml_c";
const MENU_PARENT_TAG: &str = "PopupSettingsRebornSettingsBody";

fn invalid() -> String {
    "panorama_layout_invalid".to_string()
}

fn missing() -> String {
    "panorama_layout_target_missing".to_string()
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Node {
    kind: String,
    name: Option<String>,
    children: Vec<Node>,
    position: Option<Value>,
}

impl Node {
    fn new(kind: &str, name: Option<&str>) -> Self {
        Self {
            kind: kind.to_string(),
            name: name.map(String::from),
            children: Vec::new(),
            position: None,
        }
    }

    fn from_value(value: &Value, depth: usize, count: &mut usize) -> Result<Self, String> {
        *count += 1;
        if depth > MAX_DEPTH || *count > MAX_NODES {
            return Err(invalid());
        }
        let Value::Object(entries) = value else {
            return Err(invalid());
        };
        let mut node = Node::new("", None);
        let mut has_children = false;
        for (key, item) in entries {
            match (key.as_str(), item) {
                ("eType", Value::String(kind)) if node.kind.is_empty() => node.kind = kind.clone(),
                ("name", Value::String(name)) if node.name.is_none() => {
                    node.name = Some(name.clone())
                }
                ("child", child) if !has_children => {
                    has_children = true;
                    node.children
                        .push(Node::from_value(child, depth + 1, count)?);
                }
                ("vecChildren", Value::Array(children)) if !has_children => {
                    has_children = true;
                    for child in children {
                        node.children
                            .push(Node::from_value(child, depth + 1, count)?);
                    }
                }
                ("sourceLineColumn", position) if node.position.is_none() => {
                    node.position = Some(position.clone())
                }
                _ => return Err("panorama_layout_unsupported".to_string()),
            }
        }
        if node.kind.is_empty() {
            return Err(invalid());
        }
        Ok(node)
    }

    fn to_value(&self) -> Value {
        let mut entries = vec![("eType".to_string(), Value::String(self.kind.clone()))];
        if let Some(name) = &self.name {
            entries.push(("name".to_string(), Value::String(name.clone())));
        }
        match self.children.as_slice() {
            [] => {}
            [child] => entries.push(("child".to_string(), child.to_value())),
            children => entries.push((
                "vecChildren".to_string(),
                Value::Array(children.iter().map(Node::to_value).collect()),
            )),
        }
        if let Some(position) = &self.position {
            entries.push(("sourceLineColumn".to_string(), position.clone()));
        }
        Value::Object(entries)
    }

    /// The XML tag this node decompiles to, or `None` for attributes and
    /// attribute values, which are not elements.
    fn tag(&self) -> Option<&str> {
        match self.kind.as_str() {
            "ROOT" => Some("root"),
            "STYLES" => Some("styles"),
            "SCRIPTS" => Some("scripts"),
            "SNIPPETS" => Some("snippets"),
            "SNIPPET" => Some("snippet"),
            "INCLUDE" => Some("include"),
            "PANEL" => self.name.as_deref(),
            _ => None,
        }
    }

    fn attribute(&self, key: &str) -> Option<String> {
        match self.kind.as_str() {
            "PANEL" => self
                .children
                .iter()
                .find(|child| child.kind == "PANEL_ATTRIBUTE" && child.name.as_deref() == Some(key))
                .and_then(|attribute| attribute.children.first())
                .and_then(value_text),
            "SNIPPET" if key == "name" => self.name.clone(),
            "INCLUDE" if key == "src" => self.children.first().and_then(value_text),
            _ => None,
        }
    }

    fn element_children(&self) -> impl Iterator<Item = (usize, &Node)> {
        self.children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.tag().is_some())
    }

    fn count(&self) -> usize {
        1 + self.children.iter().map(Node::count).sum::<usize>()
    }
}

/// The compiler leaves `name` out of an empty attribute value
/// (`onactivate=""`).
fn value_text(node: &Node) -> Option<String> {
    match node.kind.as_str() {
        "REFERENCE_COMPILED" => Some(format!("s2r://{}", node.name.as_deref()?)),
        "REFERENCE_PASSTHROUGH" => Some(format!("file://{}", node.name.as_deref()?)),
        "PANEL_ATTRIBUTE_VALUE" => Some(node.name.clone().unwrap_or_default()),
        _ => None,
    }
}

fn value_node(text: &str) -> Node {
    if let Some(path) = text.strip_prefix("s2r://") {
        Node::new("REFERENCE_COMPILED", Some(path))
    } else if let Some(path) = text.strip_prefix("file://") {
        Node::new("REFERENCE_PASSTHROUGH", Some(path))
    } else if text.is_empty() {
        Node::new("PANEL_ATTRIBUTE_VALUE", None)
    } else {
        Node::new("PANEL_ATTRIBUTE_VALUE", Some(text))
    }
}

fn attribute_node(key: &str, value: &str) -> Node {
    let mut attribute = Node::new("PANEL_ATTRIBUTE", Some(key));
    attribute.children.push(value_node(value));
    attribute
}

/// A compiled layout with its tree decoded.
#[derive(Debug, Clone)]
pub(crate) struct Layout {
    resource: panorama::Resource,
    layout_block: usize,
    format: [u8; 16],
    pub(crate) root: Node,
}

pub(crate) fn read(bytes: &[u8]) -> Result<Layout, String> {
    let resource = panorama::parse(bytes)?;
    let layout_block = resource
        .blocks
        .iter()
        .position(|block| &block.tag == b"LaCo")
        .ok_or_else(|| "panorama_layout_unsupported".to_string())?;
    let document = kv3::parse(&resource.blocks[layout_block].bytes)?;
    let Value::Object(top) = &document.root else {
        return Err(invalid());
    };
    let [(ast_key, Value::Object(ast))] = top.as_slice() else {
        return Err("panorama_layout_unsupported".to_string());
    };
    let [(root_key, root)] = ast.as_slice() else {
        return Err("panorama_layout_unsupported".to_string());
    };
    if ast_key != "m_AST" || root_key != "m_pRoot" {
        return Err("panorama_layout_unsupported".to_string());
    }
    let root = Node::from_value(root, 0, &mut 0)?;
    if root.kind != "ROOT" {
        return Err(invalid());
    }
    Ok(Layout {
        format: document.format,
        resource,
        layout_block,
        root,
    })
}

pub(crate) fn write(layout: &Layout) -> Result<Vec<u8>, String> {
    if layout.root.count() > MAX_NODES {
        return Err(invalid());
    }
    let document = kv3::Document {
        format: layout.format,
        root: Value::Object(vec![(
            "m_AST".to_string(),
            Value::Object(vec![("m_pRoot".to_string(), layout.root.to_value())]),
        )]),
    };
    let mut resource = layout.resource.clone();
    resource.blocks[layout.layout_block].bytes = kv3::write(&document);
    panorama::build(&resource)
}

// ---------- Minify's selectors ----------

struct Selector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<(String, String)>,
}

fn selector_patterns() -> &'static (Regex, Regex) {
    static PATTERNS: OnceLock<(Regex, Regex)> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        (
            Regex::new(
                r#"^([a-zA-Z0-9_-]+)?(?:#([a-zA-Z0-9_-]+))?((?:\.[a-zA-Z0-9_-]+)*)((?:\[[a-zA-Z0-9_-]+=['"]?[^'"\]]+['"]?\])*)$"#,
            )
            .expect("valid selector pattern"),
            Regex::new(r#"\[([a-zA-Z0-9_-]+)=['"]?([^'"\]]+)['"]?\]"#)
                .expect("valid attribute pattern"),
        )
    })
}

impl Selector {
    fn parse(text: &str) -> Result<Self, String> {
        let (selector, attribute) = selector_patterns();
        let captures = selector.captures(text).ok_or_else(invalid)?;
        let group = |index| captures.get(index).map(|found| found.as_str().to_string());
        let selector = Selector {
            tag: group(1),
            id: group(2),
            classes: captures
                .get(3)
                .map(|found| {
                    found
                        .as_str()
                        .split('.')
                        .filter(|class| !class.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            attributes: captures
                .get(4)
                .map(|found| {
                    attribute
                        .captures_iter(found.as_str())
                        .map(|pair| (pair[1].to_string(), pair[2].to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        };
        if text.is_empty() {
            return Err(invalid());
        }
        Ok(selector)
    }

    fn matches(&self, node: &Node) -> bool {
        let Some(tag) = node.tag() else {
            return false;
        };
        if self.tag.as_deref().is_some_and(|wanted| wanted != tag) {
            return false;
        }
        if self
            .id
            .as_ref()
            .is_some_and(|id| node.attribute("id").as_ref() != Some(id))
        {
            return false;
        }
        if !self.classes.is_empty() {
            let classes = node.attribute("class").unwrap_or_default();
            let classes = classes.split_whitespace().collect::<Vec<_>>();
            if !self
                .classes
                .iter()
                .all(|class| classes.contains(&class.as_str()))
            {
                return false;
            }
        }
        self.attributes
            .iter()
            .all(|(key, value)| node.attribute(key).as_ref() == Some(value))
    }
}

fn node_at<'a>(root: &'a Node, path: &[usize]) -> &'a Node {
    path.iter().fold(root, |node, index| &node.children[*index])
}

fn node_at_mut<'a>(root: &'a mut Node, path: &[usize]) -> &'a mut Node {
    path.iter()
        .fold(root, |node, index| &mut node.children[*index])
}

/// Every element in document order, as ElementTree's `iter()` visits them.
fn element_paths(node: &Node, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    out.push(path.clone());
    for (index, child) in node.element_children() {
        path.push(index);
        element_paths(child, path, out);
        path.pop();
    }
}

fn all_elements(root: &Node) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    element_paths(root, &mut Vec::new(), &mut out);
    out
}

/// Minify's `find_by_selector`: the first match in document order.
fn find(root: &Node, selector: &Selector) -> Option<Vec<usize>> {
    all_elements(root)
        .into_iter()
        .find(|path| selector.matches(node_at(root, path)))
}

/// Minify's `find_with_parent_by_selector`: the root, or else the first match
/// among the children of each element taken in document order. This visits
/// nodes in a different order than [`find`], exactly as Minify does.
fn find_with_parent(root: &Node, selector: &Selector) -> Option<Vec<usize>> {
    if selector.matches(root) {
        return Some(Vec::new());
    }
    for parent in all_elements(root) {
        for (index, child) in node_at(root, &parent).element_children() {
            if selector.matches(child) {
                let mut path = parent.clone();
                path.push(index);
                return Some(path);
            }
        }
    }
    None
}

// ---------- XML snippets ----------

#[derive(Debug, PartialEq)]
struct Element {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<Element>,
}

struct XmlReader<'a> {
    text: &'a str,
    at: usize,
}

impl XmlReader<'_> {
    fn rest(&self) -> &str {
        &self.text[self.at..]
    }

    fn skip_space_and_comments(&mut self) -> Result<(), String> {
        loop {
            let skipped = self.rest().len() - self.rest().trim_start().len();
            self.at += skipped;
            if self.rest().starts_with("<!--") {
                let end = self.rest()[4..].find("-->").ok_or_else(invalid)?;
                self.at += 4 + end + 3;
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> Result<String, String> {
        let length = self
            .rest()
            .find(|character: char| {
                !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':'))
            })
            .unwrap_or(self.rest().len());
        if length == 0 {
            return Err(invalid());
        }
        let name = self.rest()[..length].to_string();
        self.at += length;
        Ok(name)
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        if !self.rest().starts_with(token) {
            return Err(invalid());
        }
        self.at += token.len();
        Ok(())
    }

    fn element(&mut self, depth: usize) -> Result<Element, String> {
        if depth > MAX_DEPTH {
            return Err(invalid());
        }
        self.expect("<")?;
        let name = self.name()?;
        let mut attributes: Vec<(String, String)> = Vec::new();
        loop {
            self.skip_space_and_comments()?;
            if self.rest().starts_with("/>") {
                self.at += 2;
                return Ok(Element {
                    name,
                    attributes,
                    children: Vec::new(),
                });
            }
            if self.rest().starts_with('>') {
                self.at += 1;
                break;
            }
            let key = self.name()?;
            self.skip_space_and_comments()?;
            self.expect("=")?;
            self.skip_space_and_comments()?;
            let quote = self.rest().chars().next().ok_or_else(invalid)?;
            if quote != '"' && quote != '\'' {
                return Err(invalid());
            }
            self.at += 1;
            let end = self.rest().find(quote).ok_or_else(invalid)?;
            let raw = &self.rest()[..end];
            if raw.contains('<') || attributes.iter().any(|(existing, _)| *existing == key) {
                return Err(invalid());
            }
            let value = unescape(raw)?;
            self.at += end + 1;
            attributes.push((key, value));
        }
        let mut children = Vec::new();
        loop {
            self.skip_space_and_comments()?;
            if self.rest().starts_with("</") {
                self.at += 2;
                if self.name()? != name {
                    return Err(invalid());
                }
                self.skip_space_and_comments()?;
                self.expect(">")?;
                return Ok(Element {
                    name,
                    attributes,
                    children,
                });
            }
            if !self.rest().starts_with('<') {
                // Panorama layouts carry no text content.
                return Err(invalid());
            }
            children.push(self.element(depth + 1)?);
        }
    }
}

fn unescape(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let end = rest[start..].find(';').ok_or_else(invalid)? + start;
        let entity = &rest[start + 1..end];
        let character = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).map_err(|_| invalid())?
                } else if let Some(decimal) = entity.strip_prefix('#') {
                    decimal.parse::<u32>().map_err(|_| invalid())?
                } else {
                    return Err(invalid());
                };
                char::from_u32(code).ok_or_else(invalid)?
            }
        };
        out.push(character);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn parse_snippet(text: &str) -> Result<Element, String> {
    if text.len() > MAX_SNIPPET_BYTES {
        return Err(invalid());
    }
    let mut reader = XmlReader { text, at: 0 };
    reader.skip_space_and_comments()?;
    let element = reader.element(0)?;
    reader.skip_space_and_comments()?;
    if reader.at != text.len() {
        return Err(invalid());
    }
    Ok(element)
}

/// Converts an XML element to the node Valve's compiler would emit for it.
/// New nodes take the source position of the place they are inserted at.
fn element_to_node(element: &Element, position: &Value) -> Result<Node, String> {
    let attribute = |key: &str| {
        element
            .attributes
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    };
    let mut node = match element.name.to_ascii_lowercase().as_str() {
        "root" => return Err(invalid()),
        "styles" => Node::new("STYLES", None),
        "scripts" => Node::new("SCRIPTS", None),
        "snippets" => Node::new("SNIPPETS", None),
        "snippet" => Node::new("SNIPPET", Some(attribute("name").ok_or_else(invalid)?)),
        "include" => {
            let source = attribute("src").ok_or_else(invalid)?;
            if !source.starts_with("s2r://") && !source.starts_with("file://") {
                return Err(invalid());
            }
            let mut include = Node::new("INCLUDE", None);
            include.children.push(value_node(source));
            include.position = Some(position.clone());
            return Ok(include);
        }
        _ => {
            let mut panel = Node::new("PANEL", Some(&element.name));
            for (key, value) in &element.attributes {
                panel.children.push(attribute_node(key, value));
            }
            panel
        }
    };
    for child in &element.children {
        node.children.push(element_to_node(child, position)?);
    }
    node.position = Some(position.clone());
    Ok(node)
}

fn anchor_position(node: &Node) -> Value {
    match &node.position {
        Some(Value::Array(items)) if items.len() == 2 => Value::Array(items.clone()),
        _ => Value::Array(vec![Value::Int(1), Value::Int(1)]),
    }
}

// ---------- xml.json ----------

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Edit {
    AddScript {
        src: String,
    },
    AddStyleInclude {
        src: String,
    },
    SetAttribute {
        selector: String,
        attribute: String,
        value: String,
    },
    AddChild {
        selector: String,
        xml: String,
    },
    MoveInto {
        selector: String,
        new_parent_selector: String,
    },
    InsertAfter {
        selector: String,
        xml: String,
    },
    InsertBefore {
        selector: String,
        xml: String,
    },
}

/// Removes `//` line comments outside strings; Minify's `xml.json` files use
/// them to switch actions off.
fn strip_line_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    let mut in_string = false;
    while let Some(character) = characters.next() {
        if in_string {
            out.push(character);
            if character == '\\' {
                if let Some(next) = characters.next() {
                    out.push(next);
                }
            } else if character == '"' {
                in_string = false;
            }
        } else if character == '/' && characters.peek() == Some(&'/') {
            for skipped in characters.by_ref() {
                if skipped == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else {
            if character == '"' {
                in_string = true;
            }
            out.push(character);
        }
    }
    out
}

/// The compiled game resource an `xml.json` key names.
fn compiled_path(path: &str) -> Result<String, String> {
    let stem = path
        .strip_prefix("panorama/layout/")
        .and_then(|rest| rest.strip_suffix(".xml"))
        .ok_or_else(invalid)?;
    if stem.is_empty()
        || !stem.split('/').all(|part| {
            !part.is_empty()
                && part != ".."
                && part.chars().all(|character| {
                    character.is_ascii_lowercase()
                        || character.is_ascii_digit()
                        || matches!(character, '_' | '-' | '.')
                })
        })
    {
        return Err(invalid());
    }
    Ok(format!("panorama/layout/{stem}.vxml_c"))
}

/// Parses an `xml.json` into compiled layout path -> actions. Files with no
/// actions are left out. Every selector and snippet is checked here, so a
/// malformed package fails before anything is built.
pub(crate) fn parse_edits(text: &str) -> Result<BTreeMap<String, Vec<Edit>>, String> {
    if text.len() > MAX_EDITS_BYTES {
        return Err(invalid());
    }
    let raw: BTreeMap<String, Vec<Edit>> =
        serde_json::from_str(&strip_line_comments(text)).map_err(|_| invalid())?;
    let mut edits = BTreeMap::new();
    for (path, actions) in raw {
        if actions.is_empty() {
            continue;
        }
        for action in &actions {
            match action {
                Edit::AddScript { src } | Edit::AddStyleInclude { src } => {
                    if !src.starts_with("s2r://") || src.len() > 512 {
                        return Err(invalid());
                    }
                }
                Edit::SetAttribute {
                    selector,
                    attribute,
                    ..
                } => {
                    Selector::parse(selector)?;
                    if attribute.is_empty()
                        || !attribute.chars().all(|character| {
                            character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                        })
                    {
                        return Err(invalid());
                    }
                }
                Edit::AddChild { selector, xml }
                | Edit::InsertAfter { selector, xml }
                | Edit::InsertBefore { selector, xml } => {
                    Selector::parse(selector)?;
                    parse_snippet(xml)?;
                }
                Edit::MoveInto {
                    selector,
                    new_parent_selector,
                } => {
                    Selector::parse(selector)?;
                    Selector::parse(new_parent_selector)?;
                }
            }
        }
        if edits.insert(compiled_path(&path)?, actions).is_some() {
            return Err(invalid());
        }
    }
    Ok(edits)
}

/// Minify's `ensure_unique_include`: adds `src` to the root's first
/// `scripts`/`styles` element, creating it as the root's first child.
fn ensure_include(root: &mut Node, container: &str, source: &str) {
    let kind = if container == "scripts" {
        "SCRIPTS"
    } else {
        "STYLES"
    };
    let position = anchor_position(root);
    let index = match root.children.iter().position(|child| child.kind == kind) {
        Some(index) => index,
        None => {
            let mut created = Node::new(kind, None);
            created.position = Some(position.clone());
            let first_element = root
                .children
                .iter()
                .position(|child| child.kind != "PANEL_ATTRIBUTE")
                .unwrap_or(root.children.len());
            root.children.insert(first_element, created);
            first_element
        }
    };
    let container = &mut root.children[index];
    if container.children.iter().any(|include| {
        include.kind == "INCLUDE" && include.attribute("src").as_deref() == Some(source)
    }) {
        return;
    }
    let position = container
        .children
        .last()
        .map(anchor_position)
        .unwrap_or_else(|| anchor_position(container));
    let mut include = Node::new("INCLUDE", None);
    include.children.push(value_node(source));
    include.position = Some(position);
    container.children.push(include);
}

fn set_attribute(node: &mut Node, key: &str, value: &str) -> Result<(), String> {
    if node.kind != "PANEL" {
        return Err("panorama_layout_unsupported".to_string());
    }
    if let Some(existing) = node
        .children
        .iter_mut()
        .find(|child| child.kind == "PANEL_ATTRIBUTE" && child.name.as_deref() == Some(key))
    {
        existing.children = vec![value_node(value)];
        return Ok(());
    }
    let after_attributes = node
        .children
        .iter()
        .position(|child| child.kind != "PANEL_ATTRIBUTE")
        .unwrap_or(node.children.len());
    node.children
        .insert(after_attributes, attribute_node(key, value));
    Ok(())
}

pub(crate) fn apply(root: &mut Node, edit: &Edit) -> Result<(), String> {
    match edit {
        Edit::AddScript { src } => ensure_include(root, "scripts", src),
        Edit::AddStyleInclude { src } => ensure_include(root, "styles", src),
        Edit::SetAttribute {
            selector,
            attribute,
            value,
        } => {
            let path = find(root, &Selector::parse(selector)?).ok_or_else(missing)?;
            set_attribute(node_at_mut(root, &path), attribute, value)?;
        }
        Edit::AddChild { selector, xml } => {
            let path = find(root, &Selector::parse(selector)?).ok_or_else(missing)?;
            let parent = node_at_mut(root, &path);
            let child = element_to_node(&parse_snippet(xml)?, &anchor_position(parent))?;
            parent.children.push(child);
        }
        Edit::InsertAfter { selector, xml } | Edit::InsertBefore { selector, xml } => {
            let path = find_with_parent(root, &Selector::parse(selector)?).ok_or_else(missing)?;
            let (index, parent_path) = path.split_last().ok_or_else(missing)?;
            let position = anchor_position(node_at(root, &path));
            let node = element_to_node(&parse_snippet(xml)?, &position)?;
            let at = if matches!(edit, Edit::InsertAfter { .. }) {
                index + 1
            } else {
                *index
            };
            node_at_mut(root, parent_path).children.insert(at, node);
        }
        Edit::MoveInto {
            selector,
            new_parent_selector,
        } => {
            let path = find_with_parent(root, &Selector::parse(selector)?).ok_or_else(missing)?;
            let mut target =
                find(root, &Selector::parse(new_parent_selector)?).ok_or_else(missing)?;
            let (index, old_parent) = path.split_last().ok_or_else(missing)?;
            if target.starts_with(&path) {
                return Err(invalid());
            }
            // Removing the element shifts later siblings under the same parent.
            if target.len() > old_parent.len()
                && target.starts_with(old_parent)
                && target[old_parent.len()] > *index
            {
                target[old_parent.len()] -= 1;
            }
            let moved = node_at_mut(root, old_parent).children.remove(*index);
            node_at_mut(root, &target).children.push(moved);
        }
    }
    if root.count() > MAX_NODES {
        return Err(invalid());
    }
    Ok(())
}

/// Adds a mod's settings `menu.xml` to the game settings the way Minify's
/// Auto Accept Match does: inside a "Minify" section appended to the settings
/// body. Several menus share one section.
pub(crate) fn apply_menus(root: &mut Node, menus: &[String]) -> Result<(), String> {
    if menus.is_empty() {
        return Ok(());
    }
    let body = all_elements(root)
        .into_iter()
        .skip(1)
        .find(|path| node_at(root, path).tag() == Some(MENU_PARENT_TAG))
        .ok_or_else(missing)?;
    let body = node_at_mut(root, &body);
    let position = anchor_position(body);
    let mut section = element_to_node(&parse_snippet(MENU_SECTION)?, &position)?;
    for menu in menus {
        if !menu.trim_start().starts_with("<Panel ") {
            return Err(invalid());
        }
        section
            .children
            .push(element_to_node(&parse_snippet(menu)?, &position)?);
    }
    body.children.push(section);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(xml: &str) -> Node {
        let element = parse_snippet(xml).expect("parses");
        assert_eq!(element.name, "root");
        let mut root = Node::new("ROOT", None);
        for (line, child) in element.children.iter().enumerate() {
            root.children.push(
                element_to_node(
                    child,
                    &Value::Array(vec![Value::Int(line as i64 + 2), Value::Int(3)]),
                )
                .expect("converts"),
            );
        }
        root
    }

    fn print(node: &Node) -> String {
        let Some(tag) = node.tag() else {
            return String::new();
        };
        let mut out = format!("<{tag}");
        if node.kind == "INCLUDE" {
            out += &format!(" src=\"{}\"", node.attribute("src").unwrap_or_default());
        }
        for attribute in node
            .children
            .iter()
            .filter(|child| child.kind == "PANEL_ATTRIBUTE")
        {
            out += &format!(
                " {}=\"{}\"",
                attribute.name.as_deref().unwrap_or_default(),
                attribute
                    .children
                    .first()
                    .and_then(value_text)
                    .unwrap_or_default()
            );
        }
        let children = node.children.iter().map(print).collect::<String>();
        if children.is_empty() {
            out + "/>"
        } else {
            format!("{out}>{children}</{tag}>")
        }
    }

    const PAGE: &str = r#"<root>
        <styles><include src="s2r://panorama/styles/dotastyles.vcss_c" /></styles>
        <Panel id="Page" class="Root Main">
            <Panel id="Header"><Label id="MatchID" text="1" /></Panel>
            <Panel id="SteamFriends" />
            <Label id="MatchID" text="second" />
        </Panel>
    </root>"#;

    #[test]
    fn strips_comments_but_not_urls_inside_strings() {
        let text = "{\n  \"a.xml\": [ // note\n  {\"action\": \"add_script\", \"src\": \"s2r://x.vjs_c\"}\n  // {\"action\": \"gone\"}\n  ]\n}";
        let stripped = strip_line_comments(text);
        assert!(stripped.contains("s2r://x.vjs_c"));
        assert!(!stripped.contains("note") && !stripped.contains("gone"));
    }

    #[test]
    fn parses_real_minify_edit_files() {
        let edits = parse_edits(include_str!(
            "../tests/fixtures/minify_stat_site_buttons_xml.json"
        ))
        .expect("parses");
        assert_eq!(
            edits.keys().cloned().collect::<Vec<_>>(),
            vec![
                "panorama/layout/dashboard_page_post_game.vxml_c",
                "panorama/layout/showcase/dashboard_page_showcase.vxml_c"
            ]
        );
        assert_eq!(
            edits["panorama/layout/dashboard_page_post_game.vxml_c"].len(),
            4
        );
        let auto = parse_edits(include_str!(
            "../tests/fixtures/minify_auto_accept_match_xml.json"
        ))
        .expect("parses");
        assert_eq!(auto.len(), 1, "the empty settings entry is dropped");
        let hud = parse_edits(include_str!(
            "../tests/fixtures/minify_repopulate_unit_query_hud_xml.json"
        ))
        .expect("parses");
        assert_eq!(
            hud["panorama/layout/hud/dota_hud_query_unit.vxml_c"].len(),
            10
        );
    }

    #[test]
    fn rejects_unknown_actions_fields_and_paths() {
        assert!(
            parse_edits(r#"{"panorama/layout/a.xml":[{"action":"remove","selector":"x"}]}"#)
                .is_err()
        );
        assert!(parse_edits(
            r#"{"panorama/layout/a.xml":[{"action":"add_script","src":"s2r://a","x":1}]}"#
        )
        .is_err());
        assert!(
            parse_edits(r#"{"panorama/../a.xml":[{"action":"add_script","src":"s2r://a"}]}"#)
                .is_err()
        );
        assert!(parse_edits(r##"{"panorama/layout/a.xml":[{"action":"add_child","selector":"#a","xml":"<Panel>text</Panel>"}]}"##).is_err());
    }

    #[test]
    fn insert_after_uses_minifys_parent_first_order() {
        let mut root = layout(PAGE);
        // The second #MatchID is a direct child of #Page, which is visited
        // as a parent before #Header, so Minify inserts after that one.
        apply(
            &mut root,
            &Edit::InsertAfter {
                selector: "#MatchID".into(),
                xml:
                    "<Button class='A' onactivate='go(&apos;x&apos;)'><Label text='DB' /></Button>"
                        .into(),
            },
        )
        .expect("applies");
        assert!(print(&root).contains(
            r#"<Label id="MatchID" text="second"/><Button class="A" onactivate="go('x')"><Label text="DB"/></Button>"#
        ));
        // set_attribute uses plain document order and reaches #Header's child.
        apply(
            &mut root,
            &Edit::SetAttribute {
                selector: "#MatchID".into(),
                attribute: "text".into(),
                value: "changed".into(),
            },
        )
        .expect("applies");
        assert!(print(&root)
            .contains(r#"<Panel id="Header"><Label id="MatchID" text="changed"/></Panel>"#));
    }

    #[test]
    fn scripts_and_styles_are_added_once() {
        let mut root = layout(PAGE);
        for _ in 0..2 {
            apply(
                &mut root,
                &Edit::AddScript {
                    src: "s2r://panorama/scripts/ssb.vjs_c".into(),
                },
            )
            .expect("applies");
            apply(
                &mut root,
                &Edit::AddStyleInclude {
                    src: "s2r://panorama/styles/extra.vcss_c".into(),
                },
            )
            .expect("applies");
        }
        let printed = print(&root);
        assert!(printed.starts_with(
            r#"<root><scripts><include src="s2r://panorama/scripts/ssb.vjs_c"/></scripts><styles><include src="s2r://panorama/styles/dotastyles.vcss_c"/><include src="s2r://panorama/styles/extra.vcss_c"/></styles>"#
        ));
        assert_eq!(printed.matches("ssb.vjs_c").count(), 1);
    }

    #[test]
    fn moves_add_children_and_report_missing_targets() {
        let mut root = layout(PAGE);
        apply(
            &mut root,
            &Edit::InsertBefore {
                selector: "#Header".into(),
                xml: "<Panel id='Left' />".into(),
            },
        )
        .expect("applies");
        apply(
            &mut root,
            &Edit::MoveInto {
                selector: "#SteamFriends".into(),
                new_parent_selector: "#Left".into(),
            },
        )
        .expect("applies");
        apply(
            &mut root,
            &Edit::AddChild {
                selector: "Panel.Main".into(),
                xml: "<Panel id='End' />".into(),
            },
        )
        .expect("applies");
        assert!(print(&root)
            .contains(r#"<Panel id="Left"><Panel id="SteamFriends"/></Panel><Panel id="Header">"#));
        assert!(print(&root).contains(r#"<Panel id="End"/></Panel></root>"#));
        assert_eq!(
            apply(
                &mut root,
                &Edit::AddChild {
                    selector: "#Nope".into(),
                    xml: "<Panel />".into()
                }
            ),
            Err(missing())
        );
        assert!(apply(
            &mut root,
            &Edit::MoveInto {
                selector: "#Left".into(),
                new_parent_selector: "#SteamFriends".into()
            }
        )
        .is_err());
    }

    #[test]
    fn move_into_a_later_sibling_keeps_the_right_target() {
        let mut root =
            layout("<root><Panel id='A'><Panel id='X' /><Panel id='B' /></Panel></root>");
        apply(
            &mut root,
            &Edit::MoveInto {
                selector: "#X".into(),
                new_parent_selector: "#B".into(),
            },
        )
        .expect("applies");
        assert_eq!(
            print(&root),
            r#"<root><Panel id="A"><Panel id="B"><Panel id="X"/></Panel></Panel></root>"#
        );
    }

    #[test]
    fn menus_share_one_section_in_the_settings_body() {
        let mut root = layout(
            "<root><Panel id='Popup'><PopupSettingsRebornSettingsBody id='Body' /></Panel></root>",
        );
        apply_menus(
            &mut root,
            &[include_str!("../tests/fixtures/minify_auto_accept_match_menu.xml").to_string()],
        )
        .expect("applies");
        let printed = print(&root);
        assert!(printed.contains(
            r##"<PopupSettingsRebornSettingsBody id="Body"><Panel class="SettingsSectionContainer" section="#minify""##
        ));
        assert!(printed.contains(r#"<Image class="SettingsSectionTitleIcon""#));
        assert!(printed.contains(r#"convar="dota_replay_manager_download_simultaneous_requests""#));
        assert!(apply_menus(&mut layout("<root />"), &["<Panel />".into()]).is_err());
    }

    #[test]
    fn empty_attribute_values_have_no_name_as_the_compiler_writes_them() {
        let mut root = layout("<root><Panel id='A' onactivate='' /></root>");
        let attribute = &root.children[0].children[1];
        assert_eq!(
            attribute.children[0],
            Node::new("PANEL_ATTRIBUTE_VALUE", None)
        );
        assert_eq!(
            root.children[0].attribute("onactivate").as_deref(),
            Some("")
        );
        apply(
            &mut root,
            &Edit::SetAttribute {
                selector: "#A".into(),
                attribute: "onload".into(),
                value: "Go()".into(),
            },
        )
        .expect("applies");
        assert_eq!(
            print(&root),
            r#"<root><Panel id="A" onactivate="" onload="Go()"/></root>"#
        );
    }

    #[test]
    fn tree_round_trips_through_kv3_values() {
        let root = layout(PAGE);
        let value = root.to_value();
        assert_eq!(Node::from_value(&value, 0, &mut 0).expect("reads"), root);
    }
}
