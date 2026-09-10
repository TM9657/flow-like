import type { IPage, IPageState } from "../../state/backend-state/page-state";
import { normalizePageForPersistence } from "../a2ui/style-normalization";

export function widgetSpecialistMessage(message: string | undefined) {
	return message
		? {
				specialist_message: message,
				specialist_message_timing: "before_host_apply",
			}
		: {};
}

function record(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Native serialization adds default fields; every authored field must still read back. */
function retainsAuthoredValue(expected: unknown, actual: unknown): boolean {
	if (expected === undefined) return true;
	if (expected === null) return actual === null || actual === undefined;
	if (Array.isArray(expected))
		return (
			Array.isArray(actual) &&
			expected.length === actual.length &&
			expected.every((value, index) =>
				retainsAuthoredValue(value, actual[index]),
			)
		);
	if (record(expected))
		return (
			record(actual) &&
			Object.entries(expected).every(([key, value]) =>
				retainsAuthoredValue(value, actual[key]),
			)
		);
	return expected === actual;
}

export function assertWidgetPageReadback(expected: IPage, actual: IPage): void {
	for (const key of ["id", "boardId", "name", "route"] as const) {
		if (actual[key] !== expected[key])
			throw new Error(`Authoritative page readback differs at ${key}.`);
	}
	const normalized = normalizePageForPersistence(expected);
	const readback = normalizePageForPersistence(actual);
	for (const key of [
		"components",
		"content",
		"canvasSettings",
		"widgetRefs",
		"layoutType",
	] as const) {
		if (!retainsAuthoredValue(normalized[key], readback[key]))
			throw new Error(`Authoritative page readback differs at ${key}.`);
	}
}

export async function persistFlowPilotWidgetPage(
	pageState: Pick<IPageState, "updatePage" | "getPageAuthoritative">,
	appId: string,
	page: IPage,
	mode: "create" | "edit",
	specialistMessage?: string,
) {
	const target = {
		app_id: appId,
		board_id: page.boardId,
		page: { id: page.id, name: page.name, route: page.route },
		...widgetSpecialistMessage(specialistMessage),
	};
	try {
		await pageState.updatePage(appId, page);
		const persisted = await pageState.getPageAuthoritative(
			appId,
			page.id,
			page.boardId,
		);
		assertWidgetPageReadback(page, persisted);
		return {
			...target,
			status: "ok" as const,
			message: `${mode === "create" ? "Created" : "Updated"} page '${persisted.name}' (${persisted.id}) at '${persisted.route ?? ""}'. The host saved ${persisted.components.length} components and verified the page identity, content, name, and route through authoritative readback.`,
			component_count: persisted.components.length,
			staged: false,
			applied: true,
			persistence_verified: true,
		};
	} catch (error) {
		return {
			...target,
			status: "error" as const,
			code: "FLOWPILOT_WIDGET_PERSISTENCE_UNVERIFIED",
			outcome: "unknown",
			persistence_verified: false,
			message: `The host could not verify the saved page: ${error instanceof Error ? error.message : String(error)}. The write may have completed. Read this exact page before retrying; do not create a replacement.`,
		};
	}
}

export function stagedFlowPilotWidgetReceipt(
	componentCount: number,
	specialistMessage?: string,
) {
	return {
		status: "ok" as const,
		message: `The host staged ${componentCount} UI components for review in the open builder. Apply the pending changes to save them.`,
		component_count: componentCount,
		staged: true,
		applied: false,
		persistence_verified: false,
		...widgetSpecialistMessage(specialistMessage),
	};
}
