let installed = false;

export function installAudioUnlock() {
	if (installed) return;
	const OriginalContext = window.AudioContext || window.webkitAudioContext;
	if (!OriginalContext) return;
	installed = true;
	const contexts = new Set();
	class Context extends OriginalContext {
		constructor(options) {
			super(options);
			contexts.add(this);
		}
		close() {
			contexts.delete(this);
			return super.close();
		}
	}
	window.AudioContext = Context;
	if (window.webkitAudioContext) window.webkitAudioContext = Context;

	// CPAL creates its context before a gesture. Resume it from the canvas event,
	// as upstream's standalone HTML does, so playback can advance its audio clock.
	const resume = () => {
		for (const context of contexts) {
			if (context.state === "closed") contexts.delete(context);
			else if (context.state !== "running")
				void context.resume().catch(() => {});
		}
	};
	for (const type of ["pointerdown", "mousedown", "touchstart", "keydown"])
		window.addEventListener(type, resume, { capture: true, passive: true });
}

// wasm-bindgen caches the constructor when craft.js loads, before Rust starts.
installAudioUnlock();

const MAX_INPUT_BYTES = 100 * 1024 * 1024;
const MAX_PCM_BYTES = 256 * 1024 * 1024;

// Web Audio delegates compressed formats to the browser/OS. Only PCM crosses
// into the editor; there is no bundled compressed-audio decoder fallback.
export async function decodeAudio(bytes) {
	if (
		!(bytes instanceof Uint8Array) ||
		!bytes.length ||
		bytes.length > MAX_INPUT_BYTES
	)
		throw new Error("Audio files must be between 1 byte and 100 MiB.");
	const Context = window.AudioContext || window.webkitAudioContext;
	if (!Context)
		throw new Error(
			"This browser cannot decode audio. Open PCM WAV or AIFF, or use a browser with Web Audio support.",
		);
	let context;
	try {
		context = new Context({ sampleRate: 48000 });
		const audio = await context.decodeAudioData(bytes.slice().buffer);
		return pcmWave(audio);
	} catch (error) {
		if (error?.code === "PCM_LIMIT") throw error;
		throw new Error(
			"This browser could not decode this audio format. Try another browser or convert the file to PCM WAV or AIFF.",
			{ cause: error },
		);
	} finally {
		if (context && context.state !== "closed")
			await context.close().catch(() => {});
	}
}

function pcmWave(audio) {
	const channels = audio.numberOfChannels;
	const frames = audio.length;
	const rate = audio.sampleRate;
	const size = frames * channels * 4;
	if (
		!Number.isSafeInteger(channels) ||
		channels < 1 ||
		channels > 32 ||
		!Number.isSafeInteger(frames) ||
		frames < 1 ||
		!Number.isSafeInteger(rate) ||
		rate < 1 ||
		rate > 384000 ||
		!Number.isSafeInteger(size) ||
		size > MAX_PCM_BYTES
	) {
		const error = new Error(
			"The decoded audio is empty or exceeds the editor's 256 MiB PCM limit.",
		);
		error.code = "PCM_LIMIT";
		throw error;
	}
	const bytes = new Uint8Array(44 + size);
	const view = new DataView(bytes.buffer);
	const tag = (at, value) => {
		for (let i = 0; i < value.length; i++) bytes[at + i] = value.charCodeAt(i);
	};
	tag(0, "RIFF");
	view.setUint32(4, 36 + size, true);
	tag(8, "WAVEfmt ");
	view.setUint32(16, 16, true);
	view.setUint16(20, 3, true); // IEEE float PCM
	view.setUint16(22, channels, true);
	view.setUint32(24, rate, true);
	view.setUint32(28, rate * channels * 4, true);
	view.setUint16(32, channels * 4, true);
	view.setUint16(34, 32, true);
	tag(36, "data");
	view.setUint32(40, size, true);
	for (let channel = 0; channel < channels; channel++) {
		const samples = audio.getChannelData(channel);
		for (let frame = 0; frame < frames; frame++) {
			const sample = samples[frame];
			view.setFloat32(
				44 + (frame * channels + channel) * 4,
				Number.isFinite(sample) ? sample : 0,
				true,
			);
		}
	}
	return bytes;
}
