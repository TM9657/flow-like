import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import {
	existsSync,
	mkdtempSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";
import { buildConfiguration, runBuilds } from "./build-pool.mjs";

test("automatic concurrency budgets CPU and memory and caps large machines", () => {
	for (const [cpus, gib, concurrency, jobs] of [
		[1, 2, 1, "1"],
		[4, 8, 1, "4"],
		[8, 16, 2, "4"],
		[16, 8, 2, "4"],
		[64, 128, 4, "4"],
	]) {
		const configuration = buildConfiguration({
			env: {},
			cpus,
			memory: gib * 1024 ** 3,
		});
		assert.equal(configuration.concurrency, concurrency);
		assert.equal(
			configuration.env.CRAFT_BUILD_CONCURRENCY,
			String(concurrency),
		);
		assert.equal(configuration.env.CARGO_BUILD_JOBS, jobs);
	}
});

test("automatic concurrency accounts for an explicit Cargo job budget", () => {
	for (const [jobs, concurrency] of [
		["8", 2],
		["32", 1],
		["-1", 1],
	]) {
		const configuration = buildConfiguration({
			env: { CARGO_BUILD_JOBS: jobs },
			cpus: 16,
			memory: 64 * 1024 ** 3,
		});
		assert.equal(configuration.concurrency, concurrency);
		assert.equal(configuration.env.CARGO_BUILD_JOBS, jobs);
	}
});

test("explicit concurrency and Cargo job settings are respected", () => {
	const env = {
		CRAFT_BUILD_CONCURRENCY: "3",
		CARGO_BUILD_JOBS: "2",
		CI: "true",
	};
	assert.deepEqual(
		buildConfiguration({ env, cpus: 8, memory: 8 * 1024 ** 3 }),
		{
			concurrency: 3,
			env,
		},
	);
	assert.equal(
		buildConfiguration({ env: { CRAFT_BUILD_CONCURRENCY: "3" }, cpus: 8 }).env
			.CARGO_BUILD_JOBS,
		"2",
	);
	for (const value of ["", "0", "-1", "1.5", "two", "9007199254740992"])
		assert.throws(
			() => buildConfiguration({ env: { CRAFT_BUILD_CONCURRENCY: value } }),
			/CRAFT_BUILD_CONCURRENCY must be a positive integer/,
		);
});

const gatedBuild = `
const { existsSync, writeFileSync } = require("node:fs");
const { join } = require("node:path");
const [directory, id, code] = process.argv.slice(1);
writeFileSync(join(directory, id + ".started"), process.env.CARGO_BUILD_JOBS || "");
const timer = setInterval(() => {
	if (!existsSync(join(directory, id + ".release"))) return;
	clearInterval(timer);
	writeFileSync(join(directory, id + ".done"), "done");
	process.exit(Number(code));
}, 10);
`;

function task(directory, id, code = 0) {
	return { label: id, args: ["-e", gatedBuild, directory, id, String(code)] };
}

async function waitFor(path) {
	const deadline = Date.now() + 5000;
	while (!existsSync(path)) {
		assert.ok(Date.now() < deadline, `Timed out waiting for ${path}`);
		await delay(10);
	}
}

test("builds overlap within the limit and queued work starts as slots open", async () => {
	const directory = mkdtempSync(join(tmpdir(), "flow-craft-pool-"));
	const ids = ["first", "second", "third", "fourth"];
	const build = runBuilds(
		ids.map((id) => task(directory, id)),
		{
			concurrency: 2,
			cwd: directory,
			env: { ...process.env, CARGO_BUILD_JOBS: "2" },
		},
	);
	try {
		await Promise.all(
			ids.slice(0, 2).map((id) => waitFor(join(directory, `${id}.started`))),
		);
		assert.equal(existsSync(join(directory, "third.started")), false);
		assert.equal(existsSync(join(directory, "fourth.started")), false);
		writeFileSync(join(directory, "first.release"), "");
		await waitFor(join(directory, "third.started"));
		assert.equal(existsSync(join(directory, "second.done")), false);
		assert.equal(existsSync(join(directory, "fourth.started")), false);
		writeFileSync(join(directory, "third.release"), "");
		await waitFor(join(directory, "fourth.started"));
		assert.equal(existsSync(join(directory, "second.done")), false);
		for (const id of ids)
			assert.equal(readFileSync(join(directory, `${id}.started`), "utf8"), "2");
	} finally {
		for (const id of ids) writeFileSync(join(directory, `${id}.release`), "");
		await build;
		rmSync(directory, { recursive: true, force: true });
	}
});

test("a failed build stops the queue and waits for active builds before rejecting", async () => {
	const directory = mkdtempSync(join(tmpdir(), "flow-craft-failure-"));
	let settled = false;
	const build = runBuilds(
		[
			task(directory, "failed", 17),
			task(directory, "active"),
			task(directory, "queued"),
		],
		{ concurrency: 2, cwd: directory, env: process.env },
	).then(
		() => {
			settled = true;
		},
		(error) => {
			settled = true;
			return error;
		},
	);
	try {
		await waitFor(join(directory, "active.started"));
		writeFileSync(join(directory, "failed.release"), "");
		await waitFor(join(directory, "failed.done"));
		await delay(100);
		assert.equal(settled, false);
		assert.equal(existsSync(join(directory, "queued.started")), false);
		writeFileSync(join(directory, "active.release"), "");
		const error = await build;
		assert.match(error.message, /failed build failed: exit code 17/);
		assert.equal(existsSync(join(directory, "active.done")), true);
		assert.equal(existsSync(join(directory, "queued.started")), false);
	} finally {
		for (const id of ["failed", "active", "queued"])
			writeFileSync(join(directory, `${id}.release`), "");
		await build;
		rmSync(directory, { recursive: true, force: true });
	}
});

test("spawn failures and signalled children fail the build", async () => {
	const options = { concurrency: 1, cwd: tmpdir(), env: process.env };
	await assert.rejects(
		runBuilds([{ label: "missing", args: ["-e", ""] }], {
			...options,
			cwd: join(tmpdir(), "flow-craft-missing-directory", "missing"),
		}),
		/missing build failed:.*ENOENT/,
	);
	await assert.rejects(
		runBuilds(
			[
				{
					label: "signal",
					args: ["-e", 'process.kill(process.pid, "SIGTERM")'],
				},
			],
			options,
		),
		process.platform === "win32"
			? /signal build failed: exit code 1/
			: /signal build failed: signal SIGTERM/,
	);
});

for (const ignoreSignal of [false, true]) {
	test(
		ignoreSignal
			? "cancellation escalates when a worker ignores its signal"
			: "parent cancellation stops queued builds and terminates worker descendants",
		{ timeout: 10000 },
		async () => {
			const directory = mkdtempSync(join(tmpdir(), "flow-craft-cancel-"));
			const grandchildScript = `
				const { writeFileSync } = require("node:fs");
				const { join } = require("node:path");
				const directory = process.argv[1];
				process.on("SIGTERM", () => {});
				process.on("SIGINT", () => {});
				writeFileSync(join(directory, "grandchild.pid"), String(process.pid));
				let tick = 0;
				setInterval(() => writeFileSync(join(directory, "heartbeat"), String(++tick)), 10);
			`;
			const workerScript = `
				const { spawn } = require("node:child_process");
				const { writeFileSync } = require("node:fs");
				const { join } = require("node:path");
				const directory = process.argv[1];
				if (${ignoreSignal}) {
					process.on("SIGTERM", () => {});
					process.on("SIGINT", () => {});
				}
				writeFileSync(join(directory, "worker.pid"), String(process.pid));
				spawn(process.execPath, ["-e", ${JSON.stringify(grandchildScript)}, directory], { stdio: "ignore" });
				setInterval(() => {}, 1000);
			`;
			const runnerScript = `
				import { existsSync, writeFileSync } from "node:fs";
				import { join } from "node:path";
				import { runBuilds } from ${JSON.stringify(new URL("./build-pool.mjs", import.meta.url).href)};
				const directory = process.argv[1];
				const before = [process.listenerCount("SIGINT"), process.listenerCount("SIGTERM")];
				const cancellation = setInterval(() => {
					if (existsSync(join(directory, "cancel"))) {
						clearInterval(cancellation);
						process.emit("SIGTERM");
					}
				}, 10);
				try {
					await runBuilds([
						{ label: "active", args: ["-e", ${JSON.stringify(workerScript)}, directory] },
						{ label: "queued", args: ["-e", ${JSON.stringify(gatedBuild)}, directory, "queued", "0"] },
					], { concurrency: 1, cwd: directory, env: process.env, shutdownTimeout: 200 });
				} catch (error) {
					writeFileSync(join(directory, "error"), error.message);
					process.exitCode = 1;
				} finally {
					clearInterval(cancellation);
					writeFileSync(join(directory, "listeners"), JSON.stringify({
						before, after: [process.listenerCount("SIGINT"), process.listenerCount("SIGTERM")],
					}));
				}
			`;
			const runner = spawn(
				process.execPath,
				["--input-type=module", "-e", runnerScript, directory],
				{ stdio: "ignore" },
			);
			const finished = new Promise((resolve, reject) => {
				runner.once("error", reject);
				runner.once("close", (code) => resolve(code));
			});
			try {
				await waitFor(join(directory, "heartbeat"));
				if (process.platform === "win32") {
					// Windows does not deliver POSIX signals, so exercise the handler directly.
					writeFileSync(join(directory, "cancel"), "");
				} else {
					runner.kill("SIGTERM");
				}
				assert.equal(await finished, 1);
				assert.match(
					readFileSync(join(directory, "error"), "utf8"),
					/cancelled by SIGTERM/,
				);
				assert.equal(existsSync(join(directory, "queued.started")), false);
				const { before, after } = JSON.parse(
					readFileSync(join(directory, "listeners"), "utf8"),
				);
				assert.deepEqual(after, before);
				await delay(50);
				const heartbeat = readFileSync(join(directory, "heartbeat"), "utf8");
				await delay(50);
				assert.equal(
					readFileSync(join(directory, "heartbeat"), "utf8"),
					heartbeat,
				);
			} finally {
				for (const name of ["worker.pid", "grandchild.pid"]) {
					if (!existsSync(join(directory, name))) continue;
					const pid = Number(readFileSync(join(directory, name), "utf8"));
					if (process.platform === "win32") {
						spawnSync("taskkill", ["/PID", String(pid), "/T", "/F"], {
							stdio: "ignore",
						});
					} else {
						try {
							process.kill(pid, "SIGKILL");
						} catch {}
					}
				}
				runner.kill("SIGKILL");
				await finished;
				rmSync(directory, { recursive: true, force: true });
			}
		},
	);
}
