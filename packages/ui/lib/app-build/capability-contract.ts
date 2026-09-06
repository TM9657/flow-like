import { z } from "zod";

import {
	type AppBehaviorScenario,
	type AppRequirement,
	type AppResource,
	type JsonObject,
	type JsonValue,
	jsonObjectSchema,
	logicalKeySchema,
	tableColumnSchema,
} from "./contract";
import {
	APP_BEHAVIOR_SCENARIO_SCHEMA,
	type AppScenarioAssertion,
	type AppScenarioJsonAssertionOperator,
} from "./scenario-contract";

export const APP_CAPABILITY_FRAGMENT_SCHEMA =
	"flowpilot.app-capability-fragment/v1" as const;

export interface AppCapabilityFragment {
	readonly schema: typeof APP_CAPABILITY_FRAGMENT_SCHEMA;
	readonly capability: {
		readonly id: AppCapabilityId;
		readonly version: string;
		readonly fingerprint: string;
		readonly parameters: JsonObject;
	};
	readonly generation: {
		readonly mode: "specialist";
		readonly brief: string;
		readonly exact_template_source: false;
		readonly evidence_limitations: readonly string[];
	};
	readonly requirements: readonly AppRequirement[];
	readonly resources: readonly AppResource[];
	readonly scenarios: readonly AppBehaviorScenario[];
}

const capabilityFieldSchema = tableColumnSchema;
export type AppCapabilityField = z.infer<typeof capabilityFieldSchema>;

const capabilityBaseSchema = z.object({
	namespace: logicalKeySchema,
	page_name: z.string().trim().min(1).max(128),
	route: z.string().trim().min(1).max(256).startsWith("/"),
});

export const crudParametersSchema = capabilityBaseSchema
	.extend({
		entity_name: z.string().trim().min(1).max(80),
		table_name: z
			.string()
			.trim()
			.min(1)
			.max(128)
			.regex(/^[a-z][a-z0-9_]*$/),
		fields: z.array(capabilityFieldSchema).min(1).max(64),
		sample_record: jsonObjectSchema,
		sample_update: jsonObjectSchema,
	})
	.strict()
	.superRefine((parameters, context) => {
		validateCapabilityFields(
			parameters.fields,
			parameters.sample_record,
			["id", "created_at", "updated_at"],
			context,
			["fields"],
			["sample_record"],
		);
		validateSampleUpdate(parameters.fields, parameters.sample_update, context, [
			"sample_update",
		]);
	});

export const approvalParametersSchema = capabilityBaseSchema
	.extend({
		request_name: z.string().trim().min(1).max(80),
		requests_table_name: z
			.string()
			.trim()
			.min(1)
			.max(128)
			.regex(/^[a-z][a-z0-9_]*$/),
		audit_table_name: z
			.string()
			.trim()
			.min(1)
			.max(128)
			.regex(/^[a-z][a-z0-9_]*$/),
		request_fields: z.array(capabilityFieldSchema).min(1).max(64),
		sample_request: jsonObjectSchema,
		requester_id: z.string().trim().min(1).max(128),
		approver_id: z.string().trim().min(1).max(128),
	})
	.strict()
	.superRefine((parameters, context) => {
		if (parameters.requests_table_name === parameters.audit_table_name) {
			context.addIssue({
				code: z.ZodIssueCode.custom,
				message: "requests and audit tables must be distinct",
				path: ["audit_table_name"],
			});
		}
		validateCapabilityFields(
			parameters.request_fields,
			parameters.sample_request,
			["id", "status", "requester_id", "submitted_at", "updated_at", "version"],
			context,
			["request_fields"],
			["sample_request"],
		);
	});

export type CrudFoundationParameters = z.infer<typeof crudParametersSchema>;
export type ApprovalFoundationParameters = z.infer<
	typeof approvalParametersSchema
>;

export interface AppCapabilityParametersById {
	readonly "crud.foundation": CrudFoundationParameters;
	readonly "approval.foundation": ApprovalFoundationParameters;
}

export type AppCapabilityId = keyof AppCapabilityParametersById;

export interface AppCapabilitySummary {
	readonly id: AppCapabilityId;
	readonly version: "1.0.0";
	readonly fingerprint: string;
	readonly title: string;
	readonly description: string;
	readonly parameters: JsonObject;
	readonly generation_mode: "specialist";
	readonly certification: "scaffold_only";
}

export interface AppCapabilityRecipe extends AppCapabilitySummary {
	readonly content: JsonObject;
	readonly fingerprint: string;
}

export interface CapabilityDefinition<K extends AppCapabilityId> {
	readonly id: K;
	readonly version: "1.0.0";
	readonly title: string;
	readonly description: string;
	readonly parameters: JsonObject;
	readonly content: JsonObject;
	/** Literal digest pinned in source; lookup fails if recipe content drifts. */
	readonly fingerprint: string;
	readonly schema: z.ZodType<AppCapabilityParametersById[K]>;
	readonly instantiate: (
		parameters: AppCapabilityParametersById[K],
	) => Omit<AppCapabilityFragment, "capability">;
}

function validateCapabilityFields(
	fields: readonly AppCapabilityField[],
	sample: JsonObject,
	reserved: readonly string[],
	context: z.RefinementCtx,
	fieldsPath: readonly (string | number)[],
	samplePath: readonly (string | number)[],
) {
	const names = new Set<string>();
	for (const [index, field] of fields.entries()) {
		if (names.has(field.name)) {
			context.addIssue({
				code: z.ZodIssueCode.custom,
				message: `duplicate capability field ${field.name}`,
				path: [...fieldsPath, index, "name"],
			});
		}
		names.add(field.name);
		if (reserved.includes(field.name)) {
			context.addIssue({
				code: z.ZodIssueCode.custom,
				message: `${field.name} is managed by the recipe`,
				path: [...fieldsPath, index, "name"],
			});
		}
		if (field.nullable !== true && !Object.hasOwn(sample, field.name)) {
			context.addIssue({
				code: z.ZodIssueCode.custom,
				message: `sample is missing required field ${field.name}`,
				path: [...samplePath, field.name],
			});
		}
		if (Object.hasOwn(sample, field.name)) {
			const issue = sampleValueIssue(field, sample[field.name]);
			if (issue) {
				context.addIssue({
					code: z.ZodIssueCode.custom,
					message: issue,
					path: [...samplePath, field.name],
				});
			}
		}
	}
	for (const key of Object.keys(sample)) {
		if (!names.has(key)) {
			context.addIssue({
				code: z.ZodIssueCode.custom,
				message: `sample field ${key} is not declared`,
				path: [...samplePath, key],
			});
		}
	}
}

function validateSampleUpdate(
	fields: readonly AppCapabilityField[],
	update: JsonObject,
	context: z.RefinementCtx,
	path: readonly (string | number)[],
) {
	const fieldsByName = new Map(fields.map((field) => [field.name, field]));
	if (Object.keys(update).length === 0) {
		context.addIssue({
			code: z.ZodIssueCode.custom,
			message: "sample_update must change at least one declared field",
			path: [...path],
		});
	}
	for (const [key, value] of Object.entries(update)) {
		const field = fieldsByName.get(key);
		if (!field) {
			context.addIssue({
				code: z.ZodIssueCode.custom,
				message: `sample update field ${key} is not declared`,
				path: [...path, key],
			});
			continue;
		}
		const issue = sampleValueIssue(field, value);
		if (issue) {
			context.addIssue({
				code: z.ZodIssueCode.custom,
				message: issue,
				path: [...path, key],
			});
		}
	}
}

function sampleValueIssue(
	field: AppCapabilityField,
	value: unknown,
): string | undefined {
	if (value === null) {
		return field.nullable === true
			? undefined
			: `${field.name} is not nullable`;
	}
	if (["string", "binary", "date32", "timestamp:ms:UTC"].includes(field.type)) {
		return typeof value === "string"
			? undefined
			: `${field.name} sample must be a string`;
	}
	if (field.type === "boolean") {
		return typeof value === "boolean"
			? undefined
			: `${field.name} sample must be a boolean`;
	}
	if (field.type === "vector") {
		return Array.isArray(value) &&
			value.length === field.vector_size &&
			value.every(
				(entry) => typeof entry === "number" && Number.isFinite(entry),
			)
			? undefined
			: `${field.name} sample must contain exactly ${field.vector_size} finite numbers`;
	}
	if (typeof value !== "number" || !Number.isFinite(value)) {
		return `${field.name} sample must be a finite number`;
	}
	if (
		(field.type.includes("int") || field.type.includes("uint")) &&
		!Number.isInteger(value)
	) {
		return `${field.name} sample must be an integer`;
	}
	if (field.type.includes("uint") && value < 0) {
		return `${field.name} sample must be non-negative`;
	}
	return undefined;
}

export function requirement(id: string, description: string): AppRequirement {
	return { id, description };
}

export function baseColumns(): AppCapabilityField[] {
	return [
		{ name: "id", type: "string", nullable: false },
		{ name: "created_at", type: "timestamp:ms:UTC", nullable: false },
		{ name: "updated_at", type: "timestamp:ms:UTC", nullable: false },
	];
}

export function runtimeScenarioAssertions(
	stepId: string,
	stateAssertions: readonly {
		readonly id: string;
		readonly path: string;
		readonly operator: AppScenarioJsonAssertionOperator;
		readonly expected?: JsonValue;
	}[],
): AppScenarioAssertion[] {
	return [
		{
			id: `${stepId}_run_succeeded`,
			kind: "run_outcome",
			step_id: stepId,
			run_id_path: "/run_id",
		},
		...stateAssertions.map((assertion) => ({
			id: `${stepId}_${assertion.id}`,
			kind: "state" as const,
			step_id: stepId,
			path: assertion.path,
			operator: assertion.operator,
			...(Object.hasOwn(assertion, "expected")
				? { expected: assertion.expected }
				: {}),
		})),
	];
}

export function escapeJsonPointerSegment(value: string): string {
	return value.replace(/~/g, "~0").replace(/\//g, "~1");
}

export function interfaceScenario(
	namespace: string,
	targetKey: string,
	requirementId: string,
): AppBehaviorScenario {
	return {
		schema: APP_BEHAVIOR_SCENARIO_SCHEMA,
		id: `${namespace}.interface_contract`,
		description:
			"Verify that the exact active headless interface is discoverable.",
		requirement_ids: [requirementId],
		target: { resource_key: targetKey, kind: "event" },
		steps: [{ id: "describe", tool: "describe_app_interface", arguments: {} }],
		assertions: [
			{
				id: "describe_ok",
				kind: "json",
				step_id: "describe",
				path: "/status",
				operator: "equals",
				expected: "ok",
			},
			{
				id: "event_active",
				kind: "json",
				step_id: "describe",
				path: "/event/active",
				operator: "equals",
				expected: true,
			},
			{
				id: "consumer_exact",
				kind: "json",
				step_id: "describe",
				path: "/event/consumer_tool",
				operator: "equals",
				expected: "call_app_event",
			},
		],
	};
}
