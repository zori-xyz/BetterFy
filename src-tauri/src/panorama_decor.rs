#![allow(dead_code)] // Wired into the archive pipeline in a follow-up; see docs.

//! Proofs for Panorama decor in third-party archives: main-menu backgrounds,
//! HUD skins and versus screens.
//!
//! The project's rule is "only audited scripts and layouts": nothing that can
//! run code or reach outside the game is installed. A hero skin only ever
//! brought one layout shape (an image wrapper). These categories bring whole
//! layouts, style sheets and videos, so each file type gets its own proof,
//! built so that anything the reader does not recognise is refused with the
//! exact reason, never accepted by default:
//!
//! * [`prove_static_layout`] reads the layout's parsed tree (never its name).
//!   Only a closed set of Valve panel types and attributes passes; every
//!   attribute value is plain text of a fixed shape; references name a file of
//!   the same archive or one of Valve's own; no script body, no snippet, no
//!   passthrough (`file://`) reference, no HTML or browser panel. Event
//!   handlers (`onactivate="..."`) are inline code in Panorama, so they pass
//!   only as an exact, pinned *binding* ([`pins`]): element type, id,
//!   attribute and code identical to one reviewed in a real archive. A `SCRIPTS`
//!   include passes only when it names a script Valve ships and the archive
//!   does not replace. A layout with neither handlers nor scripts is
//!   [`Tier::PureDecor`]; otherwise it is [`Tier::ReviewedValveBindings`],
//!   which a policy switch can turn off.
//! * [`prove_style_sheet`] reads the compiled CSS text. Only the visual
//!   properties seen in real archives pass, values are a fixed character set
//!   with a fixed set of functions, `url()` is a quoted `s2r://` texture of
//!   the archive or of Valve, `@import` is a style, nothing else.
//! * [`check_webm`] is a *container* check of a video: EBML header, `webm`
//!   doctype, a segment that is exactly as long as the file, a closed set of
//!   top-level elements, listed codecs. It does not decode a single frame and
//!   so it does not prove the video harmless to the game's decoder.
//! * [`prove_archive`] runs the three over a whole archive and resolves
//!   in-archive references, including loops.
//!
//! What no proof here covers: that a layout or style looks right or even
//! loads in the current game, that a Valve script or event named by a layout
//! behaves as the reviewer assumed, and that text shown to the player is
//! honest.

use crate::panorama;
use crate::panorama_layout::{self, Node};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod hostile;
mod pins;
#[cfg(test)]
mod tests;

pub(crate) const MAX_LAYOUT_BYTES: usize = 64 * 1024;
pub(crate) const MAX_STYLE_BYTES: usize = 512 * 1024;
/// One video. The largest in the 61 audited archives is 19.9 MiB.
pub(crate) const MAX_WEBM_BYTES: usize = 64 * 1024 * 1024;
const MAX_ELEMENTS: usize = 4_000;
const MAX_DEPTH: usize = 48;
const MAX_ATTRIBUTE_BYTES: usize = 128;
const MAX_REFERENCE_BYTES: usize = 200;
const MAX_INCLUDES: usize = 64;
const MAX_CSS_DECLARATIONS: usize = 30_000;
const MAX_CSS_SELECTOR_BYTES: usize = 4_096;
const MAX_CSS_VALUE_BYTES: usize = 1_024;
const MAX_CSS_DEPTH: usize = 3;
const MAX_HANDLER_BYTES: usize = 192;

/// What a proof is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Layout,
    Style,
    Video,
}

/// How much the proof had to take on trust.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Tier {
    /// No event handler, no script include: pure structure and style.
    PureDecor,
    /// Carries pinned event bindings and/or includes of Valve's own scripts.
    ReviewedValveBindings,
}

/// Why a file is not accepted. `code()` is stable and is what reports count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    Empty,
    Oversized,
    PathNotAllowed,
    /// The compiled container could not be read or has an unexpected shape.
    Container,
    /// A block the audited compilers never write for this file type.
    BlockNotAllowed(String),
    /// The `DATA` block of a tree layout carries text of its own.
    LayoutTextCopy,
    /// The tree is not `root > styles? scripts? panel`.
    TreeShape,
    NodeKind(String),
    Element(String),
    Attribute {
        element: String,
        attribute: String,
    },
    AttributeValue(String),
    DuplicateAttribute(String),
    TooDeep,
    TooManyElements,
    /// An event handler that is not an exact pinned binding (or pins are off).
    EventHandler {
        element: String,
        attribute: String,
    },
    /// A `SCRIPTS` include that is not a Valve script (or scripts are off).
    ScriptInclude(String),
    /// A `file://` reference or any other reference form.
    Reference(String),
    /// A reference to a file that is neither in the archive nor Valve's.
    ReferenceMissing(String),
    /// An in-archive dependency that was itself refused.
    DependencyRefused(String),
    DependencyCycle,
    CssSyntax(&'static str),
    CssProperty(String),
    CssAtRule(String),
    CssValue(String),
    CssUrl(String),
    CssFunction(String),
    CssTooBig,
    WebmContainer(&'static str),
    WebmCodec(String),
    WebmElement(String),
    /// A video that no accepted layout of the archive plays.
    VideoUnreferenced,
    /// The category does not accept this kind of file at this path.
    NotForCategory,
}

impl Refusal {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Empty => "decor_empty",
            Self::Oversized => "decor_oversized",
            Self::PathNotAllowed => "decor_path_not_allowed",
            Self::Container => "decor_container",
            Self::BlockNotAllowed(_) => "decor_block_not_allowed",
            Self::LayoutTextCopy => "decor_layout_text_copy",
            Self::TreeShape => "decor_tree_shape",
            Self::NodeKind(_) => "decor_node_kind",
            Self::Element(_) => "decor_element_not_allowed",
            Self::Attribute { .. } => "decor_attribute_not_allowed",
            Self::AttributeValue(_) => "decor_attribute_value",
            Self::DuplicateAttribute(_) => "decor_duplicate_attribute",
            Self::TooDeep => "decor_too_deep",
            Self::TooManyElements => "decor_too_many_elements",
            Self::EventHandler { .. } => "decor_event_handler",
            Self::ScriptInclude(_) => "decor_script_include",
            Self::Reference(_) => "decor_reference_form",
            Self::ReferenceMissing(_) => "decor_reference_missing",
            Self::DependencyRefused(_) => "decor_dependency_refused",
            Self::DependencyCycle => "decor_dependency_cycle",
            Self::CssSyntax(_) => "decor_css_syntax",
            Self::CssProperty(_) => "decor_css_property",
            Self::CssAtRule(_) => "decor_css_at_rule",
            Self::CssValue(_) => "decor_css_value",
            Self::CssUrl(_) => "decor_css_url",
            Self::CssFunction(_) => "decor_css_function",
            Self::CssTooBig => "decor_css_too_big",
            Self::WebmContainer(_) => "decor_webm_container",
            Self::WebmCodec(_) => "decor_webm_codec",
            Self::WebmElement(_) => "decor_webm_element",
            Self::VideoUnreferenced => "decor_video_unreferenced",
            Self::NotForCategory => "decor_not_for_category",
        }
    }

    /// The exact reason, with the offending name where there is one.
    pub(crate) fn detail(&self) -> String {
        match self {
            Self::BlockNotAllowed(name)
            | Self::NodeKind(name)
            | Self::Element(name)
            | Self::AttributeValue(name)
            | Self::DuplicateAttribute(name)
            | Self::ScriptInclude(name)
            | Self::Reference(name)
            | Self::ReferenceMissing(name)
            | Self::DependencyRefused(name)
            | Self::CssProperty(name)
            | Self::CssAtRule(name)
            | Self::CssValue(name)
            | Self::CssUrl(name)
            | Self::CssFunction(name)
            | Self::WebmCodec(name)
            | Self::WebmElement(name) => format!("{}: {name}", self.code()),
            Self::Attribute { element, attribute } | Self::EventHandler { element, attribute } => {
                format!("{}: {element}.{attribute}", self.code())
            }
            Self::CssSyntax(why) | Self::WebmContainer(why) => {
                format!("{}: {why}", self.code())
            }
            _ => self.code().to_string(),
        }
    }
}

/// Switches and the audited facts a proof is checked against.
#[derive(Clone, Copy)]
pub(crate) struct Context<'a> {
    /// Lower-case paths of Valve's own files (its pinned file list).
    pub valve: &'a BTreeSet<String>,
    /// Every lower-case path of the archive, whatever happens to it.
    pub archive: &'a BTreeSet<String>,
    /// Accept pinned event bindings and includes of Valve's own scripts. Off,
    /// only [`Tier::PureDecor`] layouts pass.
    pub allow_reviewed_valve_bindings: bool,
}

/// Numbers behind a proof, for the review and the audit report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Facts {
    pub elements: usize,
    pub handlers: usize,
    pub script_includes: usize,
    pub style_includes: usize,
    pub valve_references: usize,
    pub archive_references: usize,
    pub declarations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Proof {
    pub kind: Kind,
    pub tier: Tier,
    /// Files of the archive this one points at.
    pub dependencies: BTreeSet<String>,
    pub facts: Facts,
}

// ---------- shared checks ----------

fn is_lower_path_char(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.' | b'/')
}

/// A reference as a layout or style spells it, as the lower-case compiled
/// path it stands for (`x.vtex` is the file `x.vtex_c`).
fn compiled_reference(name: &str) -> Result<String, Refusal> {
    let reference = || Refusal::Reference(name.chars().take(80).collect());
    if name.is_empty()
        || name.len() > MAX_REFERENCE_BYTES
        || name.starts_with('/')
        || !name.bytes().all(is_lower_path_char)
        || name
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(reference());
    }
    let compiled = [".vtex", ".vcss", ".vts", ".vjs"]
        .iter()
        .any(|suffix| name.ends_with(suffix));
    Ok(if compiled {
        format!("{name}_c")
    } else {
        name.to_string()
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReferenceKind {
    Style,
    Script,
    Texture,
    Video,
}

fn reference_suffixes(kind: ReferenceKind) -> &'static [&'static str] {
    match kind {
        ReferenceKind::Style => &[".vcss_c"],
        ReferenceKind::Script => &[".vts_c", ".vjs_c"],
        ReferenceKind::Texture => &[".vtex_c"],
        ReferenceKind::Video => &[".webm"],
    }
}

// ---------- layouts ----------

/// Panel types seen in real background, versus-screen and dashboard layouts.
/// All are Valve's own. Anything else, in particular any panel that renders
/// web content or hosts a browser, is refused by omission.
const ELEMENTS: &[&str] = &[
    "Button",
    "DOTAAntiAddictionBadge",
    "DOTAChat",
    "DOTACoachingButton",
    "DOTACoachingMenu",
    "DOTAContextMenuManager",
    "DOTACustomLobby",
    "DOTADashboard",
    "DOTADashboardBackgroundLastMatch",
    "DOTADashboardBackgroundManager",
    "DOTADashboardPopupManager",
    "DOTAFriendMenu",
    "DOTAGuildImage",
    "DOTAGuildStatus",
    "DOTAHelpTipsManager",
    "DOTAHomePage",
    "DOTAHudVersusScene",
    "DOTAHudVersusSceneFeaturedHeroInfo",
    "DOTAHudVersusSceneTeamInfo",
    "DOTANotificationsMenu",
    "DOTAParty",
    "DOTAPartySearch",
    "DOTAPlay",
    "DOTAPlusButton",
    "DOTAPlusMenu",
    "DOTARecentArmoryItemsMenu",
    "DOTAScenePanel",
    "DOTAShardsButton",
    "DOTAShardsMenu",
    "DOTATooltipManager",
    "DOTAViolator",
    "Image",
    "Label",
    "MoviePanel",
    "PageManager",
    "Panel",
    "RadioButton",
    "TextButton",
    "TextEntry",
    "ToastManager",
    "ToggleButton",
];

#[derive(Clone, Copy)]
enum Value {
    /// Letters, digits, `_`, `-`, `.` and spaces (ids, classes, names).
    Token,
    Bool,
    /// A number, optionally with an `s` suffix (a duration).
    Number,
    /// A scene or map name: lower-case path characters.
    MapName,
    /// A localisation key or short display text.
    Text,
    /// A reference to a texture (`Image`) or a video (`MoviePanel`).
    Source,
}

/// Attributes seen in real layouts and the shape their value must have.
/// An attribute that starts with `on` is an event handler and is handled
/// apart; every attribute not listed is refused.
const ATTRIBUTES: &[(&str, Value)] = &[
    ("allow-background-models", Value::Bool),
    ("autoplay", Value::Token),
    ("camera", Value::Token),
    ("chatstyle", Value::Token),
    ("class", Value::Token),
    ("dashboardclass", Value::Token),
    ("deferredalpha", Value::Bool),
    ("group", Value::Token),
    ("hittest", Value::Bool),
    ("hittestchildren", Value::Bool),
    ("id", Value::Token),
    ("light", Value::Token),
    ("live-mode", Value::Token),
    ("map", Value::MapName),
    ("maxtoastsvisible", Value::Number),
    ("name", Value::Token),
    ("override-background", Value::Token),
    ("particleonly", Value::Bool),
    ("pin-fov", Value::Token),
    ("renderdeferred", Value::Bool),
    ("repeat", Value::Bool),
    ("require-composition-layer", Value::Bool),
    ("selected", Value::Bool),
    ("showBroadcastingParties", Value::Bool),
    ("showBroadcastingPartiesAsRegularParties", Value::Bool),
    ("showLobbies", Value::Bool),
    ("spawn-effigies", Value::Bool),
    ("src", Value::Source),
    ("text", Value::Text),
    ("toastduration", Value::Number),
    ("unlocalized", Value::Bool),
];

fn token_ok(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ATTRIBUTE_BYTES
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b' '))
}

fn text_ok(value: &str) -> bool {
    value.len() <= MAX_ATTRIBUTE_BYTES
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b' ' | b'_' | b'-' | b'.' | b'#' | b'{' | b'}' | b':' | b'(' | b')' | b'+'
                )
        })
}

fn number_ok(value: &str) -> bool {
    let digits = value.strip_suffix('s').unwrap_or(value);
    !digits.is_empty()
        && digits.len() <= 8
        && digits.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && digits.bytes().filter(|b| *b == b'.').count() <= 1
}

fn map_ok(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ATTRIBUTE_BYTES
        && value.bytes().all(|b| is_lower_path_char(b) && b != b'.')
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "..")
}

/// A handler is a list of calls `Name( arguments )` separated by `;`. This is
/// the last line of defence behind the pinned bindings (a pin that broke it
/// would fail a test): no `$` calls, member access, quotes other than simple
/// single-quoted names, operators, assignments or URLs.
fn handler_shape_ok(code: &str) -> bool {
    if code.is_empty()
        || code.len() > MAX_HANDLER_BYTES
        || !code.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b' ' | b'_' | b'(' | b')' | b',' | b';' | b'.' | b'\'' | b'#' | b'-'
                )
        })
    {
        return false;
    }
    let mut depth = 0i32;
    let bytes = code.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            b'.' => {
                // Only inside a number (`0.0`).
                let before = index.checked_sub(1).map(|i| bytes[i]);
                let after = bytes.get(index + 1);
                if !before.is_some_and(|b| b.is_ascii_digit())
                    || !after.is_some_and(|b| b.is_ascii_digit())
                {
                    return false;
                }
            }
            b'\'' => {}
            _ => {}
        }
    }
    if depth != 0 || !code.matches('\'').count().is_multiple_of(2) {
        return false;
    }
    // Words that name something leaving the game's own interface. The pinned
    // bindings contain none; this keeps it that way if a pin is ever added.
    let lowered = code.to_ascii_lowercase();
    ![
        "browser", "url", "http", "web", "console", "command", "exec", "eval", "msg", "dispatch",
        "file",
    ]
    .iter()
    .any(|word| lowered.contains(word))
}

struct LayoutWalk<'a> {
    ctx: &'a Context<'a>,
    facts: Facts,
    dependencies: BTreeSet<String>,
    nodes: usize,
}

impl LayoutWalk<'_> {
    /// Resolves a reference to a file of the archive (a dependency) or of
    /// Valve. The archive wins: a file it carries replaces Valve's.
    fn resolve(&mut self, name: &str, kind: ReferenceKind) -> Result<String, Refusal> {
        let path = compiled_reference(name)?;
        if !reference_suffixes(kind)
            .iter()
            .any(|suffix| path.ends_with(suffix))
        {
            return Err(Refusal::Reference(path));
        }
        if self.ctx.archive.contains(&path) {
            self.facts.archive_references += 1;
            self.dependencies.insert(path.clone());
        } else if self.ctx.valve.contains(&path) {
            self.facts.valve_references += 1;
        } else {
            return Err(Refusal::ReferenceMissing(path));
        }
        Ok(path)
    }

    fn includes(&mut self, group: &Node, kind: ReferenceKind) -> Result<(), Refusal> {
        if group.children().len() > MAX_INCLUDES || group.name().is_some() {
            return Err(Refusal::TreeShape);
        }
        for include in group.children() {
            let [value] = include.children() else {
                return Err(Refusal::TreeShape);
            };
            if include.kind() != "INCLUDE"
                || include.name().is_some()
                || value.kind() != "REFERENCE_COMPILED"
                || !value.children().is_empty()
            {
                return Err(Refusal::Reference(
                    include.kind().chars().take(40).collect(),
                ));
            }
            let name = value.name().ok_or(Refusal::TreeShape)?;
            if kind == ReferenceKind::Script {
                let path = compiled_reference(name)?;
                // A script is Valve's own, and the archive does not replace
                // it (scripts never install, but the check does not rely on
                // that).
                if !self.ctx.allow_reviewed_valve_bindings
                    || self.ctx.archive.contains(&path)
                    || !self.ctx.valve.contains(&path)
                    || !reference_suffixes(kind)
                        .iter()
                        .any(|suffix| path.ends_with(suffix))
                {
                    return Err(Refusal::ScriptInclude(path));
                }
                self.facts.script_includes += 1;
                self.facts.valve_references += 1;
            } else {
                self.resolve(name, kind)?;
                self.facts.style_includes += 1;
            }
        }
        Ok(())
    }

    fn attribute(&mut self, tag: &str, id: Option<&str>, node: &Node) -> Result<(), Refusal> {
        let name = node.name().ok_or(Refusal::TreeShape)?;
        let [value] = node.children() else {
            return Err(Refusal::TreeShape);
        };
        if !value.children().is_empty() {
            return Err(Refusal::TreeShape);
        }
        let attribute = || Refusal::Attribute {
            element: tag.to_string(),
            attribute: name.chars().take(40).collect(),
        };
        if name.len() > 48
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(attribute());
        }
        let text = |value: &Node| -> Result<String, Refusal> {
            match value.kind() {
                "PANEL_ATTRIBUTE_VALUE" => Ok(value.name().unwrap_or("").to_string()),
                other => Err(Refusal::NodeKind(other.chars().take(40).collect())),
            }
        };
        if name.len() >= 2 && name[..2].eq_ignore_ascii_case("on") {
            let code = text(value)?;
            let handler = || Refusal::EventHandler {
                element: tag.to_string(),
                attribute: name.to_string(),
            };
            if !self.ctx.allow_reviewed_valve_bindings
                || !handler_shape_ok(&code)
                || !pins::is_reviewed(tag, id.unwrap_or(""), name, &code)
            {
                return Err(handler());
            }
            self.facts.handlers += 1;
            return Ok(());
        }
        let (_, rule) = ATTRIBUTES
            .iter()
            .find(|(allowed, _)| *allowed == name)
            .ok_or_else(attribute)?;
        let bad = || Refusal::AttributeValue(name.to_string());
        match rule {
            Value::Source => {
                if value.kind() != "REFERENCE_COMPILED" {
                    return Err(bad());
                }
                let target = value.name().ok_or_else(bad)?;
                let kind = match tag {
                    "Image" => ReferenceKind::Texture,
                    "MoviePanel" => ReferenceKind::Video,
                    _ => return Err(attribute()),
                };
                self.resolve(target, kind)?;
            }
            other => {
                let text = text(value)?;
                let ok = match other {
                    Value::Token => token_ok(&text),
                    Value::Bool => text == "true" || text == "false",
                    Value::Number => number_ok(&text),
                    Value::MapName => map_ok(&text),
                    Value::Text => text_ok(&text),
                    Value::Source => false,
                };
                if !ok {
                    return Err(bad());
                }
            }
        }
        Ok(())
    }

    fn panel(&mut self, node: &Node, depth: usize) -> Result<(), Refusal> {
        self.nodes += 1;
        if depth > MAX_DEPTH {
            return Err(Refusal::TooDeep);
        }
        if self.nodes > MAX_ELEMENTS {
            return Err(Refusal::TooManyElements);
        }
        if node.kind() != "PANEL" {
            return Err(Refusal::NodeKind(node.kind().chars().take(40).collect()));
        }
        let tag = node.name().ok_or(Refusal::TreeShape)?;
        if !ELEMENTS.contains(&tag) {
            return Err(Refusal::Element(tag.chars().take(60).collect()));
        }
        self.facts.elements += 1;
        let mut seen = BTreeSet::new();
        let mut id: Option<&str> = None;
        // The id is needed to check a handler's binding, whatever the order.
        for child in node.children() {
            if child.kind() == "PANEL_ATTRIBUTE" && child.name() == Some("id") {
                id = child.children().first().and_then(Node::name);
            }
        }
        for child in node.children() {
            match child.kind() {
                "PANEL_ATTRIBUTE" => {
                    let name = child.name().ok_or(Refusal::TreeShape)?;
                    if !seen.insert(name) {
                        return Err(Refusal::DuplicateAttribute(name.chars().take(40).collect()));
                    }
                    self.attribute(tag, id, child)?;
                }
                "PANEL" => self.panel(child, depth + 1)?,
                other => return Err(Refusal::NodeKind(other.chars().take(40).collect())),
            }
        }
        Ok(())
    }
}

/// Proves that a compiled layout is static Panorama decor.
///
/// `path` is only used for the dependency of an image wrapper elsewhere and
/// for the report; the proof is of the parsed tree. Accepted shape:
/// `root > styles? scripts? panel`, with the panel tree restricted as in the
/// module documentation. The current compiler's container is the only one
/// read: `RED2`, `DATA` with no text, `LaCo`.
pub(crate) fn prove_static_layout(
    path: &str,
    bytes: &[u8],
    ctx: &Context<'_>,
) -> Result<Proof, Refusal> {
    if bytes.is_empty() {
        return Err(Refusal::Empty);
    }
    if bytes.len() > MAX_LAYOUT_BYTES {
        return Err(Refusal::Oversized);
    }
    if !path.ends_with(".vxml_c") {
        return Err(Refusal::PathNotAllowed);
    }
    let decoded = panorama_layout::decode(bytes).map_err(|_| Refusal::Container)?;
    if !decoded.tree_container {
        return Err(Refusal::Container);
    }
    for block in &decoded.resource.blocks {
        let tag = String::from_utf8_lossy(&block.tag).to_string();
        if !matches!(tag.as_str(), "RED2" | "DATA" | "LaCo") {
            return Err(Refusal::BlockNotAllowed(tag));
        }
    }
    let root = &decoded.root;
    let mut walk = LayoutWalk {
        ctx,
        facts: Facts::default(),
        dependencies: BTreeSet::new(),
        nodes: 0,
    };
    let mut children = root.children().iter().peekable();
    if let Some(styles) = children.next_if(|node| node.kind() == "STYLES") {
        walk.includes(styles, ReferenceKind::Style)?;
    }
    if let Some(scripts) = children.next_if(|node| node.kind() == "SCRIPTS") {
        walk.includes(scripts, ReferenceKind::Script)?;
    }
    let panel = children.next().ok_or(Refusal::TreeShape)?;
    if children.next().is_some() || root.name().is_some() {
        return Err(Refusal::TreeShape);
    }
    walk.panel(panel, 0)?;
    let tier = if walk.facts.handlers == 0 && walk.facts.script_includes == 0 {
        Tier::PureDecor
    } else {
        Tier::ReviewedValveBindings
    };
    Ok(Proof {
        kind: Kind::Layout,
        tier,
        dependencies: walk.dependencies,
        facts: walk.facts,
    })
}

// ---------- style sheets ----------

/// Properties seen in the 31 distinct compiled styles of the audited
/// backgrounds and HUD skins. All visual: size, colour, spacing, transforms,
/// transitions and animations, plus `sound`, which names an event Valve
/// defines.
const PROPERTIES: &[&str] = &[
    "-s2-mix-blend-mode",
    "animation-delay",
    "animation-direction",
    "animation-duration",
    "animation-fill-mode",
    "animation-iteration-count",
    "animation-name",
    "animation-timing-function",
    "background-color",
    "background-image",
    "background-img-opacity",
    "background-position",
    "background-repeat",
    "background-size",
    "blur",
    "border",
    "border-bottom",
    "border-bottom-color",
    "border-bottom-left-radius",
    "border-bottom-right-radius",
    "border-bottom-style",
    "border-bottom-width",
    "border-brush",
    "border-color",
    "border-left",
    "border-left-color",
    "border-radius",
    "border-right",
    "border-right-color",
    "border-style",
    "border-top",
    "border-top-color",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-top-width",
    "border-width",
    "box-shadow",
    "brightness",
    "clip",
    "color",
    "context-menu-body-position",
    "context-menu-position",
    "contrast",
    "flow-children",
    "font-family",
    "font-size",
    "font-weight",
    "height",
    "horizontal-align",
    "hue-rotation",
    "ignore-parent-flow",
    "img-shadow",
    "letter-spacing",
    "line-height",
    "margin",
    "margin-bottom",
    "margin-left",
    "margin-right",
    "margin-top",
    "max-height",
    "max-width",
    "min-height",
    "min-width",
    "opacity",
    "opacity-brush",
    "opacity-mask",
    "overflow",
    "padding",
    "padding-bottom",
    "padding-left",
    "padding-right",
    "padding-top",
    "position",
    "pre-transform-scale2d",
    "saturation",
    "sound",
    "text-align",
    "text-overflow",
    "text-shadow",
    "text-transform",
    "texture-sampling",
    "tooltip-body-position",
    "tooltip-position",
    "transform",
    "transition-delay",
    "transition-duration",
    "transition-property",
    "transition-timing-function",
    "ui-scale",
    "vertical-align",
    "visibility",
    "wash-color",
    "white-space",
    "width",
    "x",
    "y",
    "z-index",
];

/// Functions seen in real values. `url` is checked on its own.
const FUNCTIONS: &[&str] = &[
    "ScaleX",
    "color-stop",
    "cubic-bezier",
    "fill-parent-flow",
    "from",
    "gaussian",
    "gradient",
    "radial",
    "rgb",
    "rgba",
    "rotateZ",
    "scale3d",
    "scaleX",
    "scaleY",
    "to",
    "translateX",
    "translateY",
    "translatex",
    "url",
    "width-percentage",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    Rule,
    Keyframes,
    Frame,
}

struct CssWalk<'a> {
    ctx: &'a Context<'a>,
    facts: Facts,
    dependencies: BTreeSet<String>,
}

fn css_selector_ok(selector: &str) -> bool {
    let trimmed = selector.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_CSS_SELECTOR_BYTES {
        return false;
    }
    let mut depth = 0i32;
    for byte in trimmed.bytes() {
        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b if b.is_ascii_alphanumeric()
                || matches!(b, b'_' | b'-' | b'#' | b'.' | b',' | b':' | b'>' | b'%')
                || b.is_ascii_whitespace() => {}
            _ => return false,
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

fn css_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 96
        && name.as_bytes()[0].is_ascii_alphabetic()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

impl CssWalk<'_> {
    fn url(&mut self, path: &str, import: bool) -> Result<(), Refusal> {
        let bad = || Refusal::CssUrl(path.chars().take(80).collect());
        let name = path.strip_prefix("s2r://").ok_or_else(bad)?;
        let kind = if import {
            ReferenceKind::Style
        } else {
            ReferenceKind::Texture
        };
        let wanted: &[&str] = if import {
            &[".vcss", ".vcss_c"]
        } else {
            &[".vtex"]
        };
        if !wanted.iter().any(|suffix| name.ends_with(suffix)) {
            return Err(bad());
        }
        let compiled = compiled_reference(name).map_err(|_| bad())?;
        if !reference_suffixes(kind)
            .iter()
            .any(|suffix| compiled.ends_with(suffix))
        {
            return Err(bad());
        }
        if self.ctx.archive.contains(&compiled) {
            self.facts.archive_references += 1;
            self.dependencies.insert(compiled);
        } else if self.ctx.valve.contains(&compiled) {
            self.facts.valve_references += 1;
        } else {
            return Err(Refusal::ReferenceMissing(compiled));
        }
        Ok(())
    }

    /// A property value or `@define` value.
    fn value(&mut self, property: &str, value: &str) -> Result<(), Refusal> {
        let bad = || {
            Refusal::CssValue(format!(
                "{property}: {}",
                value.trim().chars().take(70).collect::<String>()
            ))
        };
        let value = value.trim();
        if value.is_empty() || value.len() > MAX_CSS_VALUE_BYTES {
            return Err(bad());
        }
        // A whole quoted string: a sound event or a font name.
        if let Some(inner) = value.strip_prefix('"') {
            let inner = inner.strip_suffix('"').ok_or_else(bad)?;
            if inner.is_empty()
                || !inner
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b' '))
            {
                return Err(bad());
            }
            return Ok(());
        }
        let bytes = value.as_bytes();
        let mut index = 0usize;
        let mut depth = 0i32;
        while index < bytes.len() {
            let byte = bytes[index];
            if byte.is_ascii_alphabetic() {
                let start = index;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'-' | b'_'))
                {
                    index += 1;
                }
                let word = &value[start..index];
                let mut look = index;
                while look < bytes.len() && bytes[look] == b' ' {
                    look += 1;
                }
                if bytes.get(look) == Some(&b'(') {
                    if !FUNCTIONS.contains(&word) {
                        return Err(Refusal::CssFunction(word.chars().take(40).collect()));
                    }
                    if word == "url" {
                        // url( "s2r://..." ) and nothing else.
                        let mut at = look + 1;
                        while bytes.get(at) == Some(&b' ') {
                            at += 1;
                        }
                        if bytes.get(at) != Some(&b'"') {
                            return Err(Refusal::CssUrl(value.chars().take(80).collect()));
                        }
                        let begin = at + 1;
                        let end = value[begin..]
                            .find('"')
                            .map(|offset| begin + offset)
                            .ok_or_else(|| Refusal::CssUrl(value.chars().take(80).collect()))?;
                        let mut after = end + 1;
                        while bytes.get(after) == Some(&b' ') {
                            after += 1;
                        }
                        if bytes.get(after) != Some(&b')') {
                            return Err(Refusal::CssUrl(value.chars().take(80).collect()));
                        }
                        self.url(&value[begin..end], false)?;
                        index = after + 1;
                        continue;
                    }
                }
                continue;
            }
            if byte == b'\'' {
                // A simple single-quoted name (an animation name).
                let end = value[index + 1..]
                    .find('\'')
                    .map(|offset| index + 1 + offset)
                    .ok_or_else(bad)?;
                if !css_name_ok(&value[index + 1..end]) {
                    return Err(bad());
                }
                index = end + 1;
                continue;
            }
            match byte {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(bad());
                    }
                }
                b if b.is_ascii_digit()
                    || b.is_ascii_whitespace()
                    || matches!(b, b'#' | b'%' | b',' | b'-' | b'.' | b'_') => {}
                _ => return Err(bad()),
            }
            index += 1;
        }
        if depth != 0 {
            return Err(bad());
        }
        Ok(())
    }

    fn declaration(&mut self, text: &str) -> Result<(), Refusal> {
        let (property, value) = text
            .split_once(':')
            .ok_or(Refusal::CssSyntax("declaration without a colon"))?;
        let property = property.trim();
        if !PROPERTIES.contains(&property) {
            return Err(Refusal::CssProperty(property.chars().take(60).collect()));
        }
        self.facts.declarations += 1;
        if self.facts.declarations > MAX_CSS_DECLARATIONS {
            return Err(Refusal::CssTooBig);
        }
        self.value(property, value)
    }

    fn statement(&mut self, text: &str) -> Result<(), Refusal> {
        let (head, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
        match head {
            "@import" => {
                let rest = rest.trim();
                let inner = rest
                    .strip_prefix("url(")
                    .and_then(|r| r.strip_suffix(')'))
                    .map(str::trim)
                    .and_then(|r| r.strip_prefix('"'))
                    .and_then(|r| r.strip_suffix('"'))
                    .ok_or_else(|| Refusal::CssUrl(rest.chars().take(80).collect()))?;
                self.url(inner, true)
            }
            "@define" => {
                let (name, value) = rest
                    .split_once(':')
                    .ok_or(Refusal::CssSyntax("define without a colon"))?;
                if !css_name_ok(name.trim()) {
                    return Err(Refusal::CssSyntax("define name"));
                }
                self.value(name.trim(), value)
            }
            other => Err(Refusal::CssAtRule(other.chars().take(40).collect())),
        }
    }

    fn open(&mut self, prelude: &str, stack: &mut Vec<Block>) -> Result<(), Refusal> {
        let prelude = prelude.trim();
        if stack.len() >= MAX_CSS_DEPTH {
            return Err(Refusal::CssSyntax("nesting"));
        }
        match stack.last() {
            None => {
                if let Some(rest) = prelude.strip_prefix('@') {
                    let (head, name) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
                    let name = name.trim();
                    let name = name
                        .strip_prefix('\'')
                        .and_then(|n| n.strip_suffix('\''))
                        .unwrap_or(name);
                    if head != "keyframes" || !css_name_ok(name) {
                        return Err(Refusal::CssAtRule(format!("@{head}")));
                    }
                    stack.push(Block::Keyframes);
                } else if css_selector_ok(prelude) {
                    stack.push(Block::Rule);
                } else {
                    return Err(Refusal::CssSyntax("selector"));
                }
            }
            Some(Block::Keyframes) => {
                let frames_ok = !prelude.is_empty()
                    && prelude.split(',').all(|frame| {
                        let frame = frame.trim();
                        frame == "from"
                            || frame == "to"
                            || frame.strip_suffix('%').is_some_and(|n| {
                                !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                            })
                    });
                if !frames_ok {
                    return Err(Refusal::CssSyntax("keyframe selector"));
                }
                stack.push(Block::Frame);
            }
            Some(_) => return Err(Refusal::CssSyntax("nested rule")),
        }
        Ok(())
    }
}

/// Proves that a compiled style (`vcss_c`) is only visual CSS.
///
/// The container must be `RED2`, `DATA` and the source map `SrMa` with an
/// empty image table (the compiler writes images used by `url()` into that
/// table; the audited styles have none, because every `url()` names a Valve
/// texture by path). The text is then read with a small grammar: rules,
/// `@keyframes`, `@define` and `@import`, declarations of the listed
/// properties, values of a fixed character set and function list.
pub(crate) fn prove_style_sheet(
    path: &str,
    bytes: &[u8],
    ctx: &Context<'_>,
) -> Result<Proof, Refusal> {
    if bytes.is_empty() {
        return Err(Refusal::Empty);
    }
    if bytes.len() > MAX_STYLE_BYTES {
        return Err(Refusal::Oversized);
    }
    if !path.ends_with(".vcss_c") {
        return Err(Refusal::PathNotAllowed);
    }
    let resource = panorama::parse(bytes).map_err(|_| Refusal::Container)?;
    let mut data = None;
    for block in &resource.blocks {
        match &block.tag {
            b"DATA" if data.is_none() => data = Some(&block.bytes),
            b"RED2" | b"SrMa" => {}
            other => {
                return Err(Refusal::BlockNotAllowed(
                    String::from_utf8_lossy(other).to_string(),
                ))
            }
        }
    }
    let data = data.ok_or(Refusal::Container)?;
    if data.get(4..6) != Some(&[0, 0]) {
        return Err(Refusal::Container);
    }
    let text = panorama::data_text(&resource).map_err(|_| Refusal::Container)?;
    if !text
        .bytes()
        .all(|b| b.is_ascii_graphic() || matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
    {
        return Err(Refusal::CssSyntax("non-ASCII or control character"));
    }
    let mut walk = CssWalk {
        ctx,
        facts: Facts::default(),
        dependencies: BTreeSet::new(),
    };
    // No comments. None of the 215 audited style files has one, and a comment
    // is where a reader that parses differently from the game's hides text.
    if text.contains("/*") {
        return Err(Refusal::CssSyntax("comment"));
    }
    let clean = text;

    let mut stack: Vec<Block> = Vec::new();
    let mut buffer = String::new();
    let mut chars = clean.chars();
    while let Some(character) = chars.next() {
        match character {
            '"' => {
                buffer.push('"');
                loop {
                    match chars.next() {
                        Some('"') => {
                            buffer.push('"');
                            break;
                        }
                        Some(other) => buffer.push(other),
                        None => return Err(Refusal::CssSyntax("unterminated string")),
                    }
                }
            }
            '{' => {
                walk.open(&buffer, &mut stack)?;
                buffer.clear();
            }
            '}' => {
                if !buffer.trim().is_empty() {
                    match stack.last() {
                        Some(Block::Rule | Block::Frame) => walk.declaration(buffer.trim())?,
                        _ => return Err(Refusal::CssSyntax("loose text before }")),
                    }
                }
                buffer.clear();
                stack.pop().ok_or(Refusal::CssSyntax("unbalanced }"))?;
            }
            ';' => {
                let statement = buffer.trim();
                match stack.last() {
                    None => walk.statement(statement)?,
                    Some(Block::Rule | Block::Frame) => {
                        if !statement.is_empty() {
                            walk.declaration(statement)?;
                        }
                    }
                    Some(Block::Keyframes) => {
                        return Err(Refusal::CssSyntax("statement in keyframes"))
                    }
                }
                buffer.clear();
            }
            other => buffer.push(other),
        }
    }
    if !stack.is_empty() || !buffer.trim().is_empty() {
        return Err(Refusal::CssSyntax("unterminated block"));
    }
    Ok(Proof {
        kind: Kind::Style,
        tier: Tier::PureDecor,
        dependencies: walk.dependencies,
        facts: walk.facts,
    })
}

// ---------- WebM ----------

const EBML_HEADER: u32 = 0x1A45_DFA3;
const SEGMENT: u32 = 0x1853_8067;
const MAX_WEBM_ELEMENTS: usize = 1_000_000;

/// An EBML variable-length integer: the ID with its length marker kept, or a
/// size with it removed. `None` for a malformed one or an unknown size.
fn vint(bytes: &[u8], at: usize, keep_marker: bool) -> Option<(u64, usize)> {
    let first = *bytes.get(at)?;
    let length = first.leading_zeros() as usize + 1;
    if length > 8 {
        return None;
    }
    let slice = bytes.get(at..at + length)?;
    let mut value = if keep_marker {
        u64::from(first)
    } else {
        u64::from(first) & (0xFFu64 >> length)
    };
    for byte in &slice[1..] {
        value = (value << 8) | u64::from(*byte);
    }
    if !keep_marker && value == (1u64 << (7 * length)) - 1 {
        // Reserved "unknown size": a live stream, not a file.
        return None;
    }
    Some((value, length))
}

fn element(bytes: &[u8], at: usize) -> Result<(u32, usize, usize), Refusal> {
    let (id, id_len) = vint(bytes, at, true).ok_or(Refusal::WebmContainer("element id"))?;
    if id_len > 4 {
        return Err(Refusal::WebmContainer("element id length"));
    }
    let (size, size_len) =
        vint(bytes, at + id_len, false).ok_or(Refusal::WebmContainer("element size"))?;
    let start = at + id_len + size_len;
    let end = usize::try_from(size)
        .ok()
        .and_then(|size| start.checked_add(size))
        .filter(|end| *end <= bytes.len())
        .ok_or(Refusal::WebmContainer("element runs past the file"))?;
    Ok((id as u32, start, end))
}

fn id_name(id: u32) -> String {
    format!("0x{id:X}")
}

/// What a container check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VideoFacts {
    pub clusters: usize,
    pub video_codec: String,
    pub audio_codec: Option<String>,
}

/// A structural check of a WebM file. It establishes that the bytes are an
/// EBML document of doctype `webm` whose one segment spans the rest of the
/// file exactly, that every top-level element is one of the closed set a
/// player needs (no attachments, chapters or unknown elements), that the
/// element sizes chain without a gap or overrun, and that the tracks name a
/// VP8, VP9 or AV1 video and at most Vorbis or Opus audio. It does **not**
/// decode a frame: it cannot show that the stream is well formed inside a
/// cluster or that the game's decoder handles it safely.
pub(crate) fn check_webm(bytes: &[u8]) -> Result<VideoFacts, Refusal> {
    if bytes.is_empty() {
        return Err(Refusal::Empty);
    }
    if bytes.len() > MAX_WEBM_BYTES {
        return Err(Refusal::Oversized);
    }
    let (id, start, end) = element(bytes, 0)?;
    if id != EBML_HEADER || end - start > 256 {
        return Err(Refusal::WebmContainer("no EBML header"));
    }
    let mut doctype = None;
    let mut at = start;
    while at < end {
        let (child, child_start, child_end) = element(&bytes[..end], at)?;
        match child {
            0x4286 | 0x42F7 | 0x42F2 | 0x42F3 | 0x4287 | 0x4285 => {}
            0x4282 => doctype = Some(&bytes[child_start..child_end]),
            other => return Err(Refusal::WebmElement(id_name(other))),
        }
        at = child_end;
    }
    if doctype != Some(&b"webm"[..]) {
        return Err(Refusal::WebmContainer("doctype is not webm"));
    }
    let (id, segment_start, segment_end) = element(bytes, end)?;
    if id != SEGMENT || segment_end != bytes.len() {
        return Err(Refusal::WebmContainer(
            "segment is not exactly the rest of the file",
        ));
    }
    let (mut clusters, mut infos, mut tracks) = (0usize, 0usize, 0usize);
    let mut video_codec = None;
    let mut audio_codec = None;
    let mut at = segment_start;
    let mut count = 0usize;
    while at < segment_end {
        count += 1;
        if count > MAX_WEBM_ELEMENTS {
            return Err(Refusal::WebmContainer("too many elements"));
        }
        let (child, child_start, child_end) = element(&bytes[..segment_end], at)?;
        match child {
            // SeekHead, Cues, Tags, Void, CRC-32.
            0x114D_9B74 | 0x1C53_BB6B | 0x1254_C367 | 0xEC | 0xBF => {}
            0x1549_A966 => infos += 1,
            0x1F43_B675 => clusters += 1,
            0x1654_AE6B => {
                tracks += 1;
                let mut entry_at = child_start;
                let mut entries = 0usize;
                while entry_at < child_end {
                    let (entry, entry_start, entry_end) = element(&bytes[..child_end], entry_at)?;
                    if entry != 0xAE {
                        return Err(Refusal::WebmElement(id_name(entry)));
                    }
                    entries += 1;
                    if entries > 8 {
                        return Err(Refusal::WebmContainer("too many tracks"));
                    }
                    let (mut codec, mut kind) = (None, None);
                    let mut field_at = entry_start;
                    while field_at < entry_end {
                        let (field, field_start, field_end) =
                            element(&bytes[..entry_end], field_at)?;
                        match field {
                            0x86 => codec = Some(&bytes[field_start..field_end]),
                            0x83 if field_end - field_start == 1 => kind = Some(bytes[field_start]),
                            _ => {}
                        }
                        field_at = field_end;
                    }
                    let codec = String::from_utf8_lossy(
                        codec.ok_or(Refusal::WebmContainer("track without codec"))?,
                    )
                    .to_string();
                    match (kind, codec.as_str()) {
                        (Some(1), "V_VP8" | "V_VP9" | "V_AV1") if video_codec.is_none() => {
                            video_codec = Some(codec);
                        }
                        (Some(2), "A_VORBIS" | "A_OPUS") if audio_codec.is_none() => {
                            audio_codec = Some(codec);
                        }
                        _ => return Err(Refusal::WebmCodec(codec.chars().take(40).collect())),
                    }
                    entry_at = entry_end;
                }
            }
            other => return Err(Refusal::WebmElement(id_name(other))),
        }
        at = child_end;
    }
    if infos != 1 || tracks != 1 || clusters == 0 {
        return Err(Refusal::WebmContainer("missing info, tracks or clusters"));
    }
    Ok(VideoFacts {
        clusters,
        video_codec: video_codec.ok_or(Refusal::WebmContainer("no video track"))?,
        audio_codec,
    })
}

fn prove_video(path: &str, bytes: &[u8], valve: &BTreeSet<String>) -> Result<Proof, Refusal> {
    // A video of the author's own folder; never one that replaces a Valve file.
    if !video_path_ok(path) || valve.contains(path) {
        return Err(Refusal::PathNotAllowed);
    }
    check_webm(bytes)?;
    Ok(Proof {
        kind: Kind::Video,
        tier: Tier::PureDecor,
        dependencies: BTreeSet::new(),
        facts: Facts::default(),
    })
}

/// `folder/name.webm`: one folder of the author's own, lower-case.
fn video_path_ok(path: &str) -> bool {
    let Some(stem) = path.strip_suffix(".webm") else {
        return false;
    };
    let mut parts = stem.split('/');
    let (Some(folder), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let plain = |part: &str| {
        !part.is_empty()
            && part.len() <= 32
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
    };
    plain(folder) && plain(name) && folder != "panorama"
}

// ---------- categories and archives ----------

/// The Panorama categories of the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    Backgrounds,
    Huds,
    VersusScreens,
}

impl Category {
    /// Whether the category may carry a file of this kind at this path. The
    /// layouts and styles a category replaces are the ones its purpose needs.
    pub(crate) fn allows(self, kind: Kind, path: &str) -> bool {
        let name = path.rsplit('/').next().unwrap_or(path);
        match (self, kind) {
            (Self::Backgrounds, Kind::Layout) => {
                path.matches('/').count() == 2
                    && path.starts_with("panorama/layout/")
                    && (name == "dashboard.vxml_c"
                        || name == "dashboard_page_home.vxml_c"
                        || name == "hero_loadout_background_images.vxml_c"
                        || (name.starts_with("dashboard_background_") && name.ends_with(".vxml_c")))
            }
            (Self::Backgrounds, Kind::Style) => {
                path.matches('/').count() == 2
                    && path.starts_with("panorama/styles/")
                    && (name.starts_with("dashboard")
                        || name == "hero_loadout_background_images.vcss_c")
            }
            (Self::Backgrounds, Kind::Video) => true,
            // The HUD's own styles: the `hud/` folder, `dota_hud_*` files and
            // the stat branch (all the paths seen in the audited skins).
            (Self::Huds, Kind::Style) => {
                path.starts_with("panorama/styles/hud/")
                    || (path.matches('/').count() == 2
                        && path.starts_with("panorama/styles/")
                        && (name.starts_with("dota_hud_") || name == "ui_stat_branch.vcss_c"))
            }
            (Self::Huds, _) => false,
            (Self::VersusScreens, Kind::Layout) => path.starts_with("panorama/layout/hud/versus/"),
            (Self::VersusScreens, Kind::Style) => path.starts_with("panorama/styles/hud/versus/"),
            (Self::VersusScreens, Kind::Video) => false,
        }
    }
}

/// The kind of decor file a path is, by extension, or `None`.
pub(crate) fn kind_of(path: &str) -> Option<Kind> {
    if path.ends_with(".vxml_c") {
        Some(Kind::Layout)
    } else if path.ends_with(".vcss_c") {
        Some(Kind::Style)
    } else if path.ends_with(".webm") {
        Some(Kind::Video)
    } else {
        None
    }
}

/// Proves every layout, style and video of an archive for a category, then
/// resolves what they point at inside the archive: a file whose in-archive
/// dependency was refused is refused, a loop is refused, and a video that no
/// accepted layout plays is refused. `entries` are lower-case path and bytes.
/// Files of other types are not touched.
pub(crate) fn prove_archive(
    category: Category,
    entries: &[(String, &[u8])],
    valve: &BTreeSet<String>,
    allow_reviewed_valve_bindings: bool,
) -> BTreeMap<String, Result<Proof, Refusal>> {
    let archive = entries
        .iter()
        .map(|(path, _)| path.clone())
        .collect::<BTreeSet<_>>();
    let ctx = Context {
        valve,
        archive: &archive,
        allow_reviewed_valve_bindings,
    };
    let mut verdicts: BTreeMap<String, Result<Proof, Refusal>> = BTreeMap::new();
    for (path, bytes) in entries {
        let Some(kind) = kind_of(path) else {
            continue;
        };
        let verdict = if !category.allows(kind, path) {
            Err(Refusal::NotForCategory)
        } else {
            match kind {
                Kind::Layout => prove_static_layout(path, bytes, &ctx),
                Kind::Style => prove_style_sheet(path, bytes, &ctx),
                Kind::Video => prove_video(path, bytes, valve),
            }
        };
        verdicts.insert(path.clone(), verdict);
    }
    // Dependencies on other decor files must be accepted. The accepted set is
    // the least fixed point, so a cycle never becomes accepted.
    let decor = |path: &str| kind_of(path).is_some();
    let mut accepted: BTreeSet<String> = BTreeSet::new();
    loop {
        let before = accepted.len();
        for (path, verdict) in &verdicts {
            if accepted.contains(path) {
                continue;
            }
            if let Ok(proof) = verdict {
                if proof
                    .dependencies
                    .iter()
                    .filter(|dependency| decor(dependency))
                    .all(|dependency| accepted.contains(dependency))
                {
                    accepted.insert(path.clone());
                }
            }
        }
        if accepted.len() == before {
            break;
        }
    }
    // The rest passed on its own. It is refused because a dependency is
    // refused (directly or through another), and otherwise it is part of a
    // loop.
    let pending = verdicts
        .iter()
        .filter(|(path, verdict)| verdict.is_ok() && !accepted.contains(*path))
        .map(|(path, _)| path.clone())
        .collect::<BTreeSet<_>>();
    let mut dead: BTreeMap<String, String> = BTreeMap::new();
    loop {
        let mut changed = false;
        for path in &pending {
            if dead.contains_key(path) {
                continue;
            }
            let Some(Ok(proof)) = verdicts.get(path) else {
                continue;
            };
            let culprit = proof
                .dependencies
                .iter()
                .filter(|dependency| decor(dependency) && !accepted.contains(*dependency))
                .find(|dependency| !pending.contains(*dependency) || dead.contains_key(*dependency))
                .cloned();
            if let Some(culprit) = culprit {
                dead.insert(path.clone(), culprit);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for path in &pending {
        let refusal = match dead.get(path) {
            Some(dependency) => Refusal::DependencyRefused(dependency.clone()),
            None => Refusal::DependencyCycle,
        };
        verdicts.insert(path.clone(), Err(refusal));
    }
    // Videos nobody plays.
    let played = verdicts
        .values()
        .filter_map(|verdict| verdict.as_ref().ok())
        .filter(|proof| proof.kind == Kind::Layout)
        .flat_map(|proof| proof.dependencies.iter().cloned())
        .collect::<BTreeSet<_>>();
    for (path, verdict) in verdicts.iter_mut() {
        if matches!(verdict, Ok(proof) if proof.kind == Kind::Video) && !played.contains(path) {
            *verdict = Err(Refusal::VideoUnreferenced);
        }
    }
    verdicts
}
