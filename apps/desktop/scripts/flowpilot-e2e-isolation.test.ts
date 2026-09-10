import { afterEach, describe, expect, test } from "bun:test";
import { chmod, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
	createFlowPilotE2EIsolation,
	verifyFlowPilotE2EIsolation,
} from "./flowpilot-e2e-isolation";

const roots: string[] = [];
const runId = "e2e_1725870000000_0123456789abcdef";

async function fixture() {
	const isolation = await createFlowPilotE2EIsolation(runId);
	roots.push(isolation.root);
	return isolation;
}

afterEach(async () => {
	await Promise.all(
		roots.splice(0).map((root) => rm(root, { recursive: true, force: true })),
	);
});

describe("FlowPilot E2E isolation", () => {
	test("creates a private fresh root without copying profile or authentication data", async () => {
		const isolation = await fixture();
		await verifyFlowPilotE2EIsolation(isolation);
		expect(Object.keys(isolation.env).sort()).toEqual([
			"CACHE_DIR",
			"FLOWPILOT_E2E_DATA_ROOT",
			"FLOW_LIKE_FLOWPILOT_DRAFT_DIR",
		]);
		expect(isolation.config.identifier).toBe("com.flow-like.e2e");
		expect(isolation.config.app.windows[0]?.incognito).toBe(true);
	});

	test("never reuses a root between runs", async () => {
		const first = await fixture();
		const second = await fixture();
		expect(first.root).not.toBe(second.root);
	});

	test("rejects pre-existing data or a native writer claim", async () => {
		const isolation = await fixture();
		await writeFile(join(isolation.root, ".native-owner"), "123");
		await expect(verifyFlowPilotE2EIsolation(isolation)).rejects.toThrow(
			"not fresh",
		);
	});

	test("rejects another runner or run ownership marker", async () => {
		const isolation = await fixture();
		const markerPath = join(isolation.root, ".flowpilot-e2e-owner.json");
		const marker = JSON.parse(await readFile(markerPath, "utf8"));
		await writeFile(
			markerPath,
			JSON.stringify({ ...marker, runner_pid: process.pid + 1 }),
		);
		await expect(verifyFlowPilotE2EIsolation(isolation)).rejects.toThrow(
			"owner does not match",
		);
	});

	test("rejects shared storage or persistent webview configuration", async () => {
		const isolation = await fixture();
		isolation.env.CACHE_DIR = "/shared/cache";
		await expect(verifyFlowPilotE2EIsolation(isolation)).rejects.toThrow(
			"configuration does not match",
		);
		isolation.env.CACHE_DIR = join(isolation.root, "cache");
		const main = isolation.config.app.windows[0];
		if (!main) throw new Error("Missing main window");
		main.incognito = false;
		await expect(verifyFlowPilotE2EIsolation(isolation)).rejects.toThrow(
			"configuration does not match",
		);
	});

	test.skipIf(process.platform === "win32")(
		"rejects public permissions",
		async () => {
			const isolation = await fixture();
			await chmod(isolation.root, 0o755);
			await expect(verifyFlowPilotE2EIsolation(isolation)).rejects.toThrow(
				"private and owned",
			);
		},
	);
});
