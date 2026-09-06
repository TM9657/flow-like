import { homeLayoutFingerprint } from "../../../home/home-layout-json";
import type { IHomeLayout } from "../../../home/types";
import type {
	HomeLayoutValidationResult,
	PublicHomeLayoutValidationResult,
} from "./types";

/** Remove the private parsed-layout alias before returning validation to a tool caller. */
export function publicHomeLayoutValidation(
	validation: HomeLayoutValidationResult,
): PublicHomeLayoutValidationResult {
	const { layout, ...publicResult } = validation;
	void layout;
	return publicResult;
}

/** Return comparison metadata without duplicating layouts unless the caller asks for them. */
export function homeLayoutComparisonFields(
	currentLayout: IHomeLayout,
	baseLayout: IHomeLayout,
	defaultLayout: IHomeLayout | undefined,
	includeComparisons: boolean,
) {
	const currentFingerprint = homeLayoutFingerprint(currentLayout);
	const baseFingerprint = homeLayoutFingerprint(baseLayout);
	return {
		base_fingerprint: baseFingerprint,
		...(includeComparisons && baseFingerprint !== currentFingerprint
			? { base_layout: baseLayout }
			: {}),
		...(defaultLayout
			? {
					default_fingerprint: homeLayoutFingerprint(defaultLayout),
					...(includeComparisons &&
					homeLayoutFingerprint(defaultLayout) !== currentFingerprint
						? { default_layout: defaultLayout }
						: {}),
				}
			: {}),
	};
}

const MAX_MISSING_PROFILE_APP_IDS = 100;

/** Detect partial app inventories without trusting a backend's empty-list fallback. */
export function profileAppInventoryCoverage(
	profileAppIds: Iterable<string>,
	returnedAppIds: Iterable<string>,
) {
	const returned = new Set(returnedAppIds);
	const missing = [...profileAppIds]
		.filter((appId) => !returned.has(appId))
		.sort((left, right) => left.localeCompare(right));
	return {
		complete: missing.length === 0,
		missing_count: missing.length,
		missing_profile_app_ids: missing.slice(0, MAX_MISSING_PROFILE_APP_IDS),
		missing_ids_truncated: missing.length > MAX_MISSING_PROFILE_APP_IDS,
	};
}
