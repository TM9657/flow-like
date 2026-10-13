import {
	closeSync,
	mkdirSync,
	openSync,
	readFileSync,
	rmSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { join } from "node:path";

export function acquireBuildLock({ directory, label, wait = true }) {
	mkdirSync(directory, { recursive: true });
	const lock = join(directory, "build.lock");
	let nextNotice = 0;
	for (;;) {
		try {
			const descriptor = openSync(lock, "wx");
			writeFileSync(descriptor, String(process.pid));
			closeSync(descriptor);
			let released = false;
			const release = () => {
				if (released) return;
				released = true;
				process.removeListener("exit", release);
				try {
					if (readFileSync(lock, "utf8") === String(process.pid))
						rmSync(lock, { force: true });
				} catch (error) {
					if (error.code !== "ENOENT") throw error;
				}
			};
			process.on("exit", release);
			return release;
		} catch (error) {
			if (error.code !== "EEXIST") throw error;
			try {
				const owner = Number(readFileSync(lock, "utf8"));
				if (Number.isSafeInteger(owner) && owner > 0) {
					try {
						process.kill(owner, 0);
					} catch (error) {
						if (error.code === "ESRCH") {
							rmSync(lock, { force: true });
							continue;
						}
						if (error.code !== "EPERM") throw error;
					}
				} else if (Date.now() - statSync(lock).mtimeMs > 30_000) {
					rmSync(lock, { force: true });
					continue;
				}
			} catch (error) {
				if (error.code === "ENOENT") continue;
				throw error;
			}
			if (!wait) return null;
			// A cold suite queues several large Cargo builds. A live owner is progress,
			// so elapsed queue time must never turn a second build into a failure.
			if (Date.now() >= nextNotice) {
				console.log(`Waiting for the active ${label} build...`);
				nextNotice = Date.now() + 30_000;
			}
			Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250);
		}
	}
}
