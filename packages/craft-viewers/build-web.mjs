import { execFileSync, spawn } from "node:child_process";
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
		execFileSync(
			process.execPath,
			[join(root, "packages/wordcraft/build-web.mjs")],
			{ cwd: root, stdio: "inherit" },
		);
		for (const id of readdirSync(directory).sort()) {
			if (!existsSync(join(directory, id, "upstream.json"))) continue;
			execFileSync(
				process.execPath,
				[join(directory, "build-runtime.mjs"), id],
				{ cwd: root, stdio: "inherit" },
			);
		}
	} finally {
		release?.();
	}
}
