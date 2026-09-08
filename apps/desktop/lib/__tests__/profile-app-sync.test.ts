import type { IProfileApp } from "@flow-like/flow-like-ui";
import { describe, expect, it } from "vitest";
import {
	type OnlineProfile,
	mergeRemoteProfileMetadata,
	toLocalProfile,
} from "../profile-sync";

const remote: OnlineProfile = {
	id: "personal",
	name: "Personal",
	hub: "https://hub.example",
	created_at: "2026-09-05T10:00:00Z",
	updated_at: "2026-09-05T10:01:00Z",
	apps: [],
};

function app(
	appId: string,
	preferences: Partial<IProfileApp> = {},
): IProfileApp {
	return {
		app_id: appId,
		favorite: false,
		favorite_order: null,
		pinned: false,
		pinned_order: null,
		...preferences,
	};
}

describe("profile app synchronization", () => {
	it("keeps offline membership and preferences through acknowledgement, reload, and repeated pulls", () => {
		const offline = app("offline", {
			favorite: true,
			favorite_order: 3,
			pinned: true,
			pinned_order: 1,
		});
		const local = toLocalProfile({ ...remote, apps: [offline] });
		const offlineIds = new Set([offline.app_id]);

		mergeRemoteProfileMetadata(local, remote, false, offlineIds);
		expect(local.hub_profile.apps).toEqual([offline]);
		expect(local.hub_profile.updated).toBe(remote.updated_at);

		const reloaded = JSON.parse(JSON.stringify(local)) as typeof local;
		for (let pull = 0; pull < 2; pull++) {
			mergeRemoteProfileMetadata(reloaded, remote, false, offlineIds);
		}
		expect(reloaded.hub_profile.apps).toEqual([offline]);
	});

	it("accepts online app updates and removals while retaining offline apps", () => {
		const offline = app("offline", { pinned: true, pinned_order: 2 });
		const updated = app("online", { favorite: true, favorite_order: 0 });
		const added = app("new-online");
		const local = toLocalProfile({
			...remote,
			apps: [app("removed-online"), offline, app("online")],
		});

		mergeRemoteProfileMetadata(
			local,
			{ ...remote, apps: [updated, added] },
			false,
			new Set([offline.app_id]),
		);

		expect(local.hub_profile.apps).toEqual([updated, added, offline]);
	});

	it("uses local offline membership and preferences when the server has stale entries", () => {
		const offline = app("offline", { favorite: true, favorite_order: 4 });
		const local = toLocalProfile({ ...remote, apps: [offline] });
		const staleRemote = {
			...remote,
			apps: [app("offline"), app("removed-offline"), app("other-profile-app")],
		};
		const offlineIds = new Set([
			"offline",
			"removed-offline",
			"other-profile-app",
		]);

		mergeRemoteProfileMetadata(local, staleRemote, false, offlineIds);
		expect(local.hub_profile.apps).toEqual([offline]);

		local.hub_profile.apps = [];
		mergeRemoteProfileMetadata(local, staleRemote, false, offlineIds);
		expect(local.hub_profile.apps).toEqual([]);
	});

	it("retains offline apps when the remote app list is omitted", () => {
		const offline = app("offline");
		const local = toLocalProfile({
			...remote,
			apps: [app("online"), offline],
		});

		mergeRemoteProfileMetadata(
			local,
			{ ...remote, apps: undefined },
			false,
			new Set([offline.app_id]),
		);

		expect(local.hub_profile.apps).toEqual([offline]);
	});
});
