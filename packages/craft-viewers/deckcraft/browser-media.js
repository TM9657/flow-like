const assets = new Map();
const voices = new Map();
const posterRequests = new Map();
const posterResults = [];
let context;
let unlocked = false;
let repaint = () => {};
export function setRepaint(callback) {
	repaint = callback;
}

function message(error) {
	return error?.name === "NotAllowedError"
		? "Playback needs a click in the editor. Press Play again."
		: "This browser or operating system cannot play this media format.";
}

function mime(bytes) {
	const text = (start, length) =>
		String.fromCharCode(...bytes.subarray(start, start + length));
	if (
		text(4, 4) === "ftyp" ||
		["moov", "mdat", "wide", "free"].includes(text(4, 4))
	)
		return "video/mp4";
	if (bytes[0] === 0x1a && bytes[1] === 0x45) return "video/webm";
	if (text(0, 4) === "OggS") return "audio/ogg";
	if (["RIFF", "RF64"].includes(text(0, 4))) return "audio/wav";
	if (text(0, 4) === "fLaC") return "audio/flac";
	if (text(0, 4) === "FORM") return "audio/aiff";
	if (text(0, 4) === "caff") return "audio/x-caf";
	if (bytes[0] === 0xff && (bytes[1] & 0xf6) === 0xf0) return "audio/aac";
	if (text(0, 3) === "ID3" || (bytes[0] === 0xff && (bytes[1] & 0xe0) === 0xe0))
		return "audio/mpeg";
	return "application/octet-stream";
}

function audioContext() {
	const Constructor = globalThis.AudioContext || globalThis.webkitAudioContext;
	if (!context && Constructor) context = new Constructor();
	return context;
}

function unlock() {
	unlocked = true;
	try {
		audioContext()
			?.resume()
			.catch(() => {});
	} catch {}
	for (const voice of voices.values()) {
		if (voice.wantsPlay && voice.blocked) start(voice);
	}
}
if (typeof document !== "undefined") {
	document.addEventListener("pointerdown", unlock, true);
	document.addEventListener("keydown", unlock, true);
}

function waitFor(element, event, ready) {
	if (ready()) return Promise.resolve();
	return new Promise((resolve, reject) => {
		const done = (error) => {
			clearTimeout(timer);
			element.removeEventListener(event, success);
			element.removeEventListener("error", failure);
			error ? reject(error) : resolve();
		};
		const success = () => done();
		const failure = () => done(element.error || new Error("Unsupported media"));
		const timer = setTimeout(
			() => done(new Error("Media loading timed out")),
			30000,
		);
		element.addEventListener(event, success, { once: true });
		element.addEventListener("error", failure, { once: true });
	});
}

function mediaElement(asset, muted = true) {
	const element = document.createElement("video");
	element.playsInline = true;
	element.preload = "auto";
	element.muted = muted;
	element.src = asset.url;
	return element;
}

function releaseElement(element) {
	element.pause();
	element.removeAttribute("src");
	element.load();
}

function prune() {
	let total = [...assets.values()].reduce((sum, asset) => sum + asset.size, 0);
	for (const [key, asset] of assets) {
		if (assets.size <= 16 && total <= 128 * 1024 * 1024) break;
		if (
			asset.busy ||
			[...voices.values()].some((voice) => voice.asset === asset)
		)
			continue;
		releaseElement(asset.element);
		URL.revokeObjectURL(asset.url);
		assets.delete(key);
		total -= asset.size;
	}
}

export function hasAsset(key) {
	return assets.has(key);
}
export function register(key, bytes) {
	if (assets.has(key)) return;
	const type = mime(bytes);
	const asset = {
		key,
		type,
		size: bytes.length,
		url: URL.createObjectURL(new Blob([bytes], { type })),
		posters: new Map(),
		busy: 1,
		error: null,
	};
	asset.element = mediaElement(asset);
	asset.ready = waitFor(
		asset.element,
		"loadeddata",
		() => asset.element.readyState >= 2,
	)
		.then(() => {
			const element = asset.element;
			asset.info = {
				duration: Number.isFinite(element.duration) ? element.duration : 0,
				width: element.videoWidth || 0,
				height: element.videoHeight || 0,
			};
			asset.busy--;
			repaint();
		})
		.catch((error) => {
			asset.error = message(error);
			asset.busy--;
			repaint();
		});
	assets.set(key, asset);
	prune();
}

export async function prepare(key) {
	const asset = assets.get(key);
	await asset?.ready;
	if (asset?.info?.width) {
		try {
			await poster(key, 0);
		} catch {}
	}
}

export function probe(key) {
	const asset = assets.get(key);
	if (!asset) return { error: "The media is unavailable." };
	if (asset.error) return { error: asset.error };
	return asset.info ? { ...asset.info, type: asset.type } : { pending: true };
}

function bounds(voice) {
	const duration = Number.isFinite(voice.element.duration)
		? voice.element.duration
		: voice.spec.duration;
	let start = Math.max(0, voice.spec.trimStart);
	let end =
		duration > 0
			? Math.max(0, duration - voice.spec.trimEnd)
			: Number.POSITIVE_INFINITY;
	if (end - start < 0.01) {
		start = 0;
		end = duration;
	}
	return { start, end, duration: duration || 0 };
}

function configureAudio(voice) {
	try {
		const audio = audioContext();
		if (audio && !voice.gain) {
			voice.node = audio.createMediaElementSource(voice.element);
			voice.gain = audio.createGain();
			voice.node.connect(voice.gain).connect(audio.destination);
		}
		if (unlocked) audio?.resume().catch(() => {});
	} catch {}
}

function start(voice) {
	voice.blocked = false;
	voice.error = null;
	voice.state = "loading";
	configureAudio(voice);
	applyVolume(voice);
	const attempt = ++voice.attempt;
	voice.element
		.play()
		.then(() => {
			if (voice.attempt !== attempt || !voice.wantsPlay) return;
			voice.state = "playing";
			repaint();
		})
		.catch((error) => {
			if (voice.attempt !== attempt) return;
			voice.blocked = error?.name === "NotAllowedError";
			voice.error = message(error);
			voice.state = "paused";
			repaint();
		});
}

function voiceFor(key, assetKey) {
	let voice = voices.get(key);
	const asset = assets.get(assetKey);
	if (!asset) return null;
	if (voice && voice.asset !== asset) {
		stop(key);
		voice = null;
	}
	if (!voice) {
		const element = mediaElement(asset, false);
		voice = {
			asset,
			element,
			state: "paused",
			error: null,
			wantsPlay: false,
			blocked: false,
			attempt: 0,
			dirty: true,
			lastTime: -1,
			pendingSeek: null,
			spec: {
				trimStart: 0,
				trimEnd: 0,
				fadeIn: 0,
				fadeOut: 0,
				volume: 1,
				looping: false,
				duration: 0,
			},
		};
		if (typeof element.requestVideoFrameCallback === "function") {
			const onFrame = () => {
				if (voices.get(key) !== voice) return;
				voice.dirty = true;
				repaint();
				voice.frameCallback = element.requestVideoFrameCallback(onFrame);
			};
			voice.frameCallback = element.requestVideoFrameCallback(onFrame);
		}
		element.addEventListener("error", () => {
			voice.error = message(element.error);
			voice.state = "ended";
			voice.wantsPlay = false;
			repaint();
		});
		element.addEventListener("loadedmetadata", () => {
			if (voice.pendingSeek !== null) {
				element.currentTime = voice.pendingSeek;
				voice.pendingSeek = null;
			}
			repaint();
		});
		element.addEventListener("seeked", () => {
			voice.dirty = true;
			repaint();
		});
		element.addEventListener("ended", () => {
			if (voice.spec.looping && voice.wantsPlay) {
				element.currentTime = bounds(voice).start;
				start(voice);
			} else {
				voice.state = "ended";
				voice.wantsPlay = false;
			}
			repaint();
		});
		voices.set(key, voice);
	}
	return voice;
}

export function attach(key, assetKey) {
	voiceFor(key, assetKey);
}
export function play(key, assetKey, options) {
	const voice = voiceFor(key, assetKey);
	if (!voice) return;
	voice.spec = JSON.parse(options);
	const from = voice.spec.from;
	const resume =
		from === null && voice.state === "paused" && voice.element.currentTime > 0;
	if (!resume) seek(key, from ?? voice.spec.trimStart);
	voice.wantsPlay = true;
	start(voice);
}
export function pause(key) {
	const voice = voices.get(key);
	if (!voice) return;
	voice.attempt++;
	voice.wantsPlay = false;
	voice.element.pause();
	voice.state = "paused";
}
export function stop(key) {
	const voice = voices.get(key);
	if (!voice) return;
	voice.attempt++;
	voice.wantsPlay = false;
	if (voice.frameCallback !== undefined) {
		voice.element.cancelVideoFrameCallback?.(voice.frameCallback);
	}
	releaseElement(voice.element);
	voice.node?.disconnect();
	voice.gain?.disconnect();
	voices.delete(key);
	prune();
}
export function seek(key, time) {
	const voice = voices.get(key);
	if (!voice) return;
	const { start, end } = bounds(voice);
	const target = Math.max(
		start,
		Math.min(Number.isFinite(time) ? time : start, end),
	);
	if (voice.element.readyState >= 1) voice.element.currentTime = target;
	else voice.pendingSeek = target;
	voice.dirty = true;
}
export function status(key) {
	const voice = voices.get(key);
	if (!voice) return null;
	return {
		state: voice.state,
		position: voice.pendingSeek ?? voice.element.currentTime,
		duration: bounds(voice).duration,
		error: voice.error,
	};
}
export function tick(key) {
	const voice = voices.get(key);
	if (!voice) return;
	const { start: begin, end } = bounds(voice);
	const time = voice.element.currentTime;
	if (voice.wantsPlay && time >= end) {
		if (voice.spec.looping) {
			seek(key, begin);
			if (voice.element.paused) start(voice);
		} else {
			pause(key);
			voice.state = "ended";
		}
	}
	applyVolume(voice);
}

function applyVolume(voice) {
	const { start: begin, end } = bounds(voice);
	const time = voice.pendingSeek ?? voice.element.currentTime;
	const half = (end - begin) / 2;
	const fadeIn = Math.min(voice.spec.fadeIn, half);
	const fadeOut = Math.min(voice.spec.fadeOut, half);
	const ramp = (value) => Math.max(0, Math.min(1, value));
	const volume =
		voice.spec.volume *
		(fadeIn > 0 ? ramp((time - begin) / fadeIn) : 1) *
		(fadeOut > 0 ? ramp((end - time) / fadeOut) : 1);
	voice.element.muted = voice.spec.volume <= 0;
	if (voice.gain) {
		voice.element.volume = 1;
		voice.gain.gain.value = Math.max(0, volume);
	} else voice.element.volume = Math.max(0, Math.min(1, volume));
}

function capture(element, surface) {
	if (element.readyState < 2 || !element.videoWidth || element.seeking)
		return null;
	const scale = Math.min(
		1,
		2048 / Math.max(element.videoWidth, element.videoHeight),
	);
	const width = Math.max(1, Math.round(element.videoWidth * scale));
	const height = Math.max(1, Math.round(element.videoHeight * scale));
	const canvas = surface?.canvas || document.createElement("canvas");
	if (canvas.width !== width) canvas.width = width;
	if (canvas.height !== height) canvas.height = height;
	const ctx =
		surface?.ctx || canvas.getContext("2d", { willReadFrequently: true });
	if (!ctx) throw new Error("Canvas frame capture is unavailable.");
	ctx.drawImage(element, 0, 0, width, height);
	return { canvas, ctx, width, height };
}
export function frame(key) {
	const voice = voices.get(key);
	if (
		!voice ||
		(!voice.dirty &&
			(voice.frameCallback !== undefined ||
				voice.lastTime === voice.element.currentTime))
	)
		return null;
	try {
		const result = capture(voice.element, voice.surface);
		if (!result) return null;
		voice.surface = result;
		voice.dirty = false;
		voice.lastTime = voice.element.currentTime;
		return {
			time: voice.lastTime,
			width: result.width,
			height: result.height,
			rgba: result.ctx.getImageData(0, 0, result.width, result.height).data,
		};
	} catch (error) {
		voice.error = String(error.message || error);
		return null;
	}
}

function posterKey(time) {
	return Math.max(0, Math.round(time * 1000));
}
async function poster(assetKey, time) {
	const asset = assets.get(assetKey);
	if (!asset) throw new Error("The media is unavailable.");
	asset.busy++;
	let element;
	try {
		await asset.ready;
		if (asset.error) throw new Error(asset.error);
		const key = posterKey(time);
		if (asset.posters.has(key)) return asset.posters.get(key);
		element = mediaElement(asset);
		await waitFor(element, "loadeddata", () => element.readyState >= 2);
		const target = Math.min(
			key / 1000,
			Number.isFinite(element.duration)
				? Math.max(0, element.duration - 0.001)
				: key / 1000,
		);
		if (Math.abs(element.currentTime - target) > 0.001) {
			element.currentTime = target;
			await waitFor(
				element,
				"seeked",
				() => !element.seeking && Math.abs(element.currentTime - target) < 0.01,
			);
		}
		const result = capture(element);
		if (!result) throw new Error("This media has no decodable video frame.");
		const blob = await new Promise((resolve, reject) =>
			result.canvas.toBlob(
				(value) =>
					value
						? resolve(value)
						: reject(new Error("The poster frame could not be saved.")),
				"image/png",
			),
		);
		const bytes = new Uint8Array(await blob.arrayBuffer());
		if (asset.posters.size >= 8)
			asset.posters.delete(asset.posters.keys().next().value);
		asset.posters.set(key, bytes);
		return bytes;
	} finally {
		if (element) releaseElement(element);
		asset.busy--;
	}
}
export function posterBytes(assetKey, time) {
	return assets.get(assetKey)?.posters.get(posterKey(time)) || null;
}
export function requestPoster(shape, assetKey, time) {
	const key = `${shape}:${assetKey}:${posterKey(time)}`;
	if (posterRequests.has(key)) return;
	posterRequests.set(key, true);
	poster(assetKey, time)
		.then(() => posterResults.push({ shape, ms: posterKey(time) }))
		.catch((error) =>
			posterResults.push({ shape, error: String(error.message || error) }),
		)
		.finally(() => {
			posterRequests.delete(key);
			repaint();
		});
}
export function takePoster() {
	return posterResults.shift() || null;
}
export function pendingPosters() {
	return posterRequests.size > 0;
}
