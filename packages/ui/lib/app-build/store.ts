import {
	appBuildStateSchema,
	validateBuild,
	type AppBuildState,
} from "./state";

export interface AppBuildRef {
	readonly app_id: string;
	readonly build_id: string;
}

/** Durable compare-and-swap boundary used by the orchestration engine. */
export interface AppBuildStore {
	read(ref: AppBuildRef): Promise<AppBuildState | null>;
	create(build: AppBuildState): Promise<void>;
	compareAndSwap(
		ref: AppBuildRef,
		expected_revision: number,
		next: AppBuildState,
	): Promise<void>;
}

/** Narrow shape implemented by desktop and web backend bridges. */
export interface AppBuildStorageBackend {
	readAppBuild(appId: string, buildId: string): Promise<unknown | null>;
	writeAppBuild(
		appId: string,
		buildId: string,
		record: AppBuildState,
		expectedRevision: number | null,
	): Promise<void>;
}

function assertRef(ref: AppBuildRef, build: AppBuildState): void {
	if (build.app_id !== ref.app_id || build.build_id !== ref.build_id) {
		throw new Error("AppBuildState does not match its durable storage key.");
	}
}

function parseValidBuild(value: unknown, ref?: AppBuildRef): AppBuildState {
	const parsed = appBuildStateSchema.parse(value);
	if (ref) assertRef(ref, parsed);
	const validation = validateBuild(parsed);
	if (!validation.ok) {
		throw new Error(
			`Invalid durable AppBuildState: ${validation.issues
				.map((issue) => issue.message)
				.join("; ")}`,
		);
	}
	return validation.build;
}

export function createAppBuildStore(
	backend: AppBuildStorageBackend,
): AppBuildStore {
	return {
		async read(ref) {
			const value = await backend.readAppBuild(ref.app_id, ref.build_id);
			return value === null ? null : parseValidBuild(value, ref);
		},
		async create(build) {
			const parsed = parseValidBuild(build);
			if (parsed.revision !== 0) {
				throw new Error(
					"A newly created AppBuildState must start at revision 0.",
				);
			}
			await backend.writeAppBuild(parsed.app_id, parsed.build_id, parsed, null);
		},
		async compareAndSwap(ref, expectedRevision, next) {
			const parsed = parseValidBuild(next, ref);
			if (parsed.revision !== expectedRevision + 1) {
				throw new Error(
					`CAS revision must advance exactly once from ${expectedRevision} to ${expectedRevision + 1}.`,
				);
			}
			await backend.writeAppBuild(
				ref.app_id,
				ref.build_id,
				parsed,
				expectedRevision,
			);
		},
	};
}

export async function readRequiredBuild(
	store: AppBuildStore,
	ref: AppBuildRef,
): Promise<AppBuildState> {
	const build = await store.read(ref);
	if (!build) {
		throw new Error(
			`App build '${ref.build_id}' does not exist for app '${ref.app_id}'.`,
		);
	}
	assertRef(ref, build);
	return build;
}
