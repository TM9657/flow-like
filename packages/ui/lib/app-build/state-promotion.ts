import { readiness } from "./readiness";
import {
	promotionReceiptSchema,
	type AppBuildState,
	type PromotionReceipt,
} from "./state-contract";
import { nextBuildRevision } from "./state-internals";

export function beginPromotion(
	build: AppBuildState,
	options: {
		readonly operation_id: string;
		readonly operation_owner_id: string;
		readonly lease_expires_at_ms: number;
		readonly now_ms?: number;
	},
): AppBuildState {
	const state = readiness(build);
	if (!state.can_promote || build.validation_revision === undefined) {
		throw new Error("Build is not ready for promotion.");
	}
	const nowMs = options.now_ms ?? Date.now();
	if (options.lease_expires_at_ms <= nowMs) {
		throw new Error("A promotion operation lease must expire in the future.");
	}
	return nextBuildRevision(
		{
			...build,
			promotion: {
				status: "promoting",
				attempts: build.promotion.attempts + 1,
				requested_build_revision: build.validation_revision,
				active_operation: {
					operation_id: options.operation_id,
					owner_id: options.operation_owner_id,
					started_at_ms: nowMs,
					expires_at_ms: options.lease_expires_at_ms,
				},
			},
		},
		nowMs,
	);
}

export function recordPromotionResult(
	build: AppBuildState,
	receipt: PromotionReceipt,
	options: { readonly now_ms?: number } = {},
): AppBuildState {
	if (build.promotion.status !== "promoting") {
		throw new Error("Promotion result requires a promoting build.");
	}
	const parsed = promotionReceiptSchema.parse(receipt);
	if (
		!build.promotion.active_operation ||
		parsed.operation_id !== build.promotion.active_operation.operation_id
	) {
		throw new Error("Promotion received a stale operation result.");
	}
	if (
		parsed.build_revision !== build.promotion.requested_build_revision ||
		parsed.spec_fingerprint !== build.spec_fingerprint
	) {
		throw new Error(
			"Promotion receipt targets a stale build revision or spec.",
		);
	}
	const nowMs = options.now_ms ?? Date.now();
	const leaseMatches =
		parsed.recorded_at_ms >= build.promotion.active_operation.started_at_ms &&
		parsed.recorded_at_ms <= build.promotion.active_operation.expires_at_ms &&
		parsed.recorded_at_ms <= nowMs &&
		nowMs <= build.promotion.active_operation.expires_at_ms;
	const acceptedReceipt: PromotionReceipt = leaseMatches
		? parsed
		: {
				...parsed,
				status: "unknown",
				message:
					"Promotion result arrived outside its durable operation lease and cannot be accepted.",
			};
	return nextBuildRevision(
		{
			...build,
			promotion: {
				status:
					acceptedReceipt.status === "applied"
						? "promoted"
						: acceptedReceipt.status,
				attempts: build.promotion.attempts,
				requested_build_revision: build.promotion.requested_build_revision,
				active_operation: undefined,
				receipt: acceptedReceipt,
			},
		},
		nowMs,
	);
}
