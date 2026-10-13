import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { acquireBuildLock } from "./build-lock.mjs";

test("a competing development builder exits immediately and the owner releases idempotently", () => {
	const directory = mkdtempSync(join(tmpdir(), "flow-craft-lock-"));
	try {
		const release = acquireBuildLock({ directory, label: "test" });
		assert.equal(
			acquireBuildLock({ directory, label: "test", wait: false }),
			null,
		);
		release();
		release();
		const next = acquireBuildLock({ directory, label: "test", wait: false });
		assert.equal(typeof next, "function");
		next();
	} finally {
		rmSync(directory, { recursive: true, force: true });
	}
});
test("waits for a live builder and recovers its stale lock after exit", async () => {
	const directory = mkdtempSync(join(tmpdir(), "flow-craft-lock-"));
	const child = spawn(
		process.execPath,
		["-e", "setTimeout(()=>process.exit(0),100)"],
		{ stdio: "ignore" },
	);
	assert.ok(child.pid);
	writeFileSync(join(directory, "build.lock"), String(child.pid));
	try {
		assert.equal(
			acquireBuildLock({ directory, label: "test", wait: false }),
			null,
		);
		await new Promise((resolve) => child.once("exit", resolve));
		const release = acquireBuildLock({ directory, label: "test", wait: false });
		assert.equal(typeof release, "function");
		release();
	} finally {
		child.kill();
		rmSync(directory, { recursive: true, force: true });
	}
});
