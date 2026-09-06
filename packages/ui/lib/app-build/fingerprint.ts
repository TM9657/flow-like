import { stableStringify } from "../stable-stringify";

const FNV_64_PRIME = 0x100000001b3n;
const FNV_64_MASK = 0xffffffffffffffffn;
const FNV_64_OFFSET_A = 0xcbf29ce484222325n;
const FNV_64_OFFSET_B = 0x84222325cbf29ce4n;

function fnv1a64(value: string, offset: bigint): string {
	let hash = offset;
	for (const byte of new TextEncoder().encode(value)) {
		hash ^= BigInt(byte);
		hash = (hash * FNV_64_PRIME) & FNV_64_MASK;
	}
	return hash.toString(16).padStart(16, "0");
}

/**
 * Stable, compact change detector for JSON contracts.
 *
 * This is deliberately versioned and is not an authentication primitive. Callers use it to
 * compare desired state and bind host-issued receipts, never to authorize an operation.
 */
export function appBuildFingerprint(namespace: string, value: unknown): string {
	const canonical = stableStringify(value);
	if (typeof canonical !== "string") {
		throw new TypeError(
			"App build fingerprints require a JSON-serializable value.",
		);
	}
	const input = `${namespace}\u0000${canonical}`;
	return `fp1:${fnv1a64(`a\u0000${input}`, FNV_64_OFFSET_A)}${fnv1a64(
		`b\u0000${input}`,
		FNV_64_OFFSET_B,
	)}`;
}

export function reserveAppResourceId(
	appId: string,
	buildId: string,
	kind: string,
	logicalKey: string,
): string {
	const digest = appBuildFingerprint("resource-id", {
		app_id: appId,
		build_id: buildId,
		kind,
		logical_key: logicalKey,
	}).slice("fp1:".length, "fp1:".length + 24);
	return `fp_${kind}_${digest}`;
}
