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
