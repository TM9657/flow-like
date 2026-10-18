import { spawn } from "node:child_process";
import { availableParallelism, totalmem } from "node:os";

export function buildConfiguration({
	env = process.env,
	cpus = availableParallelism(),
	memory = Math.min(
		totalmem(),
		process.constrainedMemory?.() || Number.POSITIVE_INFINITY,
	),
} = {}) {
	const requested = env.CRAFT_BUILD_CONCURRENCY;
	if (requested !== undefined && !/^[1-9]\d*$/.test(requested))
		throw new Error("CRAFT_BUILD_CONCURRENCY must be a positive integer.");
	const cargoJobs = Number(env.CARGO_BUILD_JOBS || 4);
	const jobsPerBuild =
		Number.isSafeInteger(cargoJobs) && cargoJobs > 0 ? cargoJobs : cpus;
	const concurrency = requested
		? Number(requested)
		: Math.max(
				1,
				Math.min(
					4,
					Math.floor(cpus / jobsPerBuild),
					Math.floor(memory / (4 * 1024 ** 3)),
				),
			);
	if (!Number.isSafeInteger(concurrency))
		throw new Error("CRAFT_BUILD_CONCURRENCY must be a positive integer.");
	return {
		concurrency,
		env: {
			...env,
			CRAFT_BUILD_CONCURRENCY: String(concurrency),
			CARGO_BUILD_JOBS:
				env.CARGO_BUILD_JOBS ||
				String(Math.max(1, Math.min(4, Math.floor(cpus / concurrency)))),
		},
	};
}

export async function runBuilds(
	tasks,
	{ concurrency, cwd, env, shutdownTimeout = 5000 },
) {
	if (!Number.isSafeInteger(concurrency) || concurrency < 1)
		throw new Error("Build concurrency must be a positive integer.");
	let next = 0;
	let failure;
	let cancellation;
	const active = new Set();
	const shutdownTimers = new Map();
	function terminate(child, signal) {
		if (!child.pid) return;
		const direct = () => {
			try {
				child.kill(signal);
			} catch {}
		};
		if (process.platform === "win32") {
			const killer = spawn(
				"taskkill",
				["/PID", String(child.pid), "/T", "/F"],
				{
					stdio: "ignore",
				},
			);
			killer.once("error", direct);
			killer.once("close", (code) => {
				if (code !== 0) direct();
			});
		} else {
			try {
				// Each worker owns a process group that also contains Cargo and rustc.
				process.kill(-child.pid, signal);
			} catch (error) {
				if (error.code !== "ESRCH") direct();
			}
		}
	}
	function cancel(signal) {
		const force = Boolean(cancellation);
		cancellation ??= signal;
		failure ??= new Error(`File editor builds cancelled by ${signal}.`);
		for (const child of active) {
			terminate(child, force ? "SIGKILL" : signal);
			if (!shutdownTimers.has(child)) {
				const timer = setTimeout(
					() => terminate(child, "SIGKILL"),
					shutdownTimeout,
				);
				timer.unref();
				shutdownTimers.set(child, timer);
			}
		}
	}
	const interrupt = () => cancel("SIGINT");
	const terminateSignal = () => cancel("SIGTERM");
	process.on("SIGINT", interrupt);
	process.on("SIGTERM", terminateSignal);
	async function worker() {
		while (!failure && next < tasks.length) {
			const { label, args } = tasks[next++];
			const started = performance.now();
			console.log(`[editors] Starting ${label}`);
			try {
				await new Promise((resolve, reject) => {
					const child = spawn(process.execPath, args, {
						cwd,
						env,
						stdio: "inherit",
						detached: process.platform !== "win32",
					});
					active.add(child);
					child.once("error", reject);
					child.once("close", (code, signal) => {
						active.delete(child);
						clearTimeout(shutdownTimers.get(child));
						shutdownTimers.delete(child);
						// A terminated worker can leave descendants that ignored its signal.
						if (cancellation && process.platform !== "win32")
							terminate(child, "SIGKILL");
						if (code === 0) resolve();
						else
							reject(
								new Error(signal ? `signal ${signal}` : `exit code ${code}`),
							);
					});
				});
				console.log(
					`[editors] Finished ${label} in ${((performance.now() - started) / 1000).toFixed(1)}s`,
				);
			} catch (cause) {
				failure ??= new Error(`${label} build failed: ${cause.message}`, {
					cause,
				});
			}
		}
	}
	try {
		// Drain active builds before releasing the suite lock. A failure stops the queue.
		await Promise.all(
			Array.from({ length: Math.min(concurrency, tasks.length) }, worker),
		);
	} finally {
		process.removeListener("SIGINT", interrupt);
		process.removeListener("SIGTERM", terminateSignal);
		for (const timer of shutdownTimers.values()) clearTimeout(timer);
	}
	if (failure) throw failure;
}
