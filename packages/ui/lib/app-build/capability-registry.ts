import type {
	AppCapabilityFragment,
	AppCapabilityId,
	AppCapabilityParametersById,
	AppCapabilityRecipe,
	AppCapabilitySummary,
	CapabilityDefinition,
} from "./capability-contract";
import { appCapabilityDefinitions } from "./capability-foundations";
import {
	APP_SPEC_SCHEMA_VERSION,
	type AppSpec,
	type JsonObject,
	appSpecSchema,
} from "./contract";
import { appBuildFingerprint } from "./fingerprint";

function recipeFingerprint(
	definition: (typeof appCapabilityDefinitions)[number],
): string {
	const actual = appBuildFingerprint(
		"app-capability-recipe",
		definition.content,
	);
	if (actual !== definition.fingerprint) {
		throw new Error(
			`Capability registry fingerprint drift for ${definition.id}@${definition.version}.`,
		);
	}
	return definition.fingerprint;
}

export function listAppCapabilities(): AppCapabilitySummary[] {
	return appCapabilityDefinitions.map((definition) => ({
		id: definition.id,
		version: definition.version,
		fingerprint: recipeFingerprint(definition),
		title: definition.title,
		description: definition.description,
		parameters: definition.parameters,
		generation_mode: "specialist",
		certification: "scaffold_only",
	}));
}

export function lookupAppCapability(
	id: string,
	version: string,
): AppCapabilityRecipe {
	if (!/^\d+\.\d+\.\d+$/.test(version)) {
		throw new Error(`Capability version must be exact; received '${version}'.`);
	}
	const definition = appCapabilityDefinitions.find(
		(candidate) => candidate.id === id && candidate.version === version,
	);
	if (!definition) {
		throw new Error(`Unknown app capability ${id}@${version}.`);
	}
	return {
		id: definition.id,
		version: definition.version,
		fingerprint: recipeFingerprint(definition),
		title: definition.title,
		description: definition.description,
		parameters: definition.parameters,
		generation_mode: "specialist",
		certification: "scaffold_only",
		content: definition.content,
	};
}

export function instantiateAppCapability<K extends AppCapabilityId>(
	id: K,
	version: "1.0.0",
	parameters: AppCapabilityParametersById[K],
	expectedFingerprint?: string,
): AppCapabilityFragment;
export function instantiateAppCapability(
	id: string,
	version: string,
	parameters: unknown,
	expectedFingerprint?: string,
): AppCapabilityFragment;
export function instantiateAppCapability(
	id: string,
	version: string,
	parameters: unknown,
	expectedFingerprint?: string,
): AppCapabilityFragment {
	const recipe = lookupAppCapability(id, version);
	if (
		expectedFingerprint !== undefined &&
		expectedFingerprint !== recipe.fingerprint
	) {
		throw new Error(
			`Capability fingerprint mismatch for ${id}@${version}: expected ${expectedFingerprint}, registry has ${recipe.fingerprint}.`,
		);
	}
	const definition = appCapabilityDefinitions.find(
		(candidate) => candidate.id === id && candidate.version === version,
	) as CapabilityDefinition<AppCapabilityId> | undefined;
	if (!definition) throw new Error(`Unknown app capability ${id}@${version}.`);
	const normalized = definition.schema.parse(parameters);
	const fragment = definition.instantiate(normalized);
	return {
		...fragment,
		capability: {
			id: definition.id,
			version,
			fingerprint: recipe.fingerprint,
			parameters: normalized as JsonObject,
		},
	};
}

export function composeAppCapabilityFragments(
	app: { readonly name: string; readonly description?: string },
	fragments: readonly AppCapabilityFragment[],
): AppSpec {
	const selections = new Set<string>();
	const requirementIds = new Set<string>();
	const resourceKeys = new Set<string>();
	const scenarioIds = new Set<string>();
	for (const fragment of fragments) {
		const selection = `${fragment.capability.id}@${fragment.capability.version}`;
		if (selections.has(selection)) {
			throw new Error(`Capability selection ${selection} is duplicated.`);
		}
		selections.add(selection);
		const recipe = lookupAppCapability(
			fragment.capability.id,
			fragment.capability.version,
		);
		if (recipe.fingerprint !== fragment.capability.fingerprint) {
			throw new Error(
				`Capability fragment ${selection} has a stale fingerprint.`,
			);
		}
		for (const requirement of fragment.requirements) {
			if (requirementIds.has(requirement.id)) {
				throw new Error(
					`Capability requirement ${requirement.id} is duplicated.`,
				);
			}
			requirementIds.add(requirement.id);
		}
		for (const resource of fragment.resources) {
			if (resourceKeys.has(resource.key)) {
				throw new Error(`Capability resource ${resource.key} is duplicated.`);
			}
			resourceKeys.add(resource.key);
		}
		for (const scenario of fragment.scenarios) {
			if (scenarioIds.has(scenario.id)) {
				throw new Error(`Capability scenario ${scenario.id} is duplicated.`);
			}
			scenarioIds.add(scenario.id);
		}
	}
	return appSpecSchema.parse({
		schema_version: APP_SPEC_SCHEMA_VERSION,
		name: app.name,
		...(app.description ? { description: app.description } : {}),
		requirements: fragments.flatMap((fragment) => fragment.requirements),
		resources: fragments.flatMap((fragment) => fragment.resources),
		scenarios: fragments.flatMap((fragment) => fragment.scenarios),
	});
}
