import {
	chmod,
	lstat,
	mkdtemp,
	readFile,
	readdir,
	realpath,
	writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

const MARKER = ".flowpilot-e2e-owner.json";
const SCHEMA = "flowpilot-e2e-isolation/v1";

export interface FlowPilotE2EIsolation {
	root: string;
	runId: string;
	env: Record<string, string>;
	config: {
		identifier: string;
		app: {
			windows: {
				label: string;
				title: string;
				width: number;
				height: number;
				incognito: boolean;
			}[];
		};
		plugins: { "deep-link": { desktop: { schemes: string[] } } };
	};
}

export async function createFlowPilotE2EIsolation(
	runId: string,
): Promise<FlowPilotE2EIsolation> {
	if (!/^e2e_[0-9]+_[0-9a-f]{16}$/.test(runId))
		throw new Error("Invalid isolated E2E run ID.");
	const temporary = await realpath(tmpdir());
	const root = await mkdtemp(join(temporary, "flow-like-flowpilot-e2e-"));
	await chmod(root, 0o700);
	await writeFile(
		join(root, MARKER),
		JSON.stringify({ schema: SCHEMA, run_id: runId, runner_pid: process.pid }),
		{ flag: "wx", mode: 0o600 },
	);
	return {
		root,
		runId,
		env: {
			FLOWPILOT_E2E_DATA_ROOT: root,
			CACHE_DIR: join(root, "cache"),
			FLOW_LIKE_FLOWPILOT_DRAFT_DIR: join(
				root,
				"projects",
				".flowpilot-drafts",
			),
		},
		config: {
			identifier: "com.flow-like.e2e",
			app: {
				windows: [
					{
						label: "main",
						title: "FlowPilot E2E",
						width: 1440,
						height: 1000,
						incognito: true,
					},
				],
			},
			plugins: { "deep-link": { desktop: { schemes: ["flow-like-e2e"] } } },
		},
	};
}

export async function verifyFlowPilotE2EIsolation(
	isolation: FlowPilotE2EIsolation,
): Promise<void> {
	const { root, runId } = isolation;
	if (
		(await realpath(root)) !== root ||
		dirname(root) !== (await realpath(tmpdir())) ||
		!basename(root).startsWith("flow-like-flowpilot-e2e-")
	) {
		throw new Error(
			"Isolated E2E storage must be a canonical temporary-directory child.",
		);
	}
	const rootStat = await lstat(root);
	const markerStat = await lstat(join(root, MARKER));
	if (
		!rootStat.isDirectory() ||
		!markerStat.isFile() ||
		markerStat.size > 4096 ||
		(process.platform !== "win32" &&
			((rootStat.mode & 0o077) !== 0 ||
				(markerStat.mode & 0o077) !== 0 ||
				rootStat.uid !== process.getuid?.() ||
				markerStat.uid !== rootStat.uid))
	) {
		throw new Error(
			"Isolated E2E storage and marker must be private and owned by this user.",
		);
	}
	const owner = JSON.parse(await readFile(join(root, MARKER), "utf8"));
	if (
		owner.schema !== SCHEMA ||
		owner.run_id !== runId ||
		owner.runner_pid !== process.pid ||
		(await readdir(root)).some((entry) => entry !== MARKER)
	) {
		throw new Error(
			"Isolated E2E storage is not fresh or its owner does not match this runner.",
		);
	}
	if (
		isolation.config.identifier !== "com.flow-like.e2e" ||
		isolation.config.app.windows.length !== 1 ||
		isolation.config.app.windows[0]?.label !== "main" ||
		!isolation.config.app.windows[0]?.incognito ||
		isolation.env.FLOWPILOT_E2E_DATA_ROOT !== root ||
		isolation.env.CACHE_DIR !== join(root, "cache") ||
		isolation.env.FLOW_LIKE_FLOWPILOT_DRAFT_DIR !==
			join(root, "projects", ".flowpilot-drafts")
	) {
		throw new Error(
			"Isolated E2E configuration does not match its private data root.",
		);
	}
}
