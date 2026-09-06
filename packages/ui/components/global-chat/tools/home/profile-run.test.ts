import { describe, expect, test } from "vitest";
import {
	HomeProfileRunError,
	HomeProfileRuns,
	assertHomeProfileRun,
} from "./profile-run";

describe("Home profile run ownership", () => {
	test("rechecks the live editor after an awaited profile read before staging", async () => {
		const runs = new HomeProfileRuns();
		runs.begin("parent", "profile-a");
		let surfaceProfile = "profile-a";
		await expect(
			assertHomeProfileRun(
				runs,
				{ parentRequestId: "parent" },
				async () => {
					surfaceProfile = "profile-b";
					return "profile-a";
				},
				() => surfaceProfile,
				"profile-a",
			),
		).rejects.toBeInstanceOf(HomeProfileRunError);
	});

	test("rejects a foreign captured snapshot even after switching back", async () => {
		const runs = new HomeProfileRuns();
		runs.begin("parent", "profile-a");
		await expect(
			assertHomeProfileRun(
				runs,
				{ parentRequestId: "parent" },
				async () => "profile-a",
				() => undefined,
				"profile-b",
			),
		).rejects.toBeInstanceOf(HomeProfileRunError);
	});

	test("does not revive a run completed during an asynchronous profile read", async () => {
		const runs = new HomeProfileRuns();
		runs.begin("parent", "profile-a");
		await expect(
			assertHomeProfileRun(
				runs,
				{ parentRequestId: "parent", profileId: "profile-a" },
				async () => {
					runs.finish("parent");
					return "profile-a";
				},
				() => "profile-a",
			),
		).rejects.toBeInstanceOf(HomeProfileRunError);
	});
	test("pins nested context, discovery, validation, and apply to the parent profile", () => {
		const runs = new HomeProfileRuns();
		runs.begin("parent", "profile-a");
		const scope = { parentRequestId: "parent", profileId: "profile-a" };
		expect(() => runs.assertCurrent(scope, "profile-a")).not.toThrow();
		expect(() => runs.assertCurrent(scope, "profile-b")).toThrow(
			HomeProfileRunError,
		);
		// Switching back does not revive a run that already observed another profile.
		expect(() => runs.assertCurrent(scope, "profile-a")).toThrow(
			HomeProfileRunError,
		);
	});

	test("the parent binding wins over a new profile supplied by a nested request", () => {
		const runs = new HomeProfileRuns();
		runs.begin("parent", "profile-a");
		expect(() =>
			runs.assertCurrent(
				{ parentRequestId: "parent", profileId: "profile-b" },
				"profile-b",
			),
		).toThrow(HomeProfileRunError);
	});

	test("guards older nested requests without profile metadata and rejects late requests", () => {
		const runs = new HomeProfileRuns();
		runs.begin("parent", "profile-a");
		expect(() =>
			runs.assertCurrent({ parentRequestId: "parent" }, "profile-b"),
		).toThrow(HomeProfileRunError);
		runs.finish("parent");
		expect(() =>
			runs.assertCurrent(
				{ parentRequestId: "parent", profileId: "profile-a" },
				"profile-a",
			),
		).toThrow(HomeProfileRunError);
	});

	test("does not constrain unrelated specialists or another Home run", () => {
		const runs = new HomeProfileRuns();
		runs.begin("a", "profile-a");
		runs.begin("b", "profile-b");
		expect(() =>
			runs.assertCurrent({ parentRequestId: "a" }, "profile-b"),
		).toThrow();
		expect(() =>
			runs.assertCurrent({ parentRequestId: "b" }, "profile-b"),
		).not.toThrow();
		expect(() =>
			runs.assertCurrent({ parentRequestId: "scout" }, "profile-c"),
		).not.toThrow();
	});
});
