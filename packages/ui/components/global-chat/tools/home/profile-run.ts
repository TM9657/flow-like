export interface HomeProfileRunScope {
	parentRequestId?: string;
	profileId?: string;
}

export class HomeProfileRunError extends Error {
	readonly result: {
		status: "stale";
		code: string;
		profile_id?: string;
		message: string;
	};

	constructor(code: string, profileId?: string) {
		super(
			"This Home run no longer matches its original profile. Start a new Home request in the selected profile.",
		);
		this.name = "HomeProfileRunError";
		this.result = {
			status: "stale",
			code,
			profile_id: profileId,
			message: this.message,
		};
	}
}

/** Host-owned profile bindings survive individual tool calls and cannot follow a profile switch. */
export class HomeProfileRuns {
	private readonly runs = new Map<
		string,
		{ profileId: string; stale: boolean; finished: boolean }
	>();

	begin(parentRequestId: string, profileId: string) {
		if (!profileId || this.runs.has(parentRequestId)) {
			throw new HomeProfileRunError("home_run_identity_invalid", profileId);
		}
		this.runs.set(parentRequestId, {
			profileId,
			stale: false,
			finished: false,
		});
	}

	finish(parentRequestId: string) {
		// Keep completed Home identities for this bridge's lifetime so late tools stay
		// rejected without mistaking an unrelated specialist's parent for an expired run.
		const run = this.runs.get(parentRequestId);
		if (run) run.finished = true;
	}

	expectedProfile(scope: HomeProfileRunScope): string | undefined {
		const run = scope.parentRequestId
			? this.runs.get(scope.parentRequestId)
			: undefined;
		if (!run) return scope.profileId;
		if (run.finished) {
			throw new HomeProfileRunError("home_run_expired", run.profileId);
		}
		if (run.stale || (scope.profileId && scope.profileId !== run.profileId)) {
			run.stale = true;
			throw new HomeProfileRunError("home_profile_changed", run.profileId);
		}
		return run.profileId;
	}

	assertCurrent(
		scope: HomeProfileRunScope,
		currentProfileId: string | undefined,
	) {
		const expected = this.expectedProfile(scope);
		if (!expected || expected === currentProfileId) return;
		const run = scope.parentRequestId
			? this.runs.get(scope.parentRequestId)
			: undefined;
		if (run) run.stale = true;
		throw new HomeProfileRunError("home_profile_changed", expected);
	}
}

/** Recheck captured and live identities after the asynchronous backend profile read. */
export async function assertHomeProfileRun(
	runs: HomeProfileRuns,
	scope: HomeProfileRunScope,
	readProfileId: () => Promise<string | undefined>,
	readSurfaceProfileId: () => string | undefined,
	capturedProfileId?: string,
) {
	if (!runs.expectedProfile(scope)) return;
	runs.assertCurrent(scope, await readProfileId());
	if (capturedProfileId !== undefined)
		runs.assertCurrent(scope, capturedProfileId);
	const liveProfileId = readSurfaceProfileId();
	if (liveProfileId !== undefined) runs.assertCurrent(scope, liveProfileId);
}
