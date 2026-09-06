import { describe, expect, test } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import { IAppStatus, IAppVisibility } from "../schema/app/app";
import {
	executeAppBuildTool,
	type AppBuildToolOptions,
} from "./tool-controller";
import { exampleAppSpec } from "./test-fixtures";
import type { AppBuildState } from "./state";

function fixture() {
	let saved: AppBuildState | null = null;
	let appStatus = IAppStatus.Active;
	let storageFails = false;
	let stagingFails = false;
	let updates = 0;
	const backend = {
		appState: {
			readAppBuild: async () => structuredClone(saved),
			writeAppBuild: async (
				_app: string,
				_build: string,
				record: AppBuildState,
				expected: number | null,
			) => {
				if (storageFails) throw new Error("storage unavailable");
				if (expected !== (saved?.revision ?? null))
					throw new Error("revision conflict");
				saved = structuredClone(record);
			},
			getAppAuthoritative: async () => ({
				id: "app",
				status: appStatus,
				visibility: IAppVisibility.Private,
			}),
			updateAppAuthoritative: async (app: { status: IAppStatus }) => {
				expect(saved).not.toBeNull();
				updates++;
				if (stagingFails) throw new Error("staging unavailable");
				appStatus = app.status;
			},
		},
		boardState: { getBoardSummariesAuthoritative: async () => [] },
		eventState: { getEventsAuthoritative: async () => [] },
		pageState: { getPagesAuthoritative: async () => [] },
		widgetState: { getWidgetsAuthoritative: async () => [] },
		dbState: { listTablesAuthoritative: async () => [] },
	} as unknown as IBackendState;
	const options: AppBuildToolOptions = {
		backend,
		originalRequest: "Build an issue intake app",
		dispatch: {
			assertActive() {},
			referenceApp() {},
			delegate: async () => {
				throw new Error("unexpected delegation");
			},
			generateWidget: async () => [],
		},
		scenarios: {
			attest: async () => {
				throw new Error("runtime isolation unavailable");
			},
			invoke: async () => {
				throw new Error("unexpected runtime execution");
			},
		},
	};
	const args = {
		operation: "begin",
		app_id: "app",
		build_id: "build",
		spec: exampleAppSpec(),
	};
	return {
		options,
		args,
		saved: () => saved,
		status: () => appStatus,
		updates: () => updates,
		failStorage() {
			storageFails = true;
		},
		failStaging() {
			stagingFails = true;
		},
		recover() {
			stagingFails = false;
		},
	};
}

describe("app build tool boundary", () => {
	test("advertises unavailable runtime verification without claiming activation support", async () => {
		const f = fixture();
		const result = await executeAppBuildTool(
			{ operation: "schema" },
			f.options,
		);
		expect(result).toMatchObject({
			host_capabilities: {
				rollout: "opt_in_preview",
				runtime_verification_available: false,
				automatic_activation_available: false,
			},
		});
		expect(f.updates()).toBe(0);
	});
	test("never deactivates an app when durable intent creation fails", async () => {
		const f = fixture();
		f.failStorage();
		await expect(executeAppBuildTool(f.args, f.options)).rejects.toThrow(
			"storage unavailable",
		);
		expect(f.status()).toBe(IAppStatus.Active);
		expect(f.updates()).toBe(0);
	});
	test("resumes a persisted intent after an interrupted staging transition", async () => {
		const f = fixture();
		f.failStaging();
		await expect(executeAppBuildTool(f.args, f.options)).rejects.toThrow(
			"staging unavailable",
		);
		expect(f.saved()?.revision).toBe(0);
		expect(f.status()).toBe(IAppStatus.Active);
		f.recover();
		expect(await executeAppBuildTool(f.args, f.options)).toMatchObject({
			status: "ok",
			resumed: true,
			revision: 0,
		});
		expect(f.status()).toBe(IAppStatus.Inactive);
		expect(f.updates()).toBe(2);
	});
	test("a replay cannot replace the original request or requirement contract", async () => {
		const f = fixture();
		await executeAppBuildTool(f.args, f.options);
		await expect(
			executeAppBuildTool(f.args, {
				...f.options,
				originalRequest: "Only create a demo",
			}),
		).rejects.toThrow("different app contract");
		expect(f.saved()?.original_request).toBe(f.options.originalRequest);
		expect(f.updates()).toBe(1);
	});
	test("promotion cannot activate an unverified build", async () => {
		const f = fixture();
		await executeAppBuildTool(f.args, f.options);
		await expect(
			executeAppBuildTool({ ...f.args, operation: "promote" }, f.options),
		).rejects.toThrow();
		expect(f.status()).toBe(IAppStatus.Inactive);
		expect(f.updates()).toBe(1);
	});
});
