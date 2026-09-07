import { afterEach, describe, expect, it, vi } from "vitest";
import {
	type OnlineProfile,
	createProfileSyncQueue,
	mergeRemoteProfileMetadata,
	toLocalProfile,
} from "../profile-sync";

const profile: OnlineProfile = {
	id: "personal",
	name: "Personal",
	hub: "https://hub.example",
	created_at: "2026-09-05T10:00:00Z",
	updated_at: "2026-09-05T10:01:00Z",
};

describe("profile home synchronization", () => {
	it("preserves the selected default and embedded route query parameters", () => {
		const home_layout = {
			version: 1 as const,
			widgets: [
				{
					id: "embed",
					type: "app-embed",
					size: { columns: 6, rows: 4 },
					appearance: { variant: "card", accent: "neutral" },
					config: { appId: "app", path: "/reports?status=open&tag=a&tag=b" },
				},
			],
		};
		const local = toLocalProfile({
			...profile,
			home_layout,
			home_default_id: "template",
		});
		expect(local.hub_profile.home_layout).toEqual(home_layout);
		expect(local.hub_profile.home_default_id).toBe("template");
	});

	it("keeps explicit reset null while preserving the profile default association", () => {
		const local = toLocalProfile({
			...profile,
			home_layout: null,
			home_default_id: "template",
		});
		expect(local.hub_profile.home_layout).toBeNull();
		expect(local.hub_profile.home_default_id).toBe("template");
	});

	it("old profiles inherit the main default without creating a custom layout", () => {
		const local = toLocalProfile(profile);
		expect(local.hub_profile.home_layout).toBeNull();
		expect(local.hub_profile.home_default_id).toBeNull();
	});

	it("pulls another device's layout and later reset without losing default lineage", () => {
		const local = toLocalProfile(profile);
		const remote = {
			...profile,
			home_layout: { version: 1 as const, widgets: [] },
			home_default_id: "team-default",
		};
		mergeRemoteProfileMetadata(local, remote);
		expect(local.hub_profile.home_layout).toEqual(remote.home_layout);
		mergeRemoteProfileMetadata(local, { ...remote, home_layout: null });
		expect(local.hub_profile.home_layout).toBeNull();
		expect(local.hub_profile.home_default_id).toBe("team-default");
	});

	it("keeps a locally saved layout when the hub response omits the home fields", () => {
		const home_layout = { version: 1 as const, widgets: [] };
		const local = toLocalProfile({
			...profile,
			home_layout,
			home_default_id: "template",
		});
		mergeRemoteProfileMetadata(local, {
			...profile,
			updated_at: "2026-09-05T10:02:00Z",
		});
		expect(local.hub_profile.home_layout).toEqual(home_layout);
		expect(local.hub_profile.home_default_id).toBe("template");
		expect(local.hub_profile.updated).toBe("2026-09-05T10:02:00Z");
	});
});

describe("profile sync queue", () => {
	afterEach(() => vi.useRealTimers());

	it("sends edits saved during an active sync in a follow-up without overlapping requests", async () => {
		const queue = createProfileSyncQueue();
		let release!: () => void;
		const firstRequest = new Promise<void>((resolve) => {
			release = resolve;
		});
		const sync = vi
			.fn()
			.mockReturnValueOnce(firstRequest)
			.mockResolvedValue(undefined);
		const active = queue.request(sync);
		await Promise.resolve();
		expect(sync).toHaveBeenCalledTimes(1);
		queue.request(sync, true);
		queue.request(sync, true);
		expect(sync).toHaveBeenCalledTimes(1);
		release();
		await active;
		expect(sync).toHaveBeenCalledTimes(2);
	});

	it("throttles background refreshes while saving bypasses the cooldown", async () => {
		vi.useFakeTimers();
		const queue = createProfileSyncQueue(60_000);
		const sync = vi.fn().mockResolvedValue(undefined);
		await queue.request(sync);
		await queue.request(sync);
		expect(sync).toHaveBeenCalledTimes(1);
		await queue.request(sync, true);
		expect(sync).toHaveBeenCalledTimes(2);
		vi.advanceTimersByTime(60_000);
		await queue.request(sync);
		expect(sync).toHaveBeenCalledTimes(3);
	});

	it("cancels a queued request when its profile sync context is removed", async () => {
		const queue = createProfileSyncQueue();
		let release!: () => void;
		const sync = vi.fn(
			() =>
				new Promise<void>((resolve) => {
					release = resolve;
				}),
		);
		const active = queue.request(sync);
		await Promise.resolve();
		queue.request(sync, true);
		queue.cancel(sync);
		release();
		await active;
		expect(sync).toHaveBeenCalledTimes(1);
	});
});
