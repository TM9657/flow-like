import { spawn } from "node:child_process";
import {
	closeSync,
	existsSync,
	mkdirSync,
	openSync,
	readdirSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { acquireBuildLock } from "./build-lock.mjs";
import { buildConfiguration, runBuilds } from "./build-pool.mjs";

const script = fileURLToPath(import.meta.url);
const directory = dirname(script);
const root = resolve(directory, "../..");
const development = join(root, "target/craft-viewers/development");
if (process.argv.includes("--dev")) {
	mkdirSync(development, { recursive: true });
	const log = join(development, "build.log");
	const descriptor = openSync(log, "a");
	const child = spawn(process.execPath, [script, "--background"], {
		cwd: root,
		detached: true,
		stdio: ["ignore", descriptor, descriptor],
	});
	closeSync(descriptor);
	child.unref();
	console.log(`File editors prepare in the background. Build log: ${log}`);
} else {
	const release = process.argv.includes("--background")
		? acquireBuildLock({
				directory: development,
				label: "file editor suite",
				wait: false,
			})
		: undefined;
	if (release === null) process.exit(0);
	try {
		const tasks = [
			{
				label: "wordcraft",
				args: [join(root, "packages/wordcraft/build-web.mjs")],
			},
		];
		for (const id of readdirSync(directory).sort()) {
			if (!existsSync(join(directory, id, "upstream.json"))) continue;
			tasks.push({
				label: id,
				args: [join(directory, "build-runtime.mjs"), id],
			});
		}
		const configuration = buildConfiguration();
		console.log(
			`Preparing ${tasks.length} file editors with up to ${configuration.concurrency} parallel builds (${configuration.env.CARGO_BUILD_JOBS} Cargo jobs each).`,
		);
		await runBuilds(tasks, { ...configuration, cwd: root });
	} catch (error) {
		console.error(error.message);
		process.exitCode = 1;
	} finally {
		release?.();
	}
}
