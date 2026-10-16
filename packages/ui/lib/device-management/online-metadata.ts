import { sha256 } from "@noble/hashes/sha2";
import type { IBackendState } from "../../state/backend-state";
import type { IProfile } from "../../types";
import type { IApp } from "../schema/app/app";
import { IBitTypes } from "../schema/bit/bit";
import type { ArtifactInput } from "./artifacts";
import {
	type DeploymentCatalog,
	type EligibilityEvent,
	HUB_EXPORT_TYPES,
	approvedOnlineCatalog,
} from "./deployment";

export type ApprovedOnlineMetadata = {
	app: IApp;
	file: ArtifactInput;
	sha256: string;
	/** Deployment choices derived from exactly these approved bytes. */
	catalog: DeploymentCatalog;
};

function record(value: unknown): Record<string, unknown> {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}

function requiresCompletionModel(documents: Record<string, unknown>): boolean {
	return Object.entries(documents).some(([path, value]) => {
		if (!/^(boards|templates)\/[^/]+\/versions\/\d+\/\d+\/\d+$/.test(path))
			return false;
		const board = record(value);
		const nodes = [
			...Object.values(record(board.nodes)),
			...Object.values(record(board.layers)).flatMap((layer) =>
				Object.values(record(record(layer).nodes)),
			),
		];
		return nodes.some(
			(node) => record(node).name === "ai_generative_find_model",
		);
	});
}

/** Check the selected root Bits after model edits and before packaging either export format. */
export async function validateOnlineCompletionModelDependencies(
	metadata: ApprovedOnlineMetadata,
	backend: IBackendState,
	signal?: AbortSignal,
): Promise<void> {
	signal?.throwIfAborted();
	const bundle = JSON.parse(
		new TextDecoder().decode(await metadata.file.file.arrayBuffer()),
	);
	if (!requiresCompletionModel(record(bundle.documents))) return;
	for (const reference of new Set(metadata.app.bits)) {
		signal?.throwIfAborted();
		const split = reference.lastIndexOf(":");
		const id = reference.slice(split + 1);
		const hub = split < 0 ? undefined : reference.slice(0, split);
		const bit = await backend.bitState.getBit(id, hub);
		if (
			bit.id === id &&
			(hub === undefined || bit.hub === hub) &&
			(bit.type === IBitTypes.Llm || bit.type === IBitTypes.Vlm)
		)
			return;
	}
	throw new Error(
		"This deployment uses Find Model but includes no language or vision-language model. Add a completion model to the project's model dependencies and prepare the deployment again. Models in your profile alone are not included.",
	);
}

/** One export resolves at most this many events that follow Latest (the request is a GET). */
export const MAX_LATEST_EVENTS = 64;

type ExportType = (typeof HUB_EXPORT_TYPES)[number];

/**
 * The `types` of an export: the event types without a Page that a hub exports
 * only when asked, among the events this deploy sends and those the service
 * already has. Sorted and without duplicates; empty for a hub without
 * `event_types`, which knows no such parameter.
 */
export function exportTypes(
	events: readonly Pick<EligibilityEvent, "event_type" | "default_page_id">[],
	hubTypes: readonly string[] | undefined,
): ExportType[] {
	if (!hubTypes) return [];
	const named = new Set<ExportType>();
	for (const event of events)
		if (
			!event.default_page_id &&
			(HUB_EXPORT_TYPES as readonly string[]).includes(event.event_type)
		)
			named.add(event.event_type as ExportType);
	return [...named].sort();
}

/**
 * The export's path. `latest` names the events that follow the Latest flow and
 * are part of this deploy: only those are pinned to the version that equals
 * their flow. `types` names the types of `exportTypes`. Sorted, so the same
 * choice gives the same request; anything that keeps a bundle keys it by this.
 */
export function deviceMetadataPath(
	project: string,
	latest: readonly string[] = [],
	types: readonly string[] = [],
): string {
	const names = [...new Set(latest)].sort();
	if (names.length > MAX_LATEST_EVENTS)
		throw new Error(
			`One deploy can resolve ${MAX_LATEST_EVENTS} events that follow Latest, and this one names ${names.length}. Deploy fewer at a time, or pin some in Events.`,
		);
	const kinds = [...new Set(types)].sort();
	const unknown = kinds.find(
		(type) => !(HUB_EXPORT_TYPES as readonly string[]).includes(type),
	);
	if (unknown !== undefined)
		throw new Error(
			`The hub's export takes the event types ${HUB_EXPORT_TYPES.join(", ")}, not ${JSON.stringify(unknown)}.`,
		);
	const query = [
		...(names.length
			? [`latest=${names.map(encodeURIComponent).join(",")}`]
			: []),
		...(kinds.length ? [`types=${kinds.join(",")}`] : []),
	];
	const path = `apps/${project}/device-metadata`;
	return query.length ? `${path}?${query.join("&")}` : path;
}

/** Hash the exact locally held payload. Only the authenticated deployment approves it. */
export async function prepareOnlineMetadata(
	project: string,
	backend: IBackendState,
	profile: IProfile,
	signal?: AbortSignal,
	latest: readonly string[] = [],
	types: readonly string[] = [],
): Promise<ApprovedOnlineMetadata> {
	signal?.throwIfAborted();
	const bundle = await backend.apiState.get<{
		version: number;
		project_id: string;
		documents: Record<string, unknown>;
	}>(profile, deviceMetadataPath(project, latest, types));
	signal?.throwIfAborted();
	const app = bundle?.documents?.app as IApp | undefined;
	if (
		bundle?.version !== 1 ||
		bundle.project_id !== project ||
		!app ||
		app.id !== project ||
		app.visibility === "Offline" ||
		Object.keys(bundle.documents).length > 1024 ||
		Object.keys(bundle).some(
			(key) => !["version", "project_id", "documents"].includes(key),
		)
	)
		throw new Error(
			"The server returned an invalid executable metadata snapshot. Update the server and prepare this project again.",
		);
	const bytes = new TextEncoder().encode(JSON.stringify(bundle));
	if (bytes.length > 32 * 1024 * 1024)
		throw new Error("Executable metadata exceeds the 32 MiB deployment limit.");
	return {
		app,
		catalog: approvedOnlineCatalog(bundle.documents),
		file: {
			path: `apps/${project}/online-metadata.json`,
			file: new Blob([bytes]),
		},
		sha256: Array.from(sha256(bytes), (byte) =>
			byte.toString(16).padStart(2, "0"),
		).join(""),
	};
}
