/**
 * Whether a `list_apps` result opens FlowPilot's sealed public-web fallback.
 *
 * The question is only whether a local inventory came back, never whether every app in it read
 * cleanly. `list_apps` reports `partial` when an app's Events cannot be loaded, when a current
 * profile app is missing from the backend inventory, or when the listing hits its safety cap — and
 * each of those is still evidence about what exists locally. Demanding a clean `ok` sealed the
 * fallback off for the rest of the run, so `research_agent` failed on every attempt no matter how
 * many times discovery succeeded.
 *
 * A failed, timed-out, or absent result is not an inventory and keeps the gate shut: app absence
 * stays unproven until a listing actually returns.
 *
 * Mirrors `returned_app_inventory_result` in
 * packages/core/editor/src/flow/copilot/platform.rs — both hosts must open the fallback on the same
 * results, or the same run behaves differently depending on which backend serves it.
 */
export function isReturnedAppInventory(result: unknown): boolean {
	if (!result || typeof result !== "object") return false;
	const status = (result as Record<string, unknown>).status;
	return status === "ok" || status === "partial";
}

/**
 * Tools whose results carry no private or user-controlled data. App inventory is on the list because
 * the sealed researcher accepts no model text, so inventory metadata provably cannot cross the
 * outbound boundary; `research_agent` is on it because it is that boundary.
 *
 * Mirrors the exemptions in `platform_tool_enters_private_context`
 * (packages/core/editor/src/flow/copilot/platform.rs).
 */
const NON_PRIVATE_TOOLS = new Set(["list_apps", "research_agent"]);

/**
 * Whether dispatching this tool puts private or user-controlled data into the run's working context.
 * Default-deny: an unrecognised tool counts as private, so a newly added tool cannot silently widen
 * what public research may follow.
 */
export function entersPrivateContext(toolName: string): boolean {
	return !NON_PRIVATE_TOOLS.has(toolName);
}

/** Why the sealed public researcher may not run, or `undefined` when it may. */
export interface SealedResearchRefusal {
	code: string;
	message: string;
}

/** The run-scoped facts the gate decides on. */
export interface SealedResearchRoutingState {
	appInventoryReturned: boolean;
	privateContextEntered: boolean;
	sealedResearchUsed: boolean;
}

const FRESH_RUN: SealedResearchRoutingState = {
	appInventoryReturned: false,
	privateContextEntered: false,
	sealedResearchUsed: false,
};

/**
 * The browser-side half of the sealed-research gate. It must agree with
 * `sealed_research_gate_error` in packages/core/editor/src/flow/copilot/platform.rs: public research
 * is an everyday capability, so a plainly public request runs in the first wave with no inventory
 * round. The host still enforces what the model cannot self-police — once private data is in the
 * run, route locally first — and the one-shot cap.
 *
 * A run that has no id at all is refused. Not because of what it might leak, but because the
 * one-shot cap is recorded against the run id: without one, every dispatch would start another
 * researcher. A run that HAS an id but no state yet is simply a fresh run, and research is its
 * first act — that is the case this whole gate exists to allow.
 */
export function sealedResearchRefusal(
	runId: string | undefined,
	state: SealedResearchRoutingState | undefined,
): SealedResearchRefusal | undefined {
	if (!runId?.trim())
		return {
			code: "sealed_research_unavailable",
			message:
				"This run has no id, so the one-shot public researcher cannot be tracked. Retry in a new turn.",
		};
	const current = state ?? FRESH_RUN;
	if (current.privateContextEntered && !current.appInventoryReturned)
		return {
			code: "local_app_discovery_required",
			message:
				"This run already works with local app or user data, so call list_apps and wait for its result before sealed public research.",
		};
	if (current.sealedResearchUsed)
		return {
			code: "sealed_research_already_used",
			message:
				"The sealed public researcher is one-shot for this run; synthesize its findings and disclose remaining gaps.",
		};
	return undefined;
}
