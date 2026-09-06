import { describe, expect, it } from "vitest";

import { createBuild } from "./state";
import { createAppBuildStore, type AppBuildStorageBackend } from "./store";
import { exampleAppSpec } from "./test-fixtures";

function backendReturning(value: unknown): AppBuildStorageBackend {
	return {
		async readAppBuild() {
			return value;
		},
		async writeAppBuild() {},
	};
}

describe("AppBuildStore", () => {
	it("rejects a valid-looking record returned from the wrong durable key", async () => {
		const build = createBuild(exampleAppSpec(), {
			app_id: "other_app",
			build_id: "build_1",
			now_ms: 1,
		});
		const store = createAppBuildStore(backendReturning(build));
		await expect(
			store.read({ app_id: "app_1", build_id: "build_1" }),
		).rejects.toThrow(/durable storage key/);
	});

	it("rejects persisted derived fingerprints that do not match the contract", async () => {
		const build = createBuild(exampleAppSpec(), {
			app_id: "app_1",
			build_id: "build_1",
			now_ms: 1,
		});
		const store = createAppBuildStore(
			backendReturning({ ...build, spec_fingerprint: "fp1:tampered" }),
		);
		await expect(
			store.read({ app_id: "app_1", build_id: "build_1" }),
		).rejects.toThrow(/spec_fingerprint/);
	});
});
