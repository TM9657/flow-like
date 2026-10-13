const platformMediaEditors = new Set(["soundcraft", "deckcraft", "filmcraft"]);

// Containers, PCM and lossless file writers remain in Rust. Compressed audio
// and video are handled by browser/OS APIs in these three editor adapters.
const softwareCodec =
	/(?:^|[-_])(?:aac|ac3|eac3|h264|h264enc|h265|hevc|av1|apv|dnx|mpeg2v|mpa|mp2|mp3|vorbis|alac|opus|prores|vp8|vp9|rav1e|dav1d|openh264|ffmpeg|x264|x265|libavcodec|libvpx|minimp3|puremp3|rmp3|lewton|audiopus|mp3lame|lame)(?:[-_]|$)/;

export function usesPlatformMedia(id) {
	return platformMediaEditors.has(id);
}

function lockedPackages(lock) {
	return lock
		.split("[[package]]")
		.slice(1)
		.map((block) => {
			const fields = ["name", "version", "source", "checksum"].map((field) => {
				const value = block.match(new RegExp(`^${field} = (".*")$`, "m"));
				return value ? JSON.parse(value[1]) : null;
			});
			if (!fields[0] || !fields[1])
				throw new Error(
					"Cannot verify the editor's Cargo.lock package identity.",
				);
			return JSON.stringify(fields);
		});
}

export function assertPinnedLockPackages(original, reconciled) {
	const pinned = new Set(lockedPackages(original));
	const resolved = lockedPackages(reconciled);
	if (!pinned.size || !resolved.length)
		throw new Error("Cannot verify an empty editor Cargo.lock.");
	for (const pkg of resolved)
		if (!pinned.has(pkg))
			throw new Error(
				`The codec adapter resolved a package outside its pinned lockfile: ${pkg}. Review and pin the dependency before building.`,
			);
}

export function assertPlatformMediaDependencies(id, tree, cargoPackage) {
	if (!usesPlatformMedia(id)) return;
	const packages = new Set();
	for (const line of tree.trim().split("\n")) {
		const match = line.match(/^([\w-]+) v[\w.+-]+(?:\s|$)/);
		if (!match) throw new Error(`Cannot check ${id} codec dependency: ${line}`);
		packages.add(match[1]);
	}
	if (!packages.has(cargoPackage))
		throw new Error(`The codec dependency graph is missing ${cargoPackage}.`);
	const bundled = [...packages].filter(
		(name) => /^symphonia(?:-|$)/.test(name) || softwareCodec.test(name),
	);
	if (bundled.length)
		throw new Error(
			`${id} must use browser/OS media codecs. Remove bundled software codec dependencies before distributing it: ${bundled.sort().join(", ")}`,
		);
}
