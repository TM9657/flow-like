import type { IHomeLayout } from "../../../home/types";

export type HomeDataSourceKind = "table" | "ontology" | "query";

export type HomeDataScope = "project" | "personal";

export interface HomeToolIssue {
	severity: "error" | "warning";
	code: string;
	path: string;
	message: string;
}

export interface HomeLayoutValidationResult {
	status: "ok" | "validation_error";
	valid: boolean;
	issues: HomeToolIssue[];
	layout?: IHomeLayout;
	canonical_layout?: IHomeLayout;
	fingerprint?: string;
	byte_count?: number;
}

export type PublicHomeLayoutValidationResult = Omit<
	HomeLayoutValidationResult,
	"layout"
>;

export interface HomeReferenceValidationOptions {
	profileAppIds: Set<string>;
}
