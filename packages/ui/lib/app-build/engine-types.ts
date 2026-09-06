import type { JsonObject } from "./contract";
import type { CompiledAppResource, CompiledAppSpec } from "./compiler";
import type {
	AppBuildState,
	HostResourceResult,
	PromotionReceipt,
	StructuralEvidence,
} from "./state";

/** Immutable inputs supplied to every resource operation. */
export interface AppBuildHostResourceContext {
	readonly build: AppBuildState;
	readonly plan: CompiledAppSpec;
	readonly resource: CompiledAppResource;
	readonly signal?: AbortSignal;
}

/** Immutable inputs supplied to whole-build validation and promotion. */
export interface AppBuildHostContext {
	readonly build: AppBuildState;
	readonly plan: CompiledAppSpec;
	readonly signal?: AbortSignal;
}

export type HostResourceInspection =
	| {
			readonly status: "matches";
			readonly inspection_id: string;
			readonly physical_id: string;
			readonly desired_fingerprint: string;
			readonly observed_fingerprint: string;
			readonly inspected_at_ms: number;
			readonly resource_revision?: string;
			readonly details?: JsonObject;
	  }
	| {
			readonly status: "missing" | "drifted" | "unknown";
			readonly inspection_id: string;
			readonly physical_id: string;
			readonly desired_fingerprint: string;
			readonly observed_fingerprint?: string;
			readonly inspected_at_ms: number;
			readonly resource_revision?: string;
			readonly message: string;
			readonly details?: JsonObject;
	  };

export interface AppBuildHostAdapter {
	/** Authoritative readback. The engine always calls this before retrying uncertain work. */
	inspectResource(
		context: AppBuildHostResourceContext,
	): Promise<HostResourceInspection>;
	/** Apply one reserved resource. The result must be based on authoritative readback. */
	applyResource(
		context: AppBuildHostResourceContext,
	): Promise<HostResourceResult>;
	/** Validate the complete staged graph against authoritative persisted state. */
	validateStructure(context: AppBuildHostContext): Promise<StructuralEvidence>;
	/** Activate the exact validated revision with host-side journaling and compensation. */
	promote(context: AppBuildHostContext): Promise<PromotionReceipt>;
}

export interface AppBuildEngineOptions {
	readonly signal?: AbortSignal;
	readonly max_parallel?: number;
	readonly now_ms?: () => number;
	/** Stable identity for the controller invocation or process owning an effect lease. */
	readonly operation_owner_id?: string;
	/** Must exceed the host tool deadline. Defaults to eight hours and fifteen minutes. */
	readonly lease_duration_ms?: number;
	/** Test and host hook. It must return a new id on every call. */
	readonly create_operation_id?: () => string;
}
