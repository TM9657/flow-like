import assert from "node:assert/strict";
import test from "node:test";
import {
	assertPinnedLockPackages,
	assertPlatformMediaDependencies,
} from "./codec-policy.mjs";

test("platform media builds retain containers, rendering and lossless writers", () => {
	assert.doesNotThrow(() =>
		assertPlatformMediaDependencies(
			"filmcraft",
			[
				"filmcraft-web v0.7.0 (/source/apps/filmcraft-web)",
				"filmcraft-isobmff v0.7.0 (/source/crates/isobmff)",
				"filmcraft-mpegts v0.7.0 (/source/crates/mpegts)",
				"filmcraft-frame v0.7.0 (/source/crates/frame)",
				"filmcraft-flac v0.7.0 (/source/crates/flac)",
				"wasm-bindgen v0.2.129 (*)",
			].join("\n"),
			"filmcraft-web",
		),
	);
});

test("rejects direct and transitive software codec dependencies", () => {
	for (const codec of [
		"symphonia",
		"symphonia-codec-aac",
		"symphonia-bundle-mp3",
		"deckcraft-h264",
		"deckcraft-av1",
		"deckcraft-opus",
		"filmcraft-hevc",
		"filmcraft-h264enc",
		"filmcraft-dnx",
		"filmcraft-ac3",
		"filmcraft-prores",
		"filmcraft-apv",
		"filmcraft-vp9",
		"ffmpeg-next",
		"openh264-sys",
		"rav1e",
		"future-mp3",
		"future-mp2",
		"future-vorbis",
		"alac",
		"minimp3-sys",
		"puremp3",
		"rmp3",
		"lewton",
		"audiopus",
		"mp3lame-encoder",
	]) {
		assert.throws(
			() =>
				assertPlatformMediaDependencies(
					"deckcraft",
					`deckcraft-web v0.6.0\n${codec} v1.0.0`,
					"deckcraft-web",
				),
			/browser\/OS media codecs/,
			codec,
		);
	}
});

test("an empty, malformed or unrelated graph cannot pass the codec check", () => {
	for (const tree of ["", "not a Cargo graph", "some-other-app v1.0.0"])
		assert.throws(() =>
			assertPlatformMediaDependencies("soundcraft", tree, "soundcraft-web"),
		);
});

test("lock reconciliation can remove codecs without upgrading dependencies", () => {
	const original = `version = 4
[[package]]
name = "editor-web"
version = "1.0.0"
dependencies = ["software-codec", "wasm-bindgen"]
[[package]]
name = "software-codec"
version = "1.0.0"
[[package]]
name = "wasm-bindgen"
version = "0.2.129"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc123"
`;
	const reconciled = original
		.replace('"software-codec", ', "")
		.replace('[[package]]\nname = "software-codec"\nversion = "1.0.0"\n', "");
	assert.doesNotThrow(() => assertPinnedLockPackages(original, reconciled));
	for (const changed of [
		reconciled.replace('"0.2.129"', '"0.2.130"'),
		reconciled.replace('checksum = "abc123"', 'checksum = "changed"'),
		reconciled.replace("registry+https:", "git+https:"),
		`${reconciled}\n[[package]]\nname = "new-codec"\nversion = "1.0.0"`,
		"",
	])
		assert.throws(() => assertPinnedLockPackages(original, changed));
});
