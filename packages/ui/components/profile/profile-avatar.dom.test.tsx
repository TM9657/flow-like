// @vitest-environment happy-dom

import { type ComponentProps, act } from "react";
import { type Root, createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProfileAvatar } from "./profile-avatar";

type Profile = NonNullable<ComponentProps<typeof ProfileAvatar>["profile"]>;

const firstIcon = "https://media.example/profile.webp?signature=first";
const secondIcon = "https://media.example/profile.webp?signature=second";
const thirdIcon = "https://media.example/profile.webp?signature=third";
const fallback = "/app-logo.webp";
const profile: Profile = {
	id: "profile-a",
	hub: "hub.example",
	icon: firstIcon,
};

class ControlledImage {
	static requests: ControlledImage[] = [];
	static cached = new Set<string>();
	onload: (() => void) | null = null;
	onerror: (() => void) | null = null;
	complete = false;
	naturalWidth = 0;
	private source = "";

	constructor() {
		ControlledImage.requests.push(this);
	}

	set src(value: string) {
		this.source = value;
		if (ControlledImage.cached.has(value)) {
			this.complete = true;
			this.naturalWidth = 32;
		}
	}

	get src() {
		return this.source;
	}

	load() {
		this.complete = true;
		this.naturalWidth = 32;
		this.onload?.();
	}

	fail() {
		this.complete = true;
		this.naturalWidth = 0;
		this.onerror?.();
	}
}

let root: Root;
let container: HTMLDivElement;
let originalImage: typeof window.Image;

beforeEach(() => {
	vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
	ControlledImage.requests = [];
	ControlledImage.cached.clear();
	originalImage = window.Image;
	vi.stubGlobal("Image", ControlledImage);
	window.Image = ControlledImage as unknown as typeof window.Image;
	container = document.createElement("div");
	document.body.append(container);
	root = createRoot(container);
});

afterEach(async () => {
	await act(async () => root.unmount());
	container.remove();
	vi.unstubAllGlobals();
	window.Image = originalImage;
});

async function renderProfile(next: Profile | null | undefined = profile) {
	await act(async () => {
		root.render(<ProfileAvatar profile={next} className="rounded-lg" />);
	});
}

function image() {
	const images = container.querySelectorAll("img");
	expect(images).toHaveLength(1);
	return images[0];
}

function request(src: string) {
	const pending = ControlledImage.requests.findLast(
		(image) => image.src === src,
	);
	if (!pending) throw new Error(`Image was not requested: ${src}`);
	return pending;
}

async function load(src: string) {
	await act(async () => request(src).load());
}

describe("profile avatar", () => {
	it("keeps a loaded icon mounted without requesting it again on ordinary rerenders", async () => {
		await renderProfile();
		expect(image().getAttribute("src")).toBe(fallback);
		await load(firstIcon);
		const loaded = image();
		expect(loaded.getAttribute("src")).toBe(firstIcon);
		expect(loaded.alt).toBe("");
		const requestCount = ControlledImage.requests.length;

		await renderProfile({ ...profile });
		await renderProfile({ ...profile });
		expect(image()).toBe(loaded);
		expect(image().getAttribute("src")).toBe(firstIcon);
		expect(ControlledImage.requests).toHaveLength(requestCount);
	});

	it("keeps the current icon visible while its refreshed URL loads", async () => {
		await renderProfile();
		await load(firstIcon);
		const loaded = image();

		await renderProfile({ ...profile, icon: secondIcon });
		expect(request(secondIcon).complete).toBe(false);
		expect(image()).toBe(loaded);
		expect(image().getAttribute("src")).toBe(firstIcon);

		await load(secondIcon);
		expect(image().getAttribute("src")).toBe(secondIcon);
	});

	it.each([{ id: "profile-b" }, { hub: "another-hub.example" }])(
		"clears the previous icon when the profile identity changes: %o",
		async (identity) => {
			await renderProfile();
			await load(firstIcon);

			await renderProfile({ ...profile, ...identity, icon: secondIcon });
			expect(image().getAttribute("src")).toBe(fallback);
			await load(secondIcon);
			expect(image().getAttribute("src")).toBe(secondIcon);
		},
	);

	it.each([null, undefined, ""])(
		"uses the fallback when the icon is removed: %s",
		async (icon) => {
			await renderProfile();
			await load(firstIcon);

			await renderProfile({ ...profile, icon });
			expect(image().getAttribute("src")).toBe(fallback);
		},
	);

	it("uses the fallback when the profile is unavailable", async () => {
		await renderProfile();
		await load(firstIcon);

		await renderProfile(null);
		expect(image().getAttribute("src")).toBe(fallback);
	});

	it("falls back after a failed image and recovers when a later URL loads", async () => {
		await renderProfile();
		await act(async () => request(firstIcon).fail());
		expect(image().getAttribute("src")).toBe(fallback);

		await renderProfile({ ...profile, icon: secondIcon });
		await load(secondIcon);
		expect(image().getAttribute("src")).toBe(secondIcon);

		await renderProfile({ ...profile, icon: thirdIcon });
		expect(image().getAttribute("src")).toBe(secondIcon);
		await act(async () => request(thirdIcon).fail());
		expect(image().getAttribute("src")).toBe(fallback);
	});

	it("ignores late success and failure callbacks from a superseded URL", async () => {
		await renderProfile();
		await load(firstIcon);
		await renderProfile({ ...profile, icon: secondIcon });
		const staleLoad = request(secondIcon).onload;
		const staleError = request(secondIcon).onerror;

		await renderProfile({ ...profile, icon: thirdIcon });
		await act(async () => staleLoad?.());
		expect(image().getAttribute("src")).toBe(firstIcon);
		await load(thirdIcon);
		await act(async () => {
			staleLoad?.();
			staleError?.();
		});
		expect(image().getAttribute("src")).toBe(thirdIcon);
	});

	it("does not restore an icon from an in-flight request after removal", async () => {
		await renderProfile();
		await load(firstIcon);
		await renderProfile({ ...profile, icon: secondIcon });
		const staleLoad = request(secondIcon).onload;

		await renderProfile({ ...profile, icon: null });
		await act(async () => staleLoad?.());
		expect(image().getAttribute("src")).toBe(fallback);
	});

	it("shows an already cached icon without waiting for another load event", async () => {
		ControlledImage.cached.add(firstIcon);
		await renderProfile();
		expect(image().getAttribute("src")).toBe(firstIcon);
	});
});
