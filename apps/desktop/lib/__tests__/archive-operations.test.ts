import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	listen: vi.fn(),
	invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
	listen: mocks.listen,
}));

vi.mock("@tauri-apps/api/core", () => ({
	invoke: mocks.invoke,
}));

import type { TFunction } from "i18next";
import {
	ARCHIVE_PROGRESS_EVENT,
	type ArchivePhase,
	type IArchiveProgress,
	type IArchiveProgressEvent,
	archivePhaseLabel,
	archiveProgressPercent,
	cancelArchiveOperation,
	describeArchiveError,
	isArchiveCancelled,
	isWrongPassword,
	listenArchiveProgress,
	newOperationId,
} from "../archive-operations";

// Verbatim strings from packages/core/runtime/src/app/sharing.rs.
const CANCELLED = "Archive operation cancelled";
const BINDING_TAG = "Archive authentication failed (binding tag mismatch)";
const PASSWORD_REQUIRED = "Password required for encrypted archive";
const PLAIN_FAILURE = "failed to open archive: No such file or directory";
// Verbatim from apps/desktop/src-tauri/src/functions/app/sharing.rs.
const PICKER_DISMISSED = "Export target selection cancelled";

const fallback: TFunction<"common"> = ((
	_key: string,
	defaultValue?: unknown,
) => (typeof defaultValue === "string" ? defaultValue : "")) as never;

const progress = (
	overrides: Partial<IArchiveProgress> = {},
): IArchiveProgress => ({
	phase: "packing",
	done_bytes: 0,
	total_bytes: 0,
	done_files: 0,
	total_files: 0,
	...overrides,
});

beforeEach(() => {
	vi.clearAllMocks();
});

describe("describeArchiveError", () => {
	test("passes a raw string error through unchanged", () => {
		expect(describeArchiveError(CANCELLED)).toBe(CANCELLED);
	});

	test("unwraps an Error instance to its message", () => {
		expect(describeArchiveError(new Error(BINDING_TAG))).toBe(BINDING_TAG);
	});

	test("unwraps the object shapes Tauri command rejections arrive in", () => {
		expect(describeArchiveError({ error: PLAIN_FAILURE })).toBe(PLAIN_FAILURE);
		expect(describeArchiveError({ message: PLAIN_FAILURE })).toBe(
			PLAIN_FAILURE,
		);
	});

	test("prefers `error` over `message` when both are present", () => {
		expect(
			describeArchiveError({ error: CANCELLED, message: PLAIN_FAILURE }),
		).toBe(CANCELLED);
	});

	test("falls back to String() for shapes that carry no text", () => {
		expect(describeArchiveError(undefined)).toBe("undefined");
		expect(describeArchiveError(null)).toBe("null");
		expect(describeArchiveError(42)).toBe("42");
		expect(describeArchiveError({ error: { nested: true } })).toBe(
			"[object Object]",
		);
	});
});

describe("isArchiveCancelled", () => {
	test("recognises the backend cancellation string in every carrier", () => {
		expect(isArchiveCancelled(CANCELLED)).toBe(true);
		expect(isArchiveCancelled(new Error(CANCELLED))).toBe(true);
		expect(isArchiveCancelled({ error: CANCELLED })).toBe(true);
		expect(isArchiveCancelled({ message: CANCELLED })).toBe(true);
	});

	test("matches case-insensitively and when wrapped in anyhow context", () => {
		expect(
			isArchiveCancelled("export failed: Archive Operation CANCELLED"),
		).toBe(true);
	});

	test("treats a dismissed save panel as a cancellation, not a failure", () => {
		expect(isArchiveCancelled(PICKER_DISMISSED)).toBe(true);
		expect(isArchiveCancelled({ error: PICKER_DISMISSED })).toBe(true);
	});

	test("does not fire on other archive failures", () => {
		expect(isArchiveCancelled(PLAIN_FAILURE)).toBe(false);
		expect(isArchiveCancelled(BINDING_TAG)).toBe(false);
		expect(isArchiveCancelled(PASSWORD_REQUIRED)).toBe(false);
		expect(isArchiveCancelled(undefined)).toBe(false);
	});
});

describe("isWrongPassword", () => {
	test("recognises the binding tag mismatch in every carrier", () => {
		expect(isWrongPassword(BINDING_TAG)).toBe(true);
		expect(isWrongPassword(new Error(BINDING_TAG))).toBe(true);
		expect(isWrongPassword({ error: BINDING_TAG })).toBe(true);
		expect(isWrongPassword({ message: BINDING_TAG })).toBe(true);
	});

	test("a missing password is not a wrong password", () => {
		// The dialog shows this verbatim instead of "Wrong password" — it means
		// the archive is encrypted and none was supplied yet.
		expect(isWrongPassword(PASSWORD_REQUIRED)).toBe(false);
	});

	test("does not fire on cancellation or generic failures", () => {
		expect(isWrongPassword(CANCELLED)).toBe(false);
		expect(isWrongPassword(PLAIN_FAILURE)).toBe(false);
		expect(isWrongPassword(null)).toBe(false);
	});

	test("is case sensitive — the marker is copied verbatim from the backend", () => {
		expect(isWrongPassword("BINDING TAG MISMATCH")).toBe(false);
	});
});

describe("archivePhaseLabel", () => {
	const phases: ArchivePhase[] = [
		"compacting",
		"listing",
		"packing",
		"writing",
		"reading",
		"planning",
		"restoring",
		"cleaning",
		"finalizing",
		"done",
	];

	test("every phase the backend can emit resolves to a non-empty label", () => {
		for (const phase of phases) {
			const label = archivePhaseLabel(fallback, phase);
			expect(label, `phase ${phase}`).toBeTruthy();
		}
	});

	test("labels are distinct so the progress view never looks stuck", () => {
		const labels = phases.map((phase) => archivePhaseLabel(fallback, phase));
		expect(new Set(labels).size).toBe(phases.length);
	});

	test("each phase requests its own translation key", () => {
		const t = vi.fn(
			(_key: string, defaultValue: string) => defaultValue,
		) as unknown as TFunction<"common">;
		for (const phase of phases) archivePhaseLabel(t, phase);
		const keys = (t as unknown as ReturnType<typeof vi.fn>).mock.calls.map(
			(call) => call[0],
		);
		expect(new Set(keys).size).toBe(phases.length);
	});
});

describe("archiveProgressPercent", () => {
	test("done is always 100 even when no byte totals were reported", () => {
		expect(archiveProgressPercent(progress({ phase: "done" }))).toBe(100);
	});

	test("bytes win over files when both are known", () => {
		expect(
			archiveProgressPercent(
				progress({
					done_bytes: 25,
					total_bytes: 100,
					done_files: 9,
					total_files: 10,
				}),
			),
		).toBe(25);
	});

	test("falls back to file counts, then to zero", () => {
		expect(
			archiveProgressPercent(progress({ done_files: 1, total_files: 4 })),
		).toBe(25);
		expect(archiveProgressPercent(progress())).toBe(0);
	});

	test("clamps to 100 when the backend overshoots its own estimate", () => {
		expect(
			archiveProgressPercent(progress({ done_bytes: 500, total_bytes: 100 })),
		).toBe(100);
	});
});

describe("newOperationId", () => {
	test("mints a unique id per call", () => {
		const ids = new Set(Array.from({ length: 256 }, () => newOperationId()));
		expect(ids.size).toBe(256);
	});

	test("ids are strings usable as a Tauri command argument", () => {
		const id = newOperationId();
		expect(typeof id).toBe("string");
		expect(id.length).toBeGreaterThan(0);
	});
});

describe("listenArchiveProgress", () => {
	function capture() {
		let handler: (event: { payload: IArchiveProgressEvent }) => void = () => {};
		const unlisten = vi.fn();
		mocks.listen.mockImplementation(
			async (_name: string, cb: typeof handler) => {
				handler = cb;
				return unlisten;
			},
		);
		return {
			emit: (payload: IArchiveProgressEvent) => handler({ payload }),
			unlisten,
		};
	}

	test("subscribes to the shared progress event channel", async () => {
		capture();
		await listenArchiveProgress("op-1", () => {});
		expect(mocks.listen).toHaveBeenCalledWith(
			ARCHIVE_PROGRESS_EVENT,
			expect.any(Function),
		);
	});

	test("delivers only the events carrying the matching operation id", async () => {
		const channel = capture();
		const seen: IArchiveProgress[] = [];
		await listenArchiveProgress("op-1", (value) => seen.push(value));

		const mine = progress({ phase: "packing", done_files: 1, total_files: 2 });
		const theirs = progress({ phase: "restoring", done_files: 7 });

		channel.emit({ operation_id: "op-2", kind: "export", progress: theirs });
		channel.emit({ operation_id: "op-1", kind: "export", progress: mine });
		channel.emit({ operation_id: "", kind: "import", progress: theirs });
		channel.emit({ operation_id: "op-1 ", kind: "export", progress: theirs });

		expect(seen).toEqual([mine]);
	});

	test("a concurrent import and export never cross-feed each other", async () => {
		const channel = capture();
		const exportSeen: ArchivePhase[] = [];
		await listenArchiveProgress("op-export", (value) =>
			exportSeen.push(value.phase),
		);

		channel.emit({
			operation_id: "op-export",
			kind: "export",
			progress: progress({ phase: "packing" }),
		});
		channel.emit({
			operation_id: "op-import",
			kind: "import",
			progress: progress({ phase: "restoring" }),
		});
		channel.emit({
			operation_id: "op-export",
			kind: "export",
			progress: progress({ phase: "done" }),
		});

		expect(exportSeen).toEqual(["packing", "done"]);
	});

	test("resolves to the unlisten handle the dialogs store for cleanup", async () => {
		const channel = capture();
		await expect(listenArchiveProgress("op-1", () => {})).resolves.toBe(
			channel.unlisten,
		);
	});
});

describe("cancelArchiveOperation", () => {
	test("forwards the operation id under the camelCase argument name", async () => {
		mocks.invoke.mockResolvedValue(true);
		await expect(cancelArchiveOperation("op-1")).resolves.toBe(true);
		expect(mocks.invoke).toHaveBeenCalledWith("cancel_archive_operation", {
			operationId: "op-1",
		});
	});
});
