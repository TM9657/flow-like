import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import os from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer";
import { percentile } from "./measurement.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const options = {
	origin: process.env.BOARD_BENCHMARK_URL || "http://127.0.0.1:4336",
	output:
		process.env.BOARD_BENCHMARK_OUTPUT ||
		join(root, "tmp/board-render-benchmark"),
	trials: 3,
	duration: 3000,
	nodes: 10000,
	dpr: null,
	renderers: ["react-flow", "egui", "egui-lod"],
	views: ["overview", "detail"],
	motions: [0, 1],
	scenarios: ["idle", "pan", "zoom", "drag", "activity"],
	headed: true,
};
for (let index = 2; index < process.argv.length; index++) {
	const [key, inline] = process.argv[index].replace(/^--/, "").split("=", 2);
	if (key === "help") {
		console.log(
			"node tests/board-render-browser/run.mjs [--trials 3] [--duration 3000] [--nodes 10000] [--dpr NUMBER] [--renderers react-flow,egui,egui-lod] [--views overview,detail] [--motions 0,1] [--scenarios idle,pan,zoom,drag,activity] [--headed=true|false] [--origin URL] [--output PATH]",
		);
		process.exit(0);
	}
	if (key === "headless") {
		options.headed = false;
		continue;
	}
	if (
		key === "headed" &&
		inline === undefined &&
		!["true", "false"].includes(process.argv[index + 1])
	) {
		options.headed = true;
		continue;
	}
	const value = inline ?? process.argv[++index];
	if (value === undefined || !(key in options))
		throw new Error(`Unknown or incomplete option: --${key}`);
	if (["trials", "duration", "nodes", "dpr"].includes(key))
		options[key] = Number(value);
	else if (["renderers", "views", "scenarios"].includes(key))
		options[key] = value.split(",");
	else if (key === "motions") options.motions = value.split(",").map(Number);
	else if (key === "headed") options.headed = value !== "false";
	else options[key] = value;
}
for (const key of ["trials", "duration", "nodes"])
	assert.ok(
		Number.isInteger(options[key]) && options[key] > 0,
		`${key} must be a positive integer`,
	);
assert.ok(
	options.renderers.every((value) =>
		["react-flow", "egui", "egui-lod"].includes(value),
	),
	"Unknown renderer",
);
assert.ok(
	options.dpr === null || (Number.isFinite(options.dpr) && options.dpr > 0),
	"DPR must be positive",
);
assert.ok(
	options.views.every((value) => ["overview", "detail"].includes(value)),
	"Unknown view",
);
assert.ok(
	options.motions.every((value) => value === 0 || value === 1),
	"Motion must be 0 or 1",
);
assert.ok(
	options.scenarios.every((value) =>
		["idle", "pan", "zoom", "drag", "activity"].includes(value),
	),
	"Unknown scenario",
);

await mkdir(options.output, { recursive: true });
const report = {
	schemaVersion: 1,
	startedAt: new Date().toISOString(),
	options,
	machine: {
		platform: os.platform(),
		release: os.release(),
		arch: os.arch(),
		cpu: os.cpus()[0]?.model,
		logicalCpus: os.cpus().length,
		totalMemoryBytes: os.totalmem(),
	},
	viewport: { width: 1600, height: 1000, deviceScaleFactor: null },
	metricNotes: [
		"rAF intervals measure foreground scheduling cadence, not GPU render duration or input latency.",
		"Missed frames are estimated against a blank foreground page's measured refresh interval.",
		"mainThreadTaskSecondsPerWallSecond is CDP TaskDuration / CDP Timestamp, not whole-machine CPU utilization.",
		"CDP and process CPU intervals include scenario setup, cleanup, and final renderer stats; RAF and long-task samples cover only the measured animation interval.",
		"Browser-process CPU includes the controlled browser's renderer, GPU and other processes; it excludes the benchmark runner.",
		"JS heap omits GPU textures and may not account for all WASM linear memory. No forced garbage collection is used.",
		"The synthetic canvas workload does not measure full editor loading, sync, panels, accessibility, or command round trips.",
	],
	warnings: [],
	runs: [],
};
const save = () =>
	writeFile(
		join(options.output, "results.json"),
		`${JSON.stringify(report, null, 2)}\n`,
	);
const browser = await puppeteer.launch({
	headless: !options.headed,
	executablePath:
		process.env.CHROME_PATH ||
		"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
	args: [
		"--disable-background-timer-throttling",
		"--disable-renderer-backgrounding",
		"--disable-backgrounding-occluded-windows",
		"--enable-precise-memory-info",
		"--window-size=1600,1100",
	],
	userDataDir: join(options.output, `browser-data-${process.pid}`),
	defaultViewport: null,
});

function metricMap(result) {
	return Object.fromEntries(
		result.metrics.map(({ name, value }) => [name, value]),
	);
}

function cpuDelta(before, after) {
	if (!before || !after) return null;
	const previous = new Map(
		before.processInfo.map((entry) => [entry.id, entry]),
	);
	const deltas = after.processInfo.map((entry) => ({
		id: entry.id,
		type: entry.type,
		seconds: previous.has(entry.id)
			? Math.max(0, entry.cpuTime - previous.get(entry.id).cpuTime)
			: null,
	}));
	return {
		processes: deltas,
		totalSeconds: deltas.reduce((sum, entry) => sum + (entry.seconds ?? 0), 0),
		complete:
			deltas.every((entry) => entry.seconds !== null) &&
			before.processInfo.every((entry) =>
				after.processInfo.some((next) => next.id === entry.id),
			),
	};
}

async function inspectPage(page) {
	return page.evaluate(() => {
		const snapshot = window.boardBenchmark.snapshot();
		const canvases = [...document.querySelectorAll("canvas")].map((canvas) => {
			const rect = canvas.getBoundingClientRect();
			return {
				width: canvas.width,
				height: canvas.height,
				clientWidth: rect.width,
				clientHeight: rect.height,
			};
		});
		return {
			snapshot,
			dom: {
				nodes: document.querySelectorAll("*").length,
				flowNodes: document.querySelectorAll(".react-flow__node").length,
				flowEdges: document.querySelectorAll(".react-flow__edge").length,
				canvases,
			},
			productionScripts: [...document.scripts]
				.map((script) => script.src)
				.filter(Boolean),
			visibility: document.visibilityState,
		};
	});
}

function validateVisuals(state, renderer) {
	const snapshot = state.snapshot;
	const renderedNodes = snapshot.renderedNodes ?? snapshot.renderedNodeCount;
	const renderedEdges = snapshot.renderedEdges;
	assert.equal(
		snapshot.production,
		true,
		"Production build required: run Vite build and preview before this runner",
	);
	assert.equal(
		snapshot.nodeCount,
		options.nodes,
		"Renderer node count differs from requested workload",
	);
	if (snapshot.expectedNodeCount !== undefined)
		assert.equal(
			snapshot.nodeCount,
			snapshot.expectedNodeCount,
			"Node count differs from fixture",
		);
	assert.ok(snapshot.edgeCount > 0, "Fixture must contain edges");
	assert.equal(
		typeof snapshot.expectedEdgeCount,
		"number",
		"Snapshot must expose expectedEdgeCount from shared fixture",
	);
	assert.equal(
		snapshot.edgeCount,
		snapshot.expectedEdgeCount,
		"Edge count differs from fixture",
	);
	if (snapshot.view === "overview") {
		assert.equal(
			renderedNodes,
			snapshot.nodeCount,
			"Overview must draw every fixture node",
		);
		assert.equal(
			renderedEdges,
			snapshot.edgeCount,
			"Overview must draw every fixture edge",
		);
	}
	assert.equal(state.visibility, "visible", "Page must be in foreground");
	assert.ok(
		!state.productionScripts.some((src) => src.includes("/@vite/client")),
		"Development server detected",
	);
	if (renderer === "react-flow") {
		assert.ok(state.dom.flowNodes > 0, "React Flow has no rendered nodes");
		assert.ok(state.dom.flowEdges > 0, "React Flow has no rendered edges");
	} else {
		assert.equal(
			snapshot.lodEnabled,
			renderer === "egui-lod",
			"The renderer's actual LOD setting differs from its benchmark label",
		);
		const zoom = Array.isArray(snapshot.viewport)
			? snapshot.viewport[2]
			: snapshot.viewport.zoom;
		const expectedLevel =
			renderer === "egui" || zoom >= 0.5
				? "full"
				: zoom >= 0.2
					? "compact"
					: zoom >= 0.1
						? "overview"
						: "tiny";
		assert.equal(
			snapshot.lodLevel,
			expectedLevel,
			"Unexpected canvas detail level",
		);
		if (expectedLevel === "full") {
			assert.equal(
				snapshot.lodDetails,
				"All labels, icons, pins and detailed edges",
				"Full-detail mode must declare its complete visual workload",
			);
			assert.equal(
				snapshot.labelsDrawn,
				renderedNodes * 9,
				"Full-detail mode must draw one title and eight pin labels per node",
			);
			assert.equal(
				snapshot.pinsDrawn,
				renderedNodes * 8,
				"Full-detail mode must draw all eight pin markers per node",
			);
		}
		assert.ok(
			state.dom.canvases.some(
				(canvas) =>
					canvas.width > 0 &&
					canvas.height > 0 &&
					canvas.clientWidth > 0 &&
					canvas.clientHeight > 0,
			),
			`${renderer} has no nonempty canvas`,
		);
		const drawnCount =
			snapshot.renderedNodeCount ??
			snapshot.visibleNodeCount ??
			snapshot.drawnNodeCount;
		assert.ok(
			drawnCount > 0,
			`${renderer} snapshot must report renderedNodeCount, visibleNodeCount or drawnNodeCount > 0`,
		);
	}
}

try {
	report.browserVersion = await browser.version();
	const browserCdp = await browser.target().createCDPSession();
	try {
		report.systemInfo = await browserCdp.send("SystemInfo.getInfo");
	} catch (error) {
		report.warnings.push(`SystemInfo unavailable: ${error.message}`);
	}
	const getProcesses = () =>
		browserCdp.send("SystemInfo.getProcessInfo").catch(() => null);
	const calibration = await browser.newPage();
	report.nativeDevicePixelRatio = await calibration.evaluate(
		() => devicePixelRatio,
	);
	report.viewport.deviceScaleFactor =
		options.dpr ?? report.nativeDevicePixelRatio;
	await calibration.setViewport(report.viewport);
	await calibration.bringToFront();
	// The helper is serialized so production Vite need not publish source modules.
	const { readFile } = await import("node:fs/promises");
	const helperSource = await readFile(
		new URL("./measurement.mjs", import.meta.url),
		"utf8",
	);
	await calibration.goto("about:blank");
	report.refreshCalibration = await calibration.evaluate(async (source) => {
		const url = URL.createObjectURL(
			new Blob([source], { type: "text/javascript" }),
		);
		try {
			return await (await import(url)).calibrateRefreshRate();
		} finally {
			URL.revokeObjectURL(url);
		}
	}, helperSource);
	assert.ok(
		report.refreshCalibration.intervalMs > 0,
		"Could not calibrate refresh interval",
	);
	report.webgl = await calibration.evaluate(() => {
		const canvas = document.createElement("canvas");
		const gl = canvas.getContext("webgl2") ?? canvas.getContext("webgl");
		if (!gl) return { available: false };
		const debug = gl.getExtension("WEBGL_debug_renderer_info");
		const result = {
			available: true,
			vendor: gl.getParameter(gl.VENDOR),
			renderer: gl.getParameter(gl.RENDERER),
			unmaskedVendor: debug
				? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL)
				: null,
			unmaskedRenderer: debug
				? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL)
				: null,
			version: gl.getParameter(gl.VERSION),
		};
		gl.getExtension("WEBGL_lose_context")?.loseContext();
		return result;
	});
	const rendererDescription = JSON.stringify({
		webgl: report.webgl,
		systemInfo: report.systemInfo?.gpu,
	});
	if (
		/swiftshader|llvmpipe|software rasterizer|software rendering/i.test(
			rendererDescription,
		)
	)
		report.warnings.push(
			"Software GPU rendering was detected. These results cannot establish hardware-accelerated performance.",
		);
	if (!report.webgl.available)
		report.warnings.push("WebGL was unavailable during calibration.");
	if (!options.headed)
		report.warnings.push(
			"Headless run: compare only with results using the same browser mode.",
		);
	await calibration.close();
	for (const page of await browser.pages()) await page.close();

	const configurations = options.views.flatMap((view) =>
		options.motions.flatMap((motion) =>
			options.renderers.map((renderer) => ({ renderer, view, motion })),
		),
	);
	for (let trial = 1; trial <= options.trials; trial++) {
		const ordered =
			trial % 2 === 0 ? [...configurations].reverse() : configurations;
		for (const config of ordered) {
			const page = await browser.newPage();
			await page.setViewport(report.viewport);
			await page.bringToFront();
			const run = {
				trial,
				...config,
				renderMode: config.renderer === "egui-lod" ? "lod" : "full-detail",
				status: "running",
				consoleErrors: [],
				browserEvents: [],
				scenarios: [],
				inputScenarios: [],
			};
			report.runs.push(run);
			const lifecycleStartedAt = performance.now();
			let phase = "initialization";
			let closingPage = false;
			const browserEvent = (type, details = {}) =>
				run.browserEvents.push({
					type,
					at: new Date().toISOString(),
					elapsedMs: performance.now() - lifecycleStartedAt,
					phase,
					...details,
				});
			page.on("pageerror", (error) => run.consoleErrors.push(error.message));
			page.on("error", (error) =>
				browserEvent("renderer-crash", {
					message: error.message,
					stack: error.stack,
				}),
			);
			page.on("framenavigated", (frame) => {
				if (frame === page.mainFrame())
					browserEvent("main-frame-navigation", { url: frame.url() });
			});
			page.on("close", () =>
				browserEvent("page-close", { expected: closingPage }),
			);
			try {
				const url = new URL(options.origin);
				for (const [key, value] of Object.entries({
					...config,
					nodes: options.nodes,
				}))
					url.searchParams.set(key, String(value));
				run.url = url.href;
				const openedAt = Date.now();
				await page.goto(url.href, {
					waitUntil: "domcontentloaded",
					timeout: 120000,
				});
				await page.waitForFunction(
					() => ["ready", "error"].includes(window.boardBenchmark?.status),
					{ timeout: 120000 },
				);
				const readiness = await page.evaluate(() => ({
					status: window.boardBenchmark.status,
					error: window.boardBenchmark.error,
				}));
				assert.equal(
					readiness.status,
					"ready",
					`Renderer initialization failed: ${readiness.error}`,
				);
				run.readyWallMs = Date.now() - openedAt;
				await page.evaluate((interval) => {
					window.__boardBenchmarkRefreshIntervalMs = interval;
				}, report.refreshCalibration.intervalMs);
				phase = "warmup";
				await page.evaluate(async () => {
					await window.boardBenchmark.runScenario("pan", 600);
					await window.boardBenchmark.runScenario("idle", 600);
				});
				run.before = await inspectPage(page);
				validateVisuals(run.before, config.renderer);
				assert.equal(
					typeof run.before.snapshot.fixtureChecksum,
					"string",
					"Snapshot must expose the shared fixture checksum",
				);
				report.fixtureChecksum ??= run.before.snapshot.fixtureChecksum;
				assert.equal(
					run.before.snapshot.fixtureChecksum,
					report.fixtureChecksum,
					"Renderers did not receive identical fixtures",
				);
				if (trial === 1) {
					phase = "screenshot";
					run.screenshot = `${config.renderer}-${config.view}-motion-${config.motion}.png`;
					await page.screenshot({ path: join(options.output, run.screenshot) });
				}
				const cdp = await page.createCDPSession();
				await cdp.send("Performance.enable");
				const measure = async (name, action) => {
					phase = name;
					const beforeMetrics = metricMap(
						await cdp.send("Performance.getMetrics"),
					);
					const processWindowStart = performance.now();
					const beforeProcesses = await getProcesses();
					const wallStart = performance.now();
					const frames = await action();
					const wallSeconds = (performance.now() - wallStart) / 1000;
					const afterProcesses = await getProcesses();
					const processWallSeconds =
						(performance.now() - processWindowStart) / 1000;
					const afterMetrics = metricMap(
						await cdp.send("Performance.getMetrics"),
					);
					assert.ok(
						frames.frameCount > 1 && frames.frameTimesMs?.length > 1,
						`${name}: no frame samples`,
					);
					assert.equal(
						frames.initialVisibility,
						"visible",
						`${name}: page was hidden`,
					);
					assert.ok(
						!frames.visibilityChanges?.some(
							(event) => event.state !== "visible",
						),
						`${name}: page lost foreground visibility`,
					);
					const metricWallSeconds =
						afterMetrics.Timestamp - beforeMetrics.Timestamp;
					const taskSeconds =
						afterMetrics.TaskDuration - beforeMetrics.TaskDuration;
					const processes = cpuDelta(beforeProcesses, afterProcesses);
					const measurements = {
						name,
						...frames,
						wallSeconds,
						metricWallSeconds,
						mainThreadTaskSeconds: taskSeconds,
						mainThreadTaskSecondsPerWallSecond: taskSeconds / metricWallSeconds,
						scriptSeconds:
							afterMetrics.ScriptDuration - beforeMetrics.ScriptDuration,
						layoutSeconds:
							afterMetrics.LayoutDuration - beforeMetrics.LayoutDuration,
						recalcStyleSeconds:
							afterMetrics.RecalcStyleDuration -
							beforeMetrics.RecalcStyleDuration,
						jsHeapUsedBytes: afterMetrics.JSHeapUsedSize,
						jsHeapTotalBytes: afterMetrics.JSHeapTotalSize,
						domNodes: afterMetrics.Nodes,
						processCpu: processes,
						processWallSeconds,
						browserCpuSecondsPerWallSecond: processes?.complete
							? processes.totalSeconds / processWallSeconds
							: null,
					};
					console.log(
						`${trial}/${options.trials} ${config.renderer} ${config.view} motion=${config.motion} ${name}: rAF p95=${frames.p95Ms.toFixed(2)}ms, main-thread=${(100 * measurements.mainThreadTaskSecondsPerWallSecond).toFixed(1)}%`,
					);
					phase = "between-scenarios";
					return measurements;
				};
				for (const name of options.scenarios) {
					run.scenarios.push(
						await measure(name, () =>
							page.evaluate(
								({ name, durationMs }) =>
									window.boardBenchmark.runScenario(name, durationMs),
								{ name, durationMs: options.duration },
							),
						),
					);
				}
				if (config.view === "detail") {
					for (const name of [
						"pointer-drag-activity",
						"pointer-reconnect-activity",
					]) {
						const isDrag = name === "pointer-drag-activity";
						const prepared = await page.evaluate(() =>
							window.boardBenchmark.prepareInteraction(),
						);
						const ids = {
							nodeIndex: prepared.node.index,
							edgeIndex: prepared.reconnect.edgeIndex,
						};
						const before = await page.evaluate(({ nodeIndex, edgeIndex }) => {
							const stats = window.boardBenchmark.snapshot();
							return {
								...window.boardBenchmark.interactionSnapshot(
									nodeIndex,
									edgeIndex,
								),
								dragEvents: stats.dragEvents,
								reconnectCount: stats.reconnectCount,
							};
						}, ids);
						const from = isDrag ? prepared.node : prepared.reconnect.from;
						const to = isDrag
							? { x: from.x + 120, y: from.y + 60 }
							: prepared.reconnect.to;
						for (const point of [from, to])
							assert.ok(
								point.x > 0 &&
									point.x < report.viewport.width &&
									point.y > 0 &&
									point.y < report.viewport.height,
								`${name}: pointer outside viewport`,
							);
						if (!isDrag)
							assert.ok(
								before.edge.targetIndex !== prepared.reconnect.targetIndex ||
									before.edge.targetPin !== prepared.reconnect.targetPin,
								"Reconnect must change its endpoint",
							);
						await page.mouse.move(from.x, from.y);
						await page.evaluate(async () => {
							window.boardBenchmark.setActivityReplay(true);
							await new Promise((resolve) =>
								requestAnimationFrame(() => requestAnimationFrame(resolve)),
							);
						});
						let measured;
						try {
							measured = await measure(name, async () => {
								await page.evaluate(() =>
									window.boardBenchmark.beginInputMeasurement(),
								);
								await page.mouse.down();
								try {
									for (let step = 1; step <= 30; step++) {
										await page.mouse.move(
											from.x + ((to.x - from.x) * step) / 30,
											from.y + ((to.y - from.y) * step) / 30,
										);
										await new Promise((resolve) => setTimeout(resolve, 16));
									}
								} finally {
									await page.mouse.up();
								}
								await page.evaluate(
									() =>
										new Promise((resolve) =>
											requestAnimationFrame(() =>
												requestAnimationFrame(resolve),
											),
										),
								);
								return page.evaluate(() =>
									window.boardBenchmark.endInputMeasurement(),
								);
							});
						} finally {
							await page.evaluate(() =>
								window.boardBenchmark.setActivityReplay(false),
							);
						}
						const after = await page.evaluate(({ nodeIndex, edgeIndex }) => {
							const stats = window.boardBenchmark.snapshot();
							return {
								...window.boardBenchmark.interactionSnapshot(
									nodeIndex,
									edgeIndex,
								),
								dragEvents: stats.dragEvents,
								reconnectCount: stats.reconnectCount,
							};
						}, ids);
						assert.ok(
							measured.pointerEvents >= 20,
							`${name}: too few pointer events reached the page`,
						);
						if (isDrag) {
							assert.ok(
								after.dragEvents > before.dragEvents,
								"Pointer drag did not reach the renderer's drag handler",
							);
							assert.ok(
								Math.hypot(
									after.node.x - before.node.x,
									after.node.y - before.node.y,
								) > 20,
								"Pointer drag did not move the node",
							);
						} else {
							assert.equal(
								after.reconnectCount,
								before.reconnectCount + 1,
								"Pointer reconnect must execute exactly once",
							);
							assert.equal(
								after.edge.targetIndex,
								prepared.reconnect.targetIndex,
								"Reconnect target node did not change",
							);
							assert.equal(
								after.edge.targetPin,
								prepared.reconnect.targetPin,
								"Reconnect target pin did not change",
							);
						}
						run.inputScenarios.push({
							...measured,
							name,
							activityReplayHz: 10,
							prepared,
							before,
							after,
						});
					}
				}
				run.after = await inspectPage(page);
				validateVisuals(run.after, config.renderer);
				assert.equal(
					run.consoleErrors.length,
					0,
					`Browser errors: ${run.consoleErrors.join("; ")}`,
				);
				run.status = "passed";
			} catch (error) {
				run.status = "failed";
				run.error = error.stack || error.message;
				process.exitCode = 1;
				console.error(
					`${config.renderer}/${config.view}/motion-${config.motion}: ${error.message}`,
				);
			} finally {
				phase = "cleanup";
				closingPage = true;
				try {
					if (!page.isClosed()) await page.close();
				} catch (error) {
					browserEvent("page-close-error", { message: error.message });
				} finally {
					await save();
				}
			}
		}
	}
} catch (error) {
	report.error = error.stack || error.message;
	process.exitCode = 1;
	console.error(error);
} finally {
	report.finishedAt = new Date().toISOString();
	const grouped = new Map();
	for (const run of report.runs.filter((entry) => entry.status === "passed")) {
		for (const scenario of [...run.scenarios, ...run.inputScenarios]) {
			const key = `${run.renderer}/${run.view}/motion-${run.motion}/${scenario.name}`;
			const group = grouped.get(key) ?? {
				renderer: run.renderer,
				renderMode: run.renderMode,
				view: run.view,
				motion: run.motion,
				scenario: scenario.name,
				trials: [],
			};
			group.trials.push(scenario);
			grouped.set(key, group);
		}
	}
	report.summary = [...grouped.entries()].map(([key, group]) => ({
		key,
		renderer: group.renderer,
		renderMode: group.renderMode,
		view: group.view,
		motion: group.motion,
		scenario: group.scenario,
		completedTrials: group.trials.length,
		complete: group.trials.length === options.trials,
		medianTrialP50Ms: percentile(
			group.trials.map((entry) => entry.p50Ms),
			0.5,
		),
		medianTrialP95Ms: percentile(
			group.trials.map((entry) => entry.p95Ms),
			0.5,
		),
		medianMainThreadTaskSecondsPerWallSecond: percentile(
			group.trials.map((entry) => entry.mainThreadTaskSecondsPerWallSecond),
			0.5,
		),
		medianMissedFrameRatio: percentile(
			group.trials.map((entry) => entry.missedFrameRatio),
			0.5,
		),
		medianJsHeapUsedBytes: percentile(
			group.trials.map((entry) => entry.jsHeapUsedBytes),
			0.5,
		),
		medianPointerEventToRafP95Ms: percentile(
			group.trials
				.map((entry) => entry.pointerEventToRafP95Ms)
				.filter((value) => typeof value === "number"),
			0.5,
		),
	}));
	await save();
	await browser.close();
}
console.log(`Benchmark results: ${join(options.output, "results.json")}`);
