import type { IPage } from "../../state/backend-state/page-state";
import type { IWidget } from "../../state/backend-state/widget-state";
import type { IEvent } from "../schema/flow/event";
import type { CompiledAppResource, CompiledAppSpec } from "./compiler";
import { appBuildFingerprint } from "./fingerprint";
import type { AppBuildLifecycleBackend, ExactVersion } from "./staging-types";

export function exactVersion(
	value: readonly number[],
	label: string,
): ExactVersion {
	const [major, minor, patch] = value;
	if (
		value.length !== 3 ||
		major === undefined ||
		minor === undefined ||
		patch === undefined ||
		!value.every((part) => Number.isInteger(part) && part >= 0)
	) {
		throw new Error(`${label} returned an invalid version.`);
	}
	return [major, minor, patch];
}

function withoutVolatileFields<T extends object>(
	value: T,
	keys: readonly string[],
): Record<string, unknown> {
	return Object.fromEntries(
		Object.entries(value).filter(([key]) => !keys.includes(key)),
	);
}

export async function boardContentFingerprint(
	backend: AppBuildLifecycleBackend,
	appId: string,
	boardId: string,
	version?: ExactVersion,
): Promise<string> {
	const [board, source] = await Promise.all([
		backend.boardState.getBoardAuthoritative(appId, boardId, version),
		backend.boardState.getFlowScriptAuthoritative(appId, boardId, version),
	]);
	return appBuildFingerprint("app-build-promotion-board-v1", {
		board: withoutVolatileFields(board, [
			"created_at",
			"updated_at",
			"version",
			"hash",
		]),
		source,
	});
}

export function pageContentFingerprint(page: IPage): string {
	return appBuildFingerprint(
		"app-build-promotion-page-v1",
		withoutVolatileFields(page, ["createdAt", "updatedAt", "version"]),
	);
}

export function widgetContentFingerprint(widget: IWidget): string {
	return appBuildFingerprint(
		"app-build-promotion-widget-v1",
		withoutVolatileFields(widget, ["createdAt", "updatedAt", "version"]),
	);
}

export function eventContentFingerprint(event: IEvent): string {
	return appBuildFingerprint(
		"app-build-promotion-event-v1",
		withoutVolatileFields(event, [
			"active",
			"board_version",
			"created_at",
			"updated_at",
			"event_version",
		]),
	);
}

export function resourceByKey(
	plan: CompiledAppSpec,
	key: string,
): CompiledAppResource {
	const resource = plan.resources.find((candidate) => candidate.key === key);
	if (!resource) {
		throw new Error(`Promotion references unknown resource '${key}'.`);
	}
	return resource;
}

export function eventBoardResource(
	plan: CompiledAppSpec,
	event: Extract<CompiledAppResource, { kind: "event" }>,
): Extract<CompiledAppResource, { kind: "board" }> {
	let boardKey = event.config.board;
	if (!boardKey && event.config.page) {
		const page = resourceByKey(plan, event.config.page);
		if (page.kind !== "page") {
			throw new Error(`Event '${event.key}' references a non-page resource.`);
		}
		boardKey = page.config.board;
	}
	if (!boardKey) throw new Error(`Event '${event.key}' has no board to pin.`);
	const board = resourceByKey(plan, boardKey);
	if (board.kind !== "board") {
		throw new Error(`Event '${event.key}' references a non-board resource.`);
	}
	return board;
}
