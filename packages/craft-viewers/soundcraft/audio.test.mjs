import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { assertReviewedModules } from "./patch.mjs";

const source = readFileSync(new URL("./audio.js", import.meta.url), "utf8");
let serial = 0;

async function setup({
	channels = [
		[0, 0.5, -0.5],
		[1, -1, 0.25],
	],
	decodeError,
	audioOverride,
	unavailable = false,
} = {}) {
	const instances = [];
	const listeners = new Map();
	class AudioContext {
		state = "suspended";
		resumes = 0;
		closed = 0;
		constructor(options) {
			this.options = options;
			instances.push(this);
		}
		async decodeAudioData(bytes) {
			this.input = bytes;
			if (decodeError) throw decodeError;
			return (
				audioOverride ?? {
					numberOfChannels: channels.length,
					length: channels[0].length,
					sampleRate: 48000,
					getChannelData: (channel) => Float32Array.from(channels[channel]),
				}
			);
		}
		async close() {
			this.closed++;
			this.state = "closed";
		}
		async resume() {
			this.resumes++;
			this.state = "running";
		}
	}
	globalThis.window = {
		...(unavailable ? {} : { AudioContext }),
		addEventListener: (name, callback) => listeners.set(name, callback),
	};
	const module = await import(
		`data:text/javascript;base64,${Buffer.from(`${source}\n// ${serial++}`).toString("base64")}`
	);
	return { ...module, instances, listeners };
}

test("browser-decoded stereo audio reaches the editor as interleaved float PCM", async () => {
	const { decodeAudio, instances } = await setup();
	const input = Uint8Array.of(0x49, 0x44, 0x33, 1);
	const bytes = await decodeAudio(input);
	assert.deepEqual(input, Uint8Array.of(0x49, 0x44, 0x33, 1));
	assert.deepEqual(new Uint8Array(instances[0].input), input);
	assert.notEqual(instances[0].input, input.buffer);
	assert.equal(instances[0].options.sampleRate, 48000);
	assert.equal(instances[0].closed, 1);
	assert.equal(Buffer.from(bytes.subarray(0, 4)).toString(), "RIFF");
	assert.equal(Buffer.from(bytes.subarray(8, 16)).toString(), "WAVEfmt ");
	assert.equal(Buffer.from(bytes.subarray(36, 40)).toString(), "data");
	const wav = new DataView(bytes.buffer);
	assert.equal(wav.getUint32(4, true), bytes.length - 8);
	assert.equal(wav.getUint16(20, true), 3);
	assert.equal(wav.getUint16(22, true), 2);
	assert.equal(wav.getUint32(24, true), 48000);
	assert.equal(wav.getUint32(28, true), 384000);
	assert.equal(wav.getUint16(32, true), 8);
	assert.equal(wav.getUint16(34, true), 32);
	assert.equal(wav.getUint32(40, true), 24);
	assert.deepEqual(
		Array.from({ length: 6 }, (_, i) => wav.getFloat32(44 + i * 4, true)),
		[0, 1, 0.5, -1, -0.5, 0.25],
	);
});

test("non-finite decoded samples cannot enter the editor", async () => {
	const { decodeAudio } = await setup({
		channels: [
			[Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, 0.75],
		],
	});
	const bytes = await decodeAudio(Uint8Array.of(1));
	const view = new DataView(bytes.buffer);
	assert.deepEqual(
		Array.from({ length: 4 }, (_, i) => view.getFloat32(44 + i * 4, true)),
		[0, 0, 0, 0.75],
	);
});

test("missing Web Audio reports a usable PCM conversion alternative", async () => {
	const { decodeAudio, instances } = await setup({ unavailable: true });
	await assert.rejects(
		decodeAudio(Uint8Array.of(1)),
		/PCM WAV or AIFF.*Web Audio/,
	);
	assert.equal(instances.length, 0);
});

test("a rejected browser codec reports the unsupported format without a software fallback", async () => {
	const reason = new Error("EncodingError");
	const { decodeAudio, instances } = await setup({ decodeError: reason });
	await assert.rejects(decodeAudio(Uint8Array.of(1)), (error) => {
		assert.match(
			error.message,
			/could not decode.*Try another browser.*PCM WAV or AIFF/,
		);
		assert.equal(error.cause, reason);
		return true;
	});
	assert.equal(instances.length, 1);
	assert.equal(instances[0].closed, 1);
});

test("invalid and oversized input never starts a browser decoder", async () => {
	const { decodeAudio, instances } = await setup();
	for (const value of [
		new ArrayBuffer(1),
		new Uint8Array(),
		new Uint8Array(100 * 1024 * 1024 + 1),
	])
		await assert.rejects(decodeAudio(value), /between 1 byte and 100 MiB/);
	assert.equal(instances.length, 0);
});

for (const [label, override] of [
	["empty audio", { length: 0 }],
	["too many channels", { numberOfChannels: 33 }],
	["invalid rate", { sampleRate: 0 }],
	["oversized PCM", { length: 256 * 1024 * 1024 }],
]) {
	test(`rejects ${label} before PCM allocation and releases the decoder`, async () => {
		const { decodeAudio, instances } = await setup({
			audioOverride: {
				length: 1,
				numberOfChannels: 1,
				sampleRate: 48000,
				getChannelData: () => assert.fail("must reject before reading PCM"),
				...override,
			},
		});
		await assert.rejects(decodeAudio(Uint8Array.of(1)), /256 MiB PCM limit/);
		assert.equal(instances[0].closed, 1);
	});
}

test("the canvas gesture still resumes CPAL playback without resuming closed decoder contexts", async () => {
	const { decodeAudio, installAudioUnlock, instances, listeners } =
		await setup();
	const playback = new window.AudioContext();
	await decodeAudio(Uint8Array.of(1));
	installAudioUnlock();
	assert.equal(listeners.size, 4);
	listeners.get("pointerdown")();
	assert.equal(playback.state, "running");
	assert.equal(playback.resumes, 1);
	assert.equal(instances[1].resumes, 0);
	await playback.close();
	listeners.get("keydown")();
	assert.equal(playback.resumes, 1);
});

test("upstream codec module additions require review before the browser build", () => {
	const reviewed =
		"pub mod bitstream;\npub mod h264;\nmod movie;\n#[cfg(test)]\nmod tests;";
	const names = ["bitstream", "h264", "movie", "tests"];
	assert.doesNotThrow(() =>
		assertReviewedModules(reviewed, names, "SoundCraft video"),
	);
	for (const addition of [
		"pub mod hevc;",
		"pub(crate) mod aac;",
		"mod decoder { }",
	])
		assert.throws(
			() =>
				assertReviewedModules(
					`${reviewed}\n${addition}`,
					names,
					"SoundCraft video",
				),
			/modules changed.*review/,
		);
	assert.throws(
		() =>
			assertReviewedModules(
				reviewed.replace("pub mod h264;", ""),
				names,
				"SoundCraft video",
			),
		/modules changed.*review/,
	);
});
