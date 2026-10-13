import assert from "node:assert/strict";
import test from "node:test";

globalThis.addEventListener ??= () => {};
const codecs = await import("./browser-codecs.js");

function platform(t, failures = {}) {
	const names = [
		"VideoEncoder",
		"AudioEncoder",
		"VideoFrame",
		"AudioData",
		"AudioContext",
	];
	const originals = names.map((name) =>
		Object.getOwnPropertyDescriptor(globalThis, name),
	);
	t.after(() =>
		names.forEach((name, index) => {
			if (originals[index])
				Object.defineProperty(globalThis, name, originals[index]);
			else delete globalThis[name];
		}),
	);
	const instances = [];
	const inputs = [];
	const contexts = [];
	let probe;
	function encoder(kind) {
		return class {
			static async isConfigSupported() {
				return { supported: !failures[`${kind}Unsupported`] };
			}
			constructor(callbacks) {
				this.callbacks = callbacks;
				this.state = "unconfigured";
				this.encodeQueueSize = 0;
				this.kind = kind;
				instances.push(this);
			}
			configure() {
				if (failures[`${kind}Configure`])
					throw new Error(`${kind} configuration failed`);
				this.state = "configured";
			}
			encode(input) {
				inputs.push({ kind, bytes: [...input.data] });
				this.input = input;
			}
			async flush() {
				if (failures[`${kind}Flush`]) throw new Error(`${kind} flush failed`);
				if (kind === "audio" && this.input?.numberOfFrames === 8192) {
					probe = this.input;
					output(
						this,
						0,
						((probe.numberOfFrames + (failures.calibrationPriming || 0)) *
							1_000_000) /
							probe.sampleRate,
						failures.calibrationDescription
							? Uint8Array.of(2)
							: Uint8Array.of(1),
					);
				}
			}
			close() {
				assert.notEqual(this.state, "closed");
				this.state = "closed";
			}
		};
	}
	globalThis.VideoEncoder = encoder("video");
	globalThis.AudioEncoder = encoder("audio");
	globalThis.VideoFrame = class {
		constructor(data) {
			this.data = data.slice();
		}
		close() {}
	};
	globalThis.AudioData = class {
		constructor({ data, sampleRate, numberOfFrames }) {
			this.data = data.slice();
			this.sampleRate = sampleRate;
			this.numberOfFrames = numberOfFrames;
		}
		close() {}
	};
	globalThis.AudioContext = class {
		constructor() {
			this.closed = false;
			contexts.push(this);
		}
		async decodeAudioData() {
			const length =
				probe.numberOfFrames +
				(failures.calibrationPriming || 0) -
				(failures.calibrationTrim || 0);
			const samples = new Float32Array(length);
			if (!failures.calibrationNoise)
				samples.set(
					probe.data.slice(0, Math.min(probe.numberOfFrames, length)),
					failures.calibrationPriming || 0,
				);
			return {
				sampleRate: probe.sampleRate,
				length,
				getChannelData: () => samples,
			};
		}
		async close() {
			this.closed = true;
		}
	};
	codecs.setCalibrationMuxer(() => Uint8Array.of(1));
	return { instances, inputs, contexts };
}

test("unsupported platform codecs fail before allocating an encoder", async (t) => {
	const failures = { videoUnsupported: true };
	const state = platform(t, failures);
	await assert.rejects(
		codecs.createEncoder(16, 16, 30, 48000, 2),
		/cannot encode H.264/,
	);
	assert.equal(state.instances.length, 0);
	failures.videoUnsupported = false;
	failures.audioUnsupported = true;
	await assert.rejects(
		codecs.createEncoder(16, 16, 30, 48000, 2),
		/cannot encode AAC/,
	);
	assert.equal(state.instances.length, 0);
});

test("configuration failures close every allocated platform encoder", async (t) => {
	const failures = { videoConfigure: true };
	const state = platform(t, failures);
	await assert.rejects(
		codecs.createEncoder(16, 16, 30, 48000, 2),
		/video configuration failed/,
	);
	assert.deepEqual(
		state.instances.map((instance) => instance.state),
		["closed"],
	);
	failures.videoConfigure = false;
	failures.audioConfigure = true;
	await assert.rejects(
		codecs.createEncoder(16, 16, 30, 48000, 2),
		/audio configuration failed/,
	);
	assert.deepEqual(
		state.instances.map((instance) => instance.state),
		["closed", "closed", "closed"],
	);
});

test("async encoding retains pixels and PCM when WASM backing buffers detach", async (t) => {
	const state = platform(t);
	const id = await codecs.createEncoder(1, 1, 30, 48000, 2);
	const pixels = Uint8Array.of(1, 2, 3, 255);
	const video = codecs.encodeFrame(id, pixels, 1, 1, 0, 33333);
	structuredClone(pixels.buffer, { transfer: [pixels.buffer] });
	await video;
	const samples = Float32Array.of(0.25, -0.5);
	const audio = codecs.encodeAudio(id, samples, 1, 0);
	structuredClone(samples.buffer, { transfer: [samples.buffer] });
	await audio;
	assert.deepEqual(state.inputs, [
		{ kind: "video", bytes: [1, 2, 3, 255] },
		{ kind: "audio", bytes: [0.25, -0.5] },
	]);
	codecs.cancelEncoder(id);
	codecs.cancelEncoder(id);
	assert.ok(state.instances.every((instance) => instance.state === "closed"));
});

test("finishing failures close encoders and cannot return a partial export", async (t) => {
	const state = platform(t, { videoFlush: true });
	const id = await codecs.createEncoder(16, 16, 30, 48000, 2);
	await assert.rejects(codecs.finishEncoder(id), /video flush failed/);
	assert.ok(state.instances.every((instance) => instance.state === "closed"));
	await assert.rejects(codecs.finishEncoder(id), /no longer active/);
});

function output(encoder, timestamp, duration, description = Uint8Array.of(1)) {
	encoder.callbacks.output(
		{
			byteLength: 1,
			timestamp,
			duration,
			type: "key",
			copyTo(bytes) {
				bytes[0] = 1;
			},
		},
		{ decoderConfig: { description } },
	);
}

test("dropped video frames cannot produce a shortened successful export", async (t) => {
	const state = platform(t);
	const id = await codecs.createEncoder(1, 1, 30, 48000, 0);
	await codecs.encodeFrame(id, new Uint8Array(4), 1, 1, 0, 33333);
	await codecs.encodeFrame(id, new Uint8Array(4), 1, 1, 33333, 33334);
	output(state.instances[0], 0, 33333);
	await assert.rejects(codecs.finishEncoder(id), /dropped video frames/);
	assert.equal(state.instances[0].state, "closed");
});

test("audio gaps fail export while AAC priming and end padding are accepted", async (t) => {
	const state = platform(t);
	let id = await codecs.createEncoder(1, 1, 30, 48000, 2);
	await codecs.encodeFrame(id, new Uint8Array(4), 1, 1, 0, 33333);
	await codecs.encodeAudio(id, new Float32Array(3200), 1600, 0);
	output(state.instances[0], 0, 33333);
	output(state.instances[1], -21333, 21333);
	output(state.instances[1], 0, 21333);
	output(state.instances[1], 21333, 21333);
	const exported = await codecs.finishEncoder(id);
	assert.equal(exported.video.length, 1);
	assert.equal(exported.audio.length, 3);
	const next = state.instances.length;
	id = await codecs.createEncoder(1, 1, 30, 48000, 2);
	await codecs.encodeFrame(id, new Uint8Array(4), 1, 1, 0, 33333);
	await codecs.encodeAudio(id, new Float32Array(3200), 1600, 0);
	output(state.instances[next], 0, 33333);
	output(state.instances[next + 1], 0, 21333);
	output(state.instances[next + 1], 42666, 21333);
	await assert.rejects(codecs.finishEncoder(id), /incomplete audio timing/);
	assert.ok(state.instances.every((instance) => instance.state === "closed"));
});

test("implicit calibration trimming fails rather than guessing an audio offset", async (t) => {
	const state = platform(t, { calibrationTrim: 128 });
	const id = await codecs.createEncoder(1, 1, 30, 48000, 2);
	await codecs.encodeFrame(id, new Uint8Array(4), 1, 1, 0, 33333);
	await codecs.encodeAudio(id, new Float32Array(3200), 1600, 0);
	output(state.instances[0], 0, 33333);
	output(state.instances[1], 0, 42667);
	await assert.rejects(
		codecs.finishEncoder(id),
		/trims audio calibration samples/,
	);
	assert.ok(state.instances.every((instance) => instance.state === "closed"));
	assert.ok(state.contexts.every((context) => context.closed));
});

test("calibration reports a measured nonzero delay for the MP4 edit", async (t) => {
	const state = platform(t, { calibrationPriming: 2112 });
	const id = await codecs.createEncoder(1, 1, 30, 48000, 2);
	await codecs.encodeFrame(id, new Uint8Array(4), 1, 1, 0, 33333);
	await codecs.encodeAudio(id, new Float32Array(3200), 1600, 0);
	output(state.instances[0], 0, 33333);
	output(state.instances[1], 0, 100000);
	const exported = await codecs.finishEncoder(id);
	assert.equal(exported.audioPriming, 2112);
	assert.ok(state.instances.every((instance) => instance.state === "closed"));
	assert.ok(state.contexts.every((context) => context.closed));
});

for (const [failure, expected] of [
	["calibrationNoise", /could not be synchronized reliably/],
	["calibrationDescription", /changed its AAC configuration/],
]) {
	test(`${failure} rejects export and releases calibration resources`, async (t) => {
		const state = platform(t, { [failure]: true });
		const id = await codecs.createEncoder(1, 1, 30, 48000, 2);
		await codecs.encodeFrame(id, new Uint8Array(4), 1, 1, 0, 33333);
		await codecs.encodeAudio(id, new Float32Array(3200), 1600, 0);
		output(state.instances[0], 0, 33333);
		output(state.instances[1], 0, 100000);
		await assert.rejects(codecs.finishEncoder(id), expected);
		assert.ok(state.instances.every((instance) => instance.state === "closed"));
		assert.ok(state.contexts.every((context) => context.closed));
	});
}
