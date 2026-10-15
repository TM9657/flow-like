import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { patchThumbnailVisibility } from "./deckcraft/patch.mjs";

// The thumbnail paint block from the pinned DeckCraft revision.
const paint = `        let wpx = (tw * ppp).round() as u32;
        match app.textures.thumb(ui.ctx(), &doc, i, wpx, &mut budget) {
            Some(tex) => {
                p.image(
                    tex.id(),
                    img,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    if hidden { Color32::from_white_alpha(140) } else { Color32::WHITE },
                );
            }
            None => {
                p.rect_filled(img, CornerRadius::ZERO, Color32::WHITE);
                need_more = true;
            }
        }
`;
const after = `        if selected {
            p.rect_stroke(img.expand(2.0)`;

function replaceOnce(source, search, replacement, label) {
	assert.equal(source.split(search).length, 2, label);
	return source.replace(search, replacement);
}

test("thumbnail patch rejects changed or repeated upstream boundaries", () => {
	for (const source of [paint, after, paint + after + paint, after + paint]) {
		assert.throws(() => patchThumbnailVisibility(source, replaceOnce));
	}
});

test("a long deck spends its thumbnail budget only on visible slides and settles", () => {
	const patched = patchThumbnailVisibility(paint + after, replaceOnce);
	assert.ok(patched.endsWith(after));
	const block = patched.slice(0, -after.length);
	const directory = mkdtempSync(join(tmpdir(), "deckcraft-thumbnails-"));
	try {
		const source = join(directory, "thumbnails.rs");
		const binary = join(directory, "thumbnails");
		writeFileSync(
			source,
			`use std::collections::HashSet;
use std::ops::Range;

#[derive(Clone, Copy)]
struct Rect(usize);
impl Rect { fn from_min_max(_: (), _: ()) -> Self { Self(0) } }
fn pos2(_: f32, _: f32) {}
struct Color32;
impl Color32 {
    const WHITE: Self = Self;
    fn from_white_alpha(_: u8) -> Self { Self }
}
struct CornerRadius;
impl CornerRadius { const ZERO: Self = Self; }
struct Painter;
impl Painter {
    fn image(&self, _: usize, _: Rect, _: Rect, _: Color32) {}
    fn rect_filled(&self, _: Rect, _: CornerRadius, _: Color32) {}
}
struct Ui { visible: Range<usize> }
impl Ui {
    fn ctx(&self) -> &Self { self }
    fn is_rect_visible(&self, img: Rect) -> bool { self.visible.contains(&img.0) }
}
struct Texture;
impl Texture { fn id(&self) -> usize { 0 } }
#[derive(Default)]
struct Textures {
    cached: HashSet<usize>,
    requested: Vec<usize>,
    rendered: Vec<usize>,
}
impl Textures {
    fn thumb(&mut self, _: &Ui, _: &(), index: usize, _: u32, budget: &mut u32) -> Option<Texture> {
        self.requested.push(index);
        if self.cached.contains(&index) { return Some(Texture); }
        if *budget == 0 { return None; }
        *budget -= 1;
        self.rendered.push(index);
        self.cached.insert(index);
        Some(Texture)
    }
}
#[derive(Default)]
struct App { textures: Textures }
fn frame(app: &mut App, visible: Range<usize>) -> bool {
    let ui = Ui { visible };
    let p = Painter;
    let doc = ();
    let (tw, ppp) = (100.0_f32, 2.0_f32);
    let mut budget = 3;
    let mut need_more = false;
    for i in 0..700 {
        let img = Rect(i);
        let hidden = false;
${block}
    }
    need_more
}
fn main() {
    let mut app = App::default();
    assert!(frame(&mut app, 650..655), "visible misses need another frame");
    assert_eq!(app.textures.rendered, vec![650, 651, 652]);
    assert!(!frame(&mut app, 650..655), "offscreen slides must not keep repainting");
    assert_eq!(app.textures.rendered, vec![650, 651, 652, 653, 654]);
    assert!(app.textures.requested.iter().all(|i| (650..655).contains(i)));
    assert!(!frame(&mut app, 650..655));
    assert_eq!(app.textures.rendered.len(), 5, "stationary frames reuse textures");
    assert!(frame(&mut app, 0..5));
    assert!(!frame(&mut app, 0..5));
    assert_eq!(app.textures.rendered.len(), 10, "scrolling renders newly visible slides");
    assert!(!frame(&mut app, 650..655));
    assert_eq!(app.textures.rendered.len(), 10, "scrolling back reuses textures");
}
`,
		);
		execFileSync("rustc", ["--edition=2024", source, "-o", binary], {
			stdio: "pipe",
		});
		execFileSync(binary, [], { stdio: "pipe" });
	} finally {
		rmSync(directory, { recursive: true, force: true });
	}
});
