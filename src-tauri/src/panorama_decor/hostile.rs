//! Synthetic layouts, styles and videos, including hostile ones.

use super::*;
use crate::panorama::Block as ResourceBlock;
use crate::panorama_layout::{
    test_element as el, test_includes, test_layout_bytes, test_node, test_reference_attribute,
};

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn valve() -> BTreeSet<String> {
    set(&[
        "panorama/styles/dotastyles.vcss_c",
        "panorama/styles/dashboard.vcss_c",
        "panorama/scripts/util.vts_c",
        "panorama/scripts/hud/versus/shared.vts_c",
        "panorama/images/masks/bg_vignette_psd.vtex_c",
        "panorama/images/hud/a_png.vtex_c",
    ])
}

fn context<'a>(
    valve: &'a BTreeSet<String>,
    archive: &'a BTreeSet<String>,
    bindings: bool,
) -> Context<'a> {
    Context {
        valve,
        archive,
        allow_reviewed_valve_bindings: bindings,
    }
}

fn root(children: Vec<Node>) -> Node {
    test_node("ROOT", None, children)
}

fn styles() -> Node {
    test_includes("STYLES", &["s2r://panorama/styles/dotastyles.vcss_c"])
}

fn layout(children: Vec<Node>) -> Vec<u8> {
    test_layout_bytes(root(children), Vec::new(), None)
}

fn prove_layout(bytes: &[u8], bindings: bool) -> Result<Proof, Refusal> {
    let valve = valve();
    let archive = set(&["zxc/zxc.webm", "panorama/styles/mine.vcss_c"]);
    prove_static_layout(
        "panorama/layout/dashboard_background_last_match.vxml_c",
        bytes,
        &context(&valve, &archive, bindings),
    )
}

fn good_panel() -> Node {
    el(
        "DOTADashboardBackgroundLastMatch",
        &[],
        vec![
            el("Panel", &[("id", "BackgroundImage")], vec![]),
            el(
                "MoviePanel",
                &[
                    ("id", "MovieBackground"),
                    ("repeat", "true"),
                    ("autoplay", "onload"),
                ],
                vec![test_reference_attribute("src", "s2r://zxc/zxc.webm")],
            ),
            el(
                "DOTAScenePanel",
                &[
                    ("id", "BackgroundMap"),
                    ("map", "backgrounds/dashboard_parallax_test"),
                ],
                vec![],
            ),
        ],
    )
}

#[test]
fn a_plain_background_layout_is_pure_decor() {
    let proof = prove_layout(&layout(vec![styles(), good_panel()]), true).expect("proven");
    assert_eq!(proof.kind, Kind::Layout);
    assert_eq!(proof.tier, Tier::PureDecor);
    assert_eq!(proof.facts.handlers, 0);
    assert_eq!(
        proof.dependencies,
        set(&["zxc/zxc.webm"]),
        "the video is a dependency of the archive"
    );
    // Pure decor does not depend on the switch.
    assert!(prove_layout(&layout(vec![styles(), good_panel()]), false).is_ok());
}

#[test]
fn panel_types_and_attributes_outside_the_lists_are_refused_by_name() {
    for (tag, attributes) in [
        ("HTML", vec![("url", "http://evil.example")]),
        ("DOTAHTMLPanel", vec![]),
        ("Carousel", vec![]),
        ("Panel", vec![("url", "http://evil.example")]),
        ("Panel", vec![("layout", "file://{resources}/layout/x.xml")]),
        ("Panel", vec![("href", "x")]),
        ("Panel", vec![("style", "background-image: url(x)")]),
        ("Panel", vec![("acceptsfocus", "true")]),
    ] {
        let verdict = prove_layout(
            &layout(vec![el("Panel", &[], vec![el(tag, &attributes, vec![])])]),
            true,
        );
        assert!(
            matches!(
                verdict,
                Err(Refusal::Element(_) | Refusal::Attribute { .. })
            ),
            "{tag} {attributes:?} -> {verdict:?}"
        );
    }
}

#[test]
fn event_handlers_are_code_and_only_exact_pins_pass() {
    let hostile = [
        "$.DispatchEvent('ExternalBrowserGoToURL', 'http://evil.example')",
        "$.AsyncWebRequest('http://evil.example', {})",
        "DOTAQuit(); $.Msg('x')",
        "eval('1')",
        "javascript: alert(1)",
        "DOTAQuit()",
        "",
    ];
    for code in hostile {
        for attribute in ["onactivate", "onload", "onmouseover", "OnClick", "ONLOAD"] {
            let panel = el(
                "Panel",
                &[],
                vec![el(
                    "Button",
                    &[("id", "DOTAQuitButton"), (attribute, code)],
                    vec![],
                )],
            );
            let verdict = prove_layout(&layout(vec![panel]), true);
            assert!(
                matches!(verdict, Err(Refusal::EventHandler { .. })),
                "{attribute}={code:?} -> {verdict:?}"
            );
        }
    }
    // The exact pin passes, in the right place, and only with the switch on.
    let pinned = |id: &str, tag: &str, code: &str| {
        layout(vec![el(
            "Panel",
            &[],
            vec![el(tag, &[("id", id), ("onactivate", code)], vec![])],
        )])
    };
    let good = pinned("DOTAQuitButton", "Button", "DOTAQuit();");
    let proof = prove_layout(&good, true).expect("pinned binding");
    assert_eq!(proof.tier, Tier::ReviewedValveBindings);
    assert_eq!(proof.facts.handlers, 1);
    assert!(matches!(
        prove_layout(&good, false),
        Err(Refusal::EventHandler { .. })
    ));
    // The same code on another id, another element type, or with one more
    // character, is a different binding.
    for bad in [
        pinned("MyButton", "Button", "DOTAQuit();"),
        pinned("DOTAQuitButton", "Panel", "DOTAQuit();"),
        pinned("DOTAQuitButton", "Button", "DOTAQuit(); "),
        pinned("DOTAQuitButton", "Button", "DOTAQuit();;"),
    ] {
        assert!(matches!(
            prove_layout(&bad, true),
            Err(Refusal::EventHandler { .. })
        ));
    }
}

#[test]
fn scripts_snippets_passthrough_and_stray_nodes_are_refused() {
    let cases: Vec<(Vec<Node>, &str)> = vec![
        (
            vec![test_node("SCRIPT_BODY", None, vec![]), good_panel()],
            "script body at the root",
        ),
        (
            vec![styles(), test_node("SNIPPETS", None, vec![]), good_panel()],
            "snippets",
        ),
        (
            vec![
                test_includes("STYLES", &["file://panorama/styles/x.css"]),
                good_panel(),
            ],
            "passthrough style include",
        ),
        (
            vec![
                test_includes("STYLES", &["s2r://panorama/styles/missing.vcss_c"]),
                good_panel(),
            ],
            "style include that exists nowhere",
        ),
        (
            vec![
                test_includes("STYLES", &["s2r://panorama/styles/dotastyles.vcss_c"]),
                test_includes("SCRIPTS", &["s2r://panorama/scripts/evil.vts_c"]),
                good_panel(),
            ],
            "script that is not Valve's",
        ),
        (
            vec![
                test_includes("STYLES", &["s2r://panorama/styles/dotastyles.vcss_c"]),
                test_includes("SCRIPTS", &["s2r://zxc/zxc.webm"]),
                good_panel(),
            ],
            "script include of an archive file",
        ),
        (vec![good_panel(), good_panel()], "two root panels"),
        (vec![], "no panel"),
        (vec![good_panel(), styles()], "styles after the panel"),
        (
            vec![el(
                "Panel",
                &[],
                vec![test_node("SCRIPT_BODY", Some("alert(1)"), vec![])],
            )],
            "script body inside a panel",
        ),
    ];
    for (children, why) in cases {
        let verdict = prove_layout(&layout(children), true);
        assert!(verdict.is_err(), "{why}: {verdict:?}");
    }
}

#[test]
fn a_valve_script_passes_only_with_the_switch_on_and_never_when_the_archive_carries_it() {
    let with_script = |script: &str| {
        layout(vec![
            styles(),
            test_includes("SCRIPTS", &[script]),
            good_panel(),
        ])
    };
    let bytes = with_script("s2r://panorama/scripts/util.vts_c");
    let proof = prove_layout(&bytes, true).expect("valve script");
    assert_eq!(proof.tier, Tier::ReviewedValveBindings);
    assert_eq!(proof.facts.script_includes, 1);
    assert!(matches!(
        prove_layout(&bytes, false),
        Err(Refusal::ScriptInclude(_))
    ));
    // The same path, but the archive replaces it.
    let valve = valve();
    let archive = set(&["panorama/scripts/util.vts_c", "zxc/zxc.webm"]);
    assert!(matches!(
        prove_static_layout(
            "panorama/layout/x.vxml_c",
            &bytes,
            &context(&valve, &archive, true)
        ),
        Err(Refusal::ScriptInclude(_))
    ));
}

#[test]
fn reference_forms_and_targets_are_checked() {
    let src = |tag: &str, target: &str| {
        layout(vec![el(
            "Panel",
            &[],
            vec![el(tag, &[], vec![test_reference_attribute("src", target)])],
        )])
    };
    assert!(prove_layout(&src("Image", "s2r://panorama/images/hud/a_png.vtex"), true).is_ok());
    for (tag, target) in [
        ("Image", "file://panorama/images/hud/a_png.vtex"),
        ("Image", "s2r://panorama/images/hud/missing_png.vtex"),
        ("Image", "s2r://../../../x_png.vtex"),
        ("Image", "s2r://panorama/images/hud/A_png.vtex"),
        ("Image", "s2r://zxc/zxc.webm"),
        ("MoviePanel", "s2r://panorama/images/hud/a_png.vtex"),
        ("MoviePanel", "s2r://http/evil.example/x.webm"),
        ("Label", "s2r://zxc/zxc.webm"),
        ("Panel", "s2r://zxc/zxc.webm"),
    ] {
        assert!(
            prove_layout(&src(tag, target), true).is_err(),
            "{tag} {target}"
        );
    }
}

#[test]
fn hostile_attribute_values_are_refused() {
    let big = "A".repeat(1000);
    let cases = [
        ("id", big.as_str()),
        ("id", ""),
        ("id", "a\"b"),
        ("id", "a<script>"),
        ("class", "x;y"),
        ("hittest", "yes"),
        ("map", "../../etc"),
        ("map", "a b"),
        ("map", "http://evil.example/m"),
        ("text", "javascript:alert(1)/*"),
        ("text", "<b>"),
        ("toastduration", "5.0.0s"),
        ("toastduration", "x"),
    ];
    for (key, value) in cases {
        let panel = el("Panel", &[], vec![el("Label", &[(key, value)], vec![])]);
        let verdict = prove_layout(&layout(vec![panel]), true);
        assert!(
            matches!(
                verdict,
                Err(Refusal::AttributeValue(_) | Refusal::Attribute { .. })
            ),
            "{key}={value:?} -> {verdict:?}"
        );
    }
    let duplicate = el("Panel", &[("id", "a"), ("id", "b")], vec![]);
    assert!(matches!(
        prove_layout(&layout(vec![duplicate]), true),
        Err(Refusal::DuplicateAttribute(_))
    ));
}

#[test]
fn deep_and_huge_trees_are_refused_not_walked_forever() {
    let mut deep = el("Panel", &[], vec![]);
    for _ in 0..(MAX_DEPTH + 10) {
        deep = el("Panel", &[], vec![deep]);
    }
    let bytes = layout(vec![deep]);
    assert!(matches!(
        prove_layout(&bytes, true),
        Err(Refusal::TooDeep | Refusal::Container | Refusal::Oversized)
    ));
    let wide = el(
        "Panel",
        &[],
        (0..MAX_ELEMENTS + 5)
            .map(|_| el("Panel", &[], vec![]))
            .collect(),
    );
    assert!(prove_layout(&layout(vec![wide]), true).is_err());
}

#[test]
fn containers_that_are_not_the_audited_shape_are_refused() {
    let panel = || layout(vec![styles(), good_panel()]);
    // Extra blocks: a reference list or an unknown block.
    for tag in [*b"RERL", *b"REDI", *b"XXXX"] {
        let bytes = test_layout_bytes(
            root(vec![styles(), good_panel()]),
            vec![ResourceBlock {
                tag,
                bytes: vec![0; 8],
            }],
            None,
        );
        assert!(
            matches!(
                prove_layout(&bytes, true),
                Err(Refusal::BlockNotAllowed(_) | Refusal::Container)
            ),
            "{tag:?}"
        );
    }
    // A second copy of the layout as text beside the tree.
    let bytes = test_layout_bytes(
        root(vec![styles(), good_panel()]),
        Vec::new(),
        Some("<root><Panel/></root>"),
    );
    assert!(matches!(
        prove_layout(&bytes, true),
        Err(Refusal::Container)
    ));
    assert!(matches!(prove_layout(&[], true), Err(Refusal::Empty)));
    assert!(matches!(
        prove_layout(&vec![0u8; MAX_LAYOUT_BYTES + 1], true),
        Err(Refusal::Oversized)
    ));
    assert!(matches!(
        prove_static_layout(
            "panorama/layout/x.png",
            &panel(),
            &context(&valve(), &BTreeSet::new(), true)
        ),
        Err(Refusal::PathNotAllowed)
    ));
}

/// A tiny deterministic generator for the mutation tests.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn truncated_and_corrupted_layouts_never_panic_and_truncated_ones_never_pass() {
    let valid = layout(vec![styles(), good_panel()]);
    assert!(prove_layout(&valid, true).is_ok());
    for length in 0..valid.len() {
        assert!(
            prove_layout(&valid[..length], true).is_err(),
            "prefix of {length} bytes passed"
        );
    }
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..3_000 {
        let mut mutated = valid.clone();
        for _ in 0..(1 + rng.next() % 4) {
            let at = (rng.next() as usize) % mutated.len();
            mutated[at] = rng.next() as u8;
        }
        // Whatever it is, it must not panic; the verdict is not asserted
        // (a changed byte can still be a valid, different layout).
        let _ = prove_layout(&mutated, true);
    }
}

// ---------- styles ----------

fn style_bytes(css: &str) -> Vec<u8> {
    style_resource(css, 0, vec![])
}

fn style_resource(css: &str, images: u16, extra: Vec<ResourceBlock>) -> Vec<u8> {
    let mut data = vec![0x12, 0x34, 0x56, 0x78];
    data.extend_from_slice(&images.to_le_bytes());
    data.extend_from_slice(css.as_bytes());
    let mut blocks = vec![ResourceBlock {
        tag: *b"RED2",
        bytes: vec![1; 20],
    }];
    blocks.extend(extra);
    blocks.push(ResourceBlock {
        tag: *b"DATA",
        bytes: data,
    });
    blocks.push(ResourceBlock {
        tag: *b"SrMa",
        bytes: vec![2; 10],
    });
    panorama::build(&panorama::Resource { version: 3, blocks }).expect("builds")
}

fn prove_css(css: &str) -> Result<Proof, Refusal> {
    let valve = valve();
    let archive = set(&["panorama/images/mine_png.vtex_c"]);
    prove_style_sheet(
        "panorama/styles/dashboard.vcss_c",
        &style_bytes(css),
        &context(&valve, &archive, true),
    )
}

const GOOD_CSS: &str = r#"
@define unlearnedWashColor: #666666;
@import url("s2r://panorama/styles/dotastyles.vcss_c");
#MovieBackground, .A > .B:not(.C) {
    width: 100%; height: 100%; opacity: 0.4;
    background-color: gradient( linear, 0% 0%, 0% 100%, from( #000000 ), to( rgba(0,0,0,0.5) ) );
    transition-property: transform, blur; transition-duration: 0.45s;
    opacity-mask: url("s2r://panorama/images/masks/bg_vignette_psd.vtex");
    background-image: url( "s2r://panorama/images/mine_png.vtex" );
    sound: "ui_generic_button_click";
    animation-name: 'Pulse';
    transform: translateX( -120px ) scale3d( 1.1, 1.1, 1 );
}
@keyframes 'Pulse' {
    from { opacity: 0; }
    50.5% { opacity: 1 }
    to { transform: scaleX( 1 ); }
}
"#;

#[test]
fn ordinary_visual_css_is_accepted_and_its_references_resolved() {
    let proof = prove_css(GOOD_CSS).expect("proven");
    assert_eq!(proof.kind, Kind::Style);
    assert_eq!(proof.tier, Tier::PureDecor);
    assert_eq!(
        proof.dependencies,
        set(&["panorama/images/mine_png.vtex_c"])
    );
    assert_eq!(proof.facts.valve_references, 2);
    assert!(proof.facts.declarations >= 12);
}

#[test]
fn css_that_could_reach_outside_or_do_anything_but_style_is_refused() {
    let hostile = [
        "#a { behavior: url(evil.htc); }",
        "#a { -moz-binding: url(x); }",
        "#a { background-image: url(\"http://evil.example/x.png\"); }",
        "#a { background-image: url(\"https://evil.example/x.png\"); }",
        "#a { background-image: url(\"file://c:/x.png\"); }",
        "#a { background-image: url(\"data:image/png;base64,AAAA\"); }",
        "#a { background-image: url(\"s2r://panorama/images/hud/missing_png.vtex\"); }",
        "#a { background-image: url(\"s2r://../../x_png.vtex\"); }",
        "#a { background-image: url(s2r://panorama/images/hud/a_png.vtex); }",
        "#a { background-image: url(\"s2r://panorama/images/hud/a_png.vtex\") url(\"x\"); }",
        "#a { background-image: url(\"s2r://panorama/styles/dotastyles.vcss_c\"); }",
        "@import url(\"http://evil.example/a.css\");",
        "@import \"s2r://panorama/styles/dotastyles.vcss_c\";",
        "@import url(\"s2r://panorama/styles/nope.vcss_c\");",
        "@font-face { font-family: x; }",
        "@media screen { #a { width: 1px; } }",
        "@charset \"utf-8\";",
        "@keyframes 'A' { from { opacity: 0; } } @keyframes B x { }",
        "#a { width: expression(alert(1)); }",
        "#a { width: calc(1px + 2px); }",
        "#a { width: evil( 1 ); }",
        "#a { width: 1px } #b { visibility: visible; } }",
        "#a { x-unknown: 1; }",
        "#a { content: \"x\"; }",
        "#a { width: 1px !important; }",
        "#a { width: \\31 px; }",
        "#a { width: 1px; background-image: url(\"s2r://panorama/images/hud/a_png.vtex\"",
        "#a { width: 1px; ",
        "#a { width: 1px; /* never closed",
        "#a { width: 1px; } /* closed */",
        "/* x */ #a { width: 1px; }",
        "#a { width: \"unterminated; }",
        "#a { #b { width: 1px; } }",
        "#a { width 1px; }",
        "{ width: 1px; }",
        "#a < b { width: 1px; }",
        "#a { width: 1px; } stray text",
        "@define x 1;",
        "@define 1x: 1;",
        "#a { width: 1px\u{200b}; }",
        "#a { width: 1px;\u{7f} }",
    ];
    for css in hostile {
        let verdict = prove_css(css);
        assert!(verdict.is_err(), "accepted: {css:?}");
    }
}

#[test]
fn style_containers_are_checked_too() {
    let valve = valve();
    let archive = BTreeSet::new();
    let ctx = context(&valve, &archive, true);
    let path = "panorama/styles/dashboard.vcss_c";
    let good = style_bytes("#a { width: 1px; }");
    assert!(prove_style_sheet(path, &good, &ctx).is_ok());
    assert!(matches!(
        prove_style_sheet(path, &style_resource("#a { width: 1px; }", 1, vec![]), &ctx),
        Err(Refusal::Container)
    ));
    assert!(matches!(
        prove_style_sheet(
            path,
            &style_resource(
                "#a { width: 1px; }",
                0,
                vec![ResourceBlock {
                    tag: *b"RERL",
                    bytes: vec![0; 8]
                }]
            ),
            &ctx
        ),
        Err(Refusal::BlockNotAllowed(_))
    ));
    assert!(matches!(
        prove_style_sheet(path, &[], &ctx),
        Err(Refusal::Empty)
    ));
    assert!(matches!(
        prove_style_sheet(path, &vec![0u8; MAX_STYLE_BYTES + 1], &ctx),
        Err(Refusal::Oversized)
    ));
    assert!(matches!(
        prove_style_sheet("panorama/styles/x.css", &good, &ctx),
        Err(Refusal::PathNotAllowed)
    ));
    for length in 0..good.len() {
        assert!(prove_style_sheet(path, &good[..length], &ctx).is_err());
    }
    let huge = format!("#a {{ width: {}; }}", "1".repeat(MAX_CSS_VALUE_BYTES + 1));
    assert!(prove_css(&huge).is_err());
    let many = format!(
        "#a {{ {} }}",
        "width: 1px;".repeat(MAX_CSS_DECLARATIONS + 1)
    );
    assert!(matches!(
        prove_style_sheet(path, &style_bytes(&many), &ctx),
        Err(Refusal::CssTooBig | Refusal::Oversized)
    ));
}

#[test]
fn mutated_css_never_panics_and_forbidden_words_never_get_through() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let words = [
        "javascript:",
        "http://",
        "expression(",
        "@font-face",
        "behavior:",
        "url(x)",
        "\\",
        "<script>",
        "file://",
        "data:",
    ];
    let base = GOOD_CSS.as_bytes();
    for round in 0..4_000 {
        let mut text = base.to_vec();
        for _ in 0..(1 + rng.next() % 3) {
            let at = (rng.next() as usize) % text.len();
            match rng.next() % 3 {
                0 => text[at] = (32 + rng.next() % 95) as u8,
                1 => {
                    text.remove(at);
                }
                _ => {
                    let word = words[(rng.next() as usize) % words.len()];
                    for (offset, byte) in word.bytes().enumerate() {
                        text.insert(at + offset, byte);
                    }
                }
            }
        }
        let Ok(css) = String::from_utf8(text) else {
            continue;
        };
        let verdict = prove_css(&css);
        if verdict.is_ok() {
            // Words made of characters no selector, name or value may hold.
            // (`data:` or `javascript:` alone can be a selector, which is
            // harmless; as a URL they fail the `url()` rule instead.)
            for word in ["http://", "file://", "<script>", "\\", "@font-face"] {
                assert!(
                    !css.contains(word),
                    "round {round}: {word:?} got through: {css}"
                );
            }
        }
    }
}

// ---------- videos ----------

fn ebml(id: u32, body: &[u8]) -> Vec<u8> {
    let mut out = id
        .to_be_bytes()
        .into_iter()
        .skip_while(|byte| *byte == 0)
        .collect::<Vec<_>>();
    out.extend_from_slice(&(0x1000_0000u32 | body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    out
}

fn webm_header(doctype: &[u8]) -> Vec<u8> {
    let body = [
        ebml(0x4286, &[1]),
        ebml(0x42F7, &[1]),
        ebml(0x42F2, &[4]),
        ebml(0x42F3, &[8]),
        ebml(0x4282, doctype),
        ebml(0x4287, &[4]),
        ebml(0x4285, &[2]),
    ]
    .concat();
    ebml(0x1A45_DFA3, &body)
}

fn track(codec: &[u8], kind: u8) -> Vec<u8> {
    ebml(
        0xAE,
        &[ebml(0xD7, &[1]), ebml(0x83, &[kind]), ebml(0x86, codec)].concat(),
    )
}

fn webm_with(top: Vec<Vec<u8>>) -> Vec<u8> {
    let mut file = webm_header(b"webm");
    file.extend(ebml(0x1853_8067, &top.concat()));
    file
}

fn good_top() -> Vec<Vec<u8>> {
    vec![
        ebml(0x114D_9B74, &[0; 4]),
        ebml(0xEC, &[0; 6]),
        ebml(0x1549_A966, &ebml(0x2A_D7B1, &[0x0F, 0x42, 0x40])),
        ebml(
            0x1654_AE6B,
            &[track(b"V_VP9", 1), track(b"A_OPUS", 2)].concat(),
        ),
        ebml(0x1F43_B675, &ebml(0xE7, &[0])),
        ebml(0x1F43_B675, &ebml(0xE7, &[1])),
        ebml(0x1C53_BB6B, &[0; 4]),
        ebml(0x1254_C367, &[0; 4]),
    ]
}

#[test]
fn a_well_formed_webm_container_passes_and_says_what_it_found() {
    let facts = check_webm(&webm_with(good_top())).expect("container");
    assert_eq!(facts.clusters, 2);
    assert_eq!(facts.video_codec, "V_VP9");
    assert_eq!(facts.audio_codec.as_deref(), Some("A_OPUS"));
}

#[test]
fn webm_containers_that_are_not_plain_video_are_refused() {
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    // Wrong doctype.
    let mut wrong = webm_header(b"matroska");
    wrong.extend(ebml(0x1853_8067, &good_top().concat()));
    cases.push(("matroska doctype", wrong));
    // Attachments and chapters can carry files.
    for (name, id) in [("attachments", 0x1941_A469u32), ("chapters", 0x1043_A770)] {
        let mut top = good_top();
        top.insert(3, ebml(id, &[0; 8]));
        cases.push((name, webm_with(top)));
    }
    // An unknown top-level element.
    let mut top = good_top();
    top.insert(2, ebml(0x1234_5678, &[1, 2, 3]));
    cases.push(("unknown element", webm_with(top)));
    // Codecs.
    for codec in [
        &b"V_MS/VFW/FOURCC"[..],
        b"V_UNCOMPRESSED",
        b"V_QUICKTIME",
        b"S_TEXT/ASS",
    ] {
        let mut top = good_top();
        top[3] = ebml(0x1654_AE6B, &track(codec, 1));
        cases.push(("codec", webm_with(top)));
    }
    // A second video track, a track with no codec, no tracks, no cluster.
    let mut top = good_top();
    top[3] = ebml(
        0x1654_AE6B,
        &[track(b"V_VP9", 1), track(b"V_VP8", 1)].concat(),
    );
    cases.push(("two video tracks", webm_with(top)));
    let mut top = good_top();
    top[3] = ebml(
        0x1654_AE6B,
        &ebml(0xAE, &[ebml(0xD7, &[1]), ebml(0x83, &[1])].concat()),
    );
    cases.push(("track without codec", webm_with(top)));
    let mut top = good_top();
    top.remove(3);
    cases.push(("no tracks", webm_with(top)));
    let mut top = good_top();
    top.retain(|e| !e.starts_with(&[0x1F, 0x43, 0xB6, 0x75]));
    cases.push(("no clusters", webm_with(top)));
    // The segment must span the rest of the file exactly.
    let mut trailing = webm_with(good_top());
    trailing.extend_from_slice(&[0, 0, 0]);
    cases.push(("trailing bytes", trailing));
    let mut short = webm_with(good_top());
    short.truncate(short.len() - 3);
    cases.push(("short segment", short));
    // Unknown segment size (a live stream).
    let mut live = webm_header(b"webm");
    live.extend_from_slice(&[
        0x18, 0x53, 0x80, 0x67, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ]);
    live.extend(good_top().concat());
    cases.push(("unknown segment size", live));
    // Not EBML at all.
    cases.push((
        "png",
        vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13],
    ));
    cases.push(("empty header", ebml(0x1A45_DFA3, &[])));
    for (why, bytes) in cases {
        assert!(check_webm(&bytes).is_err(), "{why} was accepted");
    }
    assert!(matches!(check_webm(&[]), Err(Refusal::Empty)));
}

#[test]
fn truncated_and_corrupted_webm_never_panic_and_truncated_never_pass() {
    let valid = webm_with(good_top());
    for length in 0..valid.len() {
        assert!(
            check_webm(&valid[..length]).is_err(),
            "prefix of {length} bytes passed"
        );
    }
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    for _ in 0..5_000 {
        let mut mutated = valid.clone();
        for _ in 0..(1 + rng.next() % 4) {
            let at = (rng.next() as usize) % mutated.len();
            mutated[at] = rng.next() as u8;
        }
        let _ = check_webm(&mutated);
    }
    // Element sizes that run past the file or overflow.
    let mut huge = webm_header(b"webm");
    huge.extend_from_slice(&[
        0x18, 0x53, 0x80, 0x67, 0x01, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
    ]);
    assert!(check_webm(&huge).is_err());
    assert!(check_webm(&[0u8; 64]).is_err());
    assert!(check_webm(&[0xFFu8; 64]).is_err());
}

#[test]
fn the_size_bound_is_checked_before_anything_is_read() {
    assert!(matches!(
        check_webm(&vec![0x1A; MAX_WEBM_BYTES + 1]),
        Err(Refusal::Oversized)
    ));
}

// ---------- archives ----------

fn archive_entries<'a>(items: &'a [(&str, Vec<u8>)]) -> Vec<(String, &'a [u8])> {
    items
        .iter()
        .map(|(path, bytes)| (path.to_string(), bytes.as_slice()))
        .collect()
}

fn background_layout(video: &str) -> Vec<u8> {
    layout(vec![
        styles(),
        el(
            "DOTADashboardBackgroundLastMatch",
            &[],
            vec![el(
                "MoviePanel",
                &[("id", "MovieBackground")],
                vec![test_reference_attribute("src", &format!("s2r://{video}"))],
            )],
        ),
    ])
}

#[test]
fn an_archive_is_accepted_whole_or_each_failure_is_listed() {
    let valve = valve();
    let webm = webm_with(good_top());
    let items = vec![
        (
            "panorama/layout/dashboard_background_last_match.vxml_c",
            background_layout("zxc/zxc.webm"),
        ),
        ("zxc/zxc.webm", webm.clone()),
        (
            "panorama/styles/dashboard_background_last_match.vcss_c",
            style_bytes("#MovieBackground { opacity: 0.4; }"),
        ),
        ("panorama/version.txt", b"content_x".to_vec()),
    ];
    let verdicts = prove_archive(
        Category::Backgrounds,
        &archive_entries(&items),
        &valve,
        true,
    );
    assert_eq!(verdicts.len(), 3, "only decor files get a verdict");
    assert!(verdicts.values().all(Result::is_ok), "{verdicts:?}");

    // The layout names a video the archive does not carry.
    let items = vec![(
        "panorama/layout/dashboard_background_last_match.vxml_c",
        background_layout("zxc/other.webm"),
    )];
    let verdicts = prove_archive(
        Category::Backgrounds,
        &archive_entries(&items),
        &valve,
        true,
    );
    assert!(matches!(
        verdicts.values().next().unwrap(),
        Err(Refusal::ReferenceMissing(_))
    ));

    // A video nobody plays, a video that is not a video, a nested video path.
    let items = vec![
        ("zxc/zxc.webm", webm.clone()),
        ("zxc/fake.webm", b"MZ\x90\x00 not a video".to_vec()),
        ("a/b/c.webm", webm.clone()),
    ];
    let verdicts = prove_archive(
        Category::Backgrounds,
        &archive_entries(&items),
        &valve,
        true,
    );
    assert!(matches!(
        verdicts["zxc/zxc.webm"],
        Err(Refusal::VideoUnreferenced)
    ));
    assert!(matches!(
        verdicts["zxc/fake.webm"],
        Err(Refusal::WebmContainer(_))
    ));
    assert!(matches!(
        verdicts["a/b/c.webm"],
        Err(Refusal::PathNotAllowed)
    ));
}

#[test]
fn categories_only_take_the_files_their_purpose_needs() {
    let valve = valve();
    let items = vec![
        (
            "panorama/layout/popups/popup_accept_match.vxml_c",
            background_layout("zxc/zxc.webm"),
        ),
        ("zxc/zxc.webm", webm_with(good_top())),
        (
            "panorama/styles/dashboard.vcss_c",
            style_bytes("#a { width: 1px; }"),
        ),
        (
            "panorama/styles/popups/popup_accept_match.vcss_c",
            style_bytes("#a { width: 1px; }"),
        ),
    ];
    let entries = archive_entries(&items);
    // A background may not replace the accept-match popup, nor a popup style.
    let verdicts = prove_archive(Category::Backgrounds, &entries, &valve, true);
    assert!(matches!(
        verdicts["panorama/layout/popups/popup_accept_match.vxml_c"],
        Err(Refusal::NotForCategory)
    ));
    assert!(matches!(
        verdicts["panorama/styles/popups/popup_accept_match.vcss_c"],
        Err(Refusal::NotForCategory)
    ));
    assert!(verdicts["panorama/styles/dashboard.vcss_c"].is_ok());
    // A HUD skin carries styles only; a versus screen no video.
    let verdicts = prove_archive(Category::Huds, &entries, &valve, true);
    assert!(matches!(
        verdicts["zxc/zxc.webm"],
        Err(Refusal::NotForCategory)
    ));
    assert!(matches!(
        verdicts["panorama/layout/popups/popup_accept_match.vxml_c"],
        Err(Refusal::NotForCategory)
    ));
    let verdicts = prove_archive(Category::VersusScreens, &entries, &valve, true);
    assert!(verdicts.values().all(|verdict| verdict.is_err()));
    // A HUD skin styles the HUD, not the popups or the dashboard.
    let items = vec![
        (
            "panorama/styles/hud/dota_hud_xp.vcss_c",
            style_bytes("#a { width: 1px; }"),
        ),
        (
            "panorama/styles/dota_hud_roshan_timer.vcss_c",
            style_bytes("#a { width: 1px; }"),
        ),
        (
            "panorama/styles/ui_stat_branch.vcss_c",
            style_bytes("#a { width: 1px; }"),
        ),
        (
            "panorama/styles/dashboard.vcss_c",
            style_bytes("#a { width: 1px; }"),
        ),
        (
            "panorama/styles/popups/popup_settings.vcss_c",
            style_bytes("#a { width: 1px; }"),
        ),
    ];
    let verdicts = prove_archive(Category::Huds, &archive_entries(&items), &valve, true);
    let accepted = verdicts
        .iter()
        .filter(|(_, verdict)| verdict.is_ok())
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        accepted,
        [
            "panorama/styles/dota_hud_roshan_timer.vcss_c",
            "panorama/styles/hud/dota_hud_xp.vcss_c",
            "panorama/styles/ui_stat_branch.vcss_c"
        ]
    );
    // A video never replaces one of Valve's own files.
    let mut with_video = valve.clone();
    with_video.insert("zxc/zxc.webm".to_string());
    let items = vec![
        (
            "panorama/layout/dashboard_background_last_match.vxml_c",
            background_layout("zxc/zxc.webm"),
        ),
        ("zxc/zxc.webm", webm_with(good_top())),
    ];
    let verdicts = prove_archive(
        Category::Backgrounds,
        &archive_entries(&items),
        &with_video,
        true,
    );
    assert!(matches!(
        verdicts["zxc/zxc.webm"],
        Err(Refusal::PathNotAllowed)
    ));
    assert!(verdicts["panorama/layout/dashboard_background_last_match.vxml_c"].is_err());
}

#[test]
fn in_archive_references_must_be_accepted_themselves_and_loops_are_refused() {
    let valve = valve();
    // `a` imports `b`, which has a forbidden property; `c` and `d` import
    // each other; `e` imports a good style.
    let import = |target: &str| {
        format!("@import url(\"s2r://panorama/styles/{target}.vcss_c\"); #x {{ width: 1px; }}")
    };
    let items = vec![
        (
            "panorama/styles/dashboard_a.vcss_c",
            style_bytes(&import("dashboard_b")),
        ),
        (
            "panorama/styles/dashboard_b.vcss_c",
            style_bytes("#x { behavior: url(x); }"),
        ),
        (
            "panorama/styles/dashboard_c.vcss_c",
            style_bytes(&import("dashboard_d")),
        ),
        (
            "panorama/styles/dashboard_d.vcss_c",
            style_bytes(&import("dashboard_c")),
        ),
        (
            "panorama/styles/dashboard_e.vcss_c",
            style_bytes(&import("dashboard_f")),
        ),
        (
            "panorama/styles/dashboard_f.vcss_c",
            style_bytes("#x { width: 2px; }"),
        ),
        (
            "panorama/styles/dashboard_g.vcss_c",
            style_bytes(&import("dashboard_g")),
        ),
        (
            "panorama/styles/dashboard_h.vcss_c",
            style_bytes(&import("dashboard_a")),
        ),
    ];
    let verdicts = prove_archive(
        Category::Backgrounds,
        &archive_entries(&items),
        &valve,
        true,
    );
    let get = |name: &str| &verdicts[&format!("panorama/styles/dashboard_{name}.vcss_c")];
    assert!(matches!(get("b"), Err(Refusal::CssProperty(_))));
    assert!(matches!(get("a"), Err(Refusal::DependencyRefused(_))));
    assert!(matches!(get("h"), Err(Refusal::DependencyRefused(_))));
    assert!(matches!(get("c"), Err(Refusal::DependencyCycle)));
    assert!(matches!(get("d"), Err(Refusal::DependencyCycle)));
    assert!(matches!(get("g"), Err(Refusal::DependencyCycle)));
    assert!(get("e").is_ok());
    assert!(get("f").is_ok());
}

#[test]
fn every_pinned_binding_is_plain_and_names_a_listed_element() {
    let mut seen = BTreeSet::new();
    for (element, id, attribute, code) in pins::all() {
        assert!(ELEMENTS.contains(element), "{element}");
        assert!(attribute.len() >= 2 && attribute[..2].eq_ignore_ascii_case("on"));
        assert!(handler_shape_ok(code), "{code}");
        assert!(id.is_empty() || token_ok(id), "{id}");
        assert!(
            seen.insert((element, id, attribute)),
            "two pins for one {element}#{id}.{attribute}"
        );
    }
    assert!(!seen.is_empty());
    // The shape check refuses what the review would.
    for code in [
        "$.Msg('x')",
        "a.b()",
        "x = 1",
        "ExternalBrowserGoToURL('http://x')",
        "f(\"x\")",
        "f(",
        "f(1))",
        "a b `c`",
        "f( 1.x )",
    ] {
        assert!(!handler_shape_ok(code), "{code}");
    }
}
