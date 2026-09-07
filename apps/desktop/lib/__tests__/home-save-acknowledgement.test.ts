import type { IProfile } from "@flow-like/flow-like-ui";
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TauriBackend } from "../../components/tauri-provider";
import { UserState } from "../../components/tauri-provider/user-state";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../api", () => ({ fetcher: vi.fn() }));
vi.mock("../apps-db", () => ({ appsDB: {} }));
vi.mock("../notifications-db", () => ({}));

describe("desktop Home save acknowledgement", () => {
	const dispatchEvent = vi.fn();
	const userState = new UserState({} as TauriBackend);
	const layout = { version: 1 as const, widgets: [] };
	const saved: IProfile = {
		id: "personal",
		name: "Personal",
		bits: [],
		created: "2026-09-07T10:00:00Z",
		home_layout: layout,
		updated: "2026-09-07T11:00:00.001Z",
	};

	beforeEach(() => {
		vi.clearAllMocks();
		vi.stubGlobal("window", { dispatchEvent });
	});
	afterEach(() => vi.unstubAllGlobals());

	it("returns the committed snapshot before requesting synchronization", async () => {
		let acknowledge!: (profile: IProfile) => void;
		vi.mocked(invoke).mockReturnValueOnce(
			new Promise<IProfile>((resolve) => {
				acknowledge = resolve;
			}),
		);
		const saving = userState.saveHomeLayout(layout, "personal");
		expect(dispatchEvent).not.toHaveBeenCalled();
		acknowledge(saved);
		await expect(saving).resolves.toBe(saved);
		expect(invoke).toHaveBeenCalledExactlyOnceWith(
			"profile_update_home_layout",
			{ profileId: "personal", layout },
		);
		expect(dispatchEvent.mock.calls[0][0].type).toBe("flow-like:profile-sync");
	});

	it.each([undefined, null, { ...saved, id: "other-profile" }])(
		"rejects an old or mismatched native response: %j",
		async (response) => {
			vi.mocked(invoke).mockResolvedValueOnce(response);
			await expect(
				userState.saveHomeLayout(layout, "personal"),
			).rejects.toThrow("Restart or update the app");
			expect(dispatchEvent).not.toHaveBeenCalled();
		},
	);

	it("propagates a disk write failure without requesting synchronization", async () => {
		vi.mocked(invoke).mockRejectedValueOnce(new Error("Disk is full"));
		await expect(userState.saveHomeLayout(layout, "personal")).rejects.toThrow(
			"Disk is full",
		);
		expect(dispatchEvent).not.toHaveBeenCalled();
	});
});
