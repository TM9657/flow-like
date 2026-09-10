import { describe, expect, test } from "bun:test";
import { type HistoryEntry, decodeEntry } from "./flow-history";
import {
	BoardHistory,
	BoardHistoryRegistry,
	type HistoryPersistence,
	type HistorySnapshot,
	type HistoryWrite,
	remoteBoardAppliedHandler,
} from "./flow-history-store";
import type { IGenericCommand } from "./schema";

const command = (id: string): IGenericCommand =>
	({ command_type: "MoveNode", node_id: id }) as unknown as IGenericCommand;

/** In-memory persistence that applies writes exactly like the Dexie adapter would. */
class FakePersistence implements HistoryPersistence {
	rows = new Map<string, Map<number, HistoryEntry>>();
	cursors = new Map<string, { appliedSeq: number; nextSeq: number }>();
	deliveries = new Set<string>();
	writes: HistoryWrite[] = [];
	failWrites = false;
	loadDelay: Promise<void> = Promise.resolve();

	async load(board: string): Promise<HistorySnapshot | undefined> {
		await this.loadDelay;
		const rows = this.rows.get(board);
		const cursor = this.cursors.get(board);
		if (!rows && !cursor) return undefined;
		return {
			rows: [...(rows?.values() ?? [])],
			appliedSeq: cursor?.appliedSeq ?? 0,
			nextSeq: cursor?.nextSeq ?? 1,
		};
	}

	async write(board: string, change: HistoryWrite): Promise<void> {
		this.writes.push(change);
		if (this.failWrites) throw new Error("disk full");
		const rows = this.rows.get(board) ?? new Map<number, HistoryEntry>();
		if (change.clear) rows.clear();
		for (const seq of change.remove ?? []) rows.delete(seq);
		for (const entry of change.put ?? []) rows.set(entry.seq, entry);
		this.rows.set(board, rows);
		this.cursors.set(board, {
			appliedSeq: change.appliedSeq,
			nextSeq: change.nextSeq,
		});
		if (change.delivery) this.deliveries.add(`${board}:${change.delivery}`);
	}

	async hasDelivery(board: string, deliveryId: string): Promise<boolean> {
		return this.deliveries.has(`${board}:${deliveryId}`);
	}

	seqs(board: string): number[] {
		return [...(this.rows.get(board)?.keys() ?? [])].sort((a, b) => a - b);
	}
}

const batchIds = (entry: HistoryEntry | undefined) =>
	entry
		? decodeEntry(entry).map(
				(item) => (item as unknown as { node_id: string }).node_id,
			)
		: undefined;

const must = <T>(value: T | undefined): T => {
	if (value === undefined) throw new Error("expected a value");
	return value;
};

describe("BoardHistory", () => {
	test("record / take / confirm keep memory and persistence in step", async () => {
		const persistence = new FakePersistence();
		const history = new BoardHistory("k", persistence);
		await history.record([command("a")]);
		await history.record([command("b")]);
		expect(history.getSummary()).toMatchObject({ undoDepth: 2, redoDepth: 0 });

		const entry = await history.take("undo");
		expect(batchIds(entry)).toEqual(["b"]);
		expect(history.getSummary()).toMatchObject({ undoDepth: 1, redoDepth: 1 });
		history.confirm();
		await history.flush();

		expect(persistence.seqs("k")).toEqual([1, 2]);
		expect(persistence.cursors.get("k")).toEqual({ appliedSeq: 1, nextSeq: 3 });

		const redone = await history.take("redo");
		expect(batchIds(redone)).toEqual(["b"]);
		history.confirm();
		await history.flush();
		expect(persistence.cursors.get("k")).toEqual({ appliedSeq: 2, nextSeq: 3 });
	});

	test("a failed replay restores the cursor; a repeated failure drops the entry", async () => {
		const persistence = new FakePersistence();
		const history = new BoardHistory("k", persistence);
		await history.record([command("a")]);
		await history.record([command("b")]);

		const first = must(await history.take("undo"));
		expect(history.fail("undo", first)).toBe(false);
		expect(history.getSummary()).toMatchObject({ undoDepth: 2, redoDepth: 0 });

		const second = must(await history.take("undo"));
		expect(second.seq).toBe(first.seq);
		expect(history.fail("undo", second)).toBe(true);
		expect(history.getSummary()).toMatchObject({ undoDepth: 1, redoDepth: 0 });
		await history.flush();
		expect(persistence.seqs("k")).toEqual([1]);
		expect(persistence.cursors.get("k")?.appliedSeq).toBe(1);

		// The next undo moves on to the surviving entry.
		expect(batchIds(await history.take("undo"))).toEqual(["a"]);
	});

	test("a success between two failures resets the strike", async () => {
		const history = new BoardHistory("k", undefined);
		await history.record([command("a")]);
		const entry = must(await history.take("undo"));
		history.fail("undo", entry);
		const again = must(await history.take("undo"));
		history.confirm();
		await history.take("redo");
		history.confirm();
		const third = must(await history.take("undo"));
		expect(third.seq).toBe(again.seq);
		expect(history.fail("undo", third)).toBe(false);
	});

	test("recording after an undo discards the redo tail in memory and on disk", async () => {
		const persistence = new FakePersistence();
		const history = new BoardHistory("k", persistence);
		await history.record([command("a")]);
		await history.record([command("b")]);
		await history.take("undo");
		history.confirm();
		await history.record([command("c")]);
		await history.flush();
		expect(persistence.seqs("k")).toEqual([1, 3]);
		expect(history.getSummary()).toMatchObject({ undoDepth: 2, redoDepth: 0 });
	});

	test("hydrates from persistence before the first operation, in call order", async () => {
		const persistence = new FakePersistence();
		let release: () => void = () => {};
		persistence.loadDelay = new Promise((resolve) => {
			release = resolve;
		});
		persistence.rows.set(
			"k",
			new Map([
				[
					1,
					{
						seq: 1,
						at: 0,
						payload: JSON.stringify([command("old")]),
						bytes: 1,
					},
				],
			]),
		);
		persistence.cursors.set("k", { appliedSeq: 1, nextSeq: 2 });

		const history = new BoardHistory("k", persistence);
		const recorded = history.record([command("new")]);
		const taken = history.take("undo");
		release();
		await recorded;
		expect(batchIds(await taken)).toEqual(["new"]);
		expect(history.getSummary()).toMatchObject({ undoDepth: 1, redoDepth: 1 });
	});

	test("an entry recorded in a previous session is undoable after rehydration", async () => {
		const persistence = new FakePersistence();
		const first = new BoardHistory("k", persistence);
		await first.record([command("a")]);
		await first.record([command("b")]);
		await first.take("undo");
		first.confirm();
		await first.flush();

		const second = new BoardHistory("k", persistence);
		await second.ready;
		expect(second.getSummary()).toMatchObject({ undoDepth: 1, redoDepth: 1 });
		expect(batchIds(await second.take("redo"))).toEqual(["b"]);
	});

	test("persistence failures never break the in-memory history", async () => {
		const persistence = new FakePersistence();
		persistence.failWrites = true;
		const history = new BoardHistory("k", persistence);
		await history.record([command("a")]);
		await history.flush();
		expect(history.getSummary().canUndo).toBe(true);
		expect(batchIds(await history.take("undo"))).toEqual(["a"]);
	});

	test("recordOnce records a delivery a single time across instances", async () => {
		const persistence = new FakePersistence();
		const history = new BoardHistory("k", persistence);
		expect(await history.recordOnce([command("a")], "d1")).toBe(true);
		expect(await history.recordOnce([command("a")], "d1")).toBe(false);
		await history.flush();

		const later = new BoardHistory("k", persistence);
		expect(await later.recordOnce([command("a")], "d1")).toBe(false);
		expect(later.getSummary().undoDepth).toBe(1);
	});

	test("recordOnce in invalidate mode empties the history but keeps the marker", async () => {
		const persistence = new FakePersistence();
		const history = new BoardHistory("k", persistence);
		await history.record([command("a")]);
		expect(await history.recordOnce([command("r")], "d2", "invalidate")).toBe(
			true,
		);
		await history.flush();
		expect(history.getSummary().canUndo).toBe(false);
		expect(persistence.seqs("k")).toEqual([]);
		expect(await persistence.hasDelivery("k", "d2")).toBe(true);
	});

	test("exclusive serializes operations so an undo waits for the edit in flight", async () => {
		const history = new BoardHistory("k", undefined);
		const order: string[] = [];
		let finishEdit: () => void = () => {};
		const backendCommit = new Promise<void>((resolve) => {
			finishEdit = resolve;
		});
		const edit = history.exclusive(async () => {
			await backendCommit;
			await history.record([command("edit")]);
			order.push("edit");
		});
		const undo = history.exclusive(async () => {
			const entry = await history.take("undo");
			order.push(`undo:${batchIds(entry)?.[0]}`);
		});
		finishEdit();
		await Promise.all([edit, undo]);
		expect(order).toEqual(["edit", "undo:edit"]);
	});

	test("exclusive keeps running after a failed operation", async () => {
		const history = new BoardHistory("k", undefined);
		await expect(
			history.exclusive(async () => {
				throw new Error("boom");
			}),
		).rejects.toThrow("boom");
		expect(await history.exclusive(async () => "next")).toBe("next");
	});

	test("subscribers are told when the summary changes and only then", async () => {
		const history = new BoardHistory("k", undefined);
		let notifications = 0;
		history.subscribe(() => {
			notifications++;
		});
		await history.record([command("a")]);
		expect(notifications).toBe(1);
		await history.record([]);
		expect(notifications).toBe(1);
		await history.take("undo");
		expect(notifications).toBe(2);
	});
});

describe("BoardHistoryRegistry", () => {
	test("hands out one history per board", () => {
		const registry = new BoardHistoryRegistry(undefined);
		expect(registry.get("a", "b")).toBe(registry.get("a", "b"));
		expect(registry.get("a", "b")).not.toBe(registry.get("a", "c"));
	});

	test("clear reaches persistence even when the board is not resident", async () => {
		const persistence = new FakePersistence();
		const registry = new BoardHistoryRegistry(persistence);
		await persistence.write("a_b", {
			put: [{ seq: 1, at: 0, payload: "[]", bytes: 2 }],
			appliedSeq: 1,
			nextSeq: 2,
		});
		await registry.clear("a", "b");
		expect(persistence.seqs("a_b")).toEqual([]);
	});

	test("clear empties a resident history", async () => {
		const registry = new BoardHistoryRegistry(undefined);
		const history = registry.get("a", "b");
		await history.record([command("x")]);
		await registry.clear("a", "b");
		expect(history.getSummary().canUndo).toBe(false);
	});
});

describe("remote board applied handler", () => {
	const dispatch = (detail: unknown) =>
		new CustomEvent("flow:remote-board-applied", { detail });

	test("clears only on a reset, never on a plain sync (defect: online undo worked once)", async () => {
		const cleared: string[] = [];
		const handler = remoteBoardAppliedHandler({
			clear: async (appId, boardId) => {
				cleared.push(`${appId}/${boardId}`);
			},
		});
		handler(dispatch({ appId: "a", boardId: "b", reason: "sync" }));
		handler(dispatch({ appId: "a", boardId: "b" }));
		expect(cleared).toEqual([]);
		handler(dispatch({ appId: "a", boardId: "b", reason: "reset" }));
		await Promise.resolve();
		expect(cleared).toEqual(["a/b"]);
	});

	test("swallows clear failures", async () => {
		const handler = remoteBoardAppliedHandler({
			clear: async () => {
				throw new Error("nope");
			},
		});
		expect(() =>
			handler(dispatch({ appId: "a", boardId: "b", reason: "reset" })),
		).not.toThrow();
		await Promise.resolve();
	});
});

describe("transient failures", () => {
	test("never count against an entry, so a network blip cannot drop it", async () => {
		const history = new BoardHistory("k", undefined);
		await history.record([command("a")]);
		for (let attempt = 0; attempt < 5; attempt++) {
			const entry = must(await history.take("undo"));
			expect(history.fail("undo", entry, true)).toBe(false);
		}
		expect(history.getSummary()).toMatchObject({ undoDepth: 1, redoDepth: 0 });
	});

	test("a transient failure between two refusals keeps the strike", async () => {
		const history = new BoardHistory("k", undefined);
		await history.record([command("a")]);
		history.fail("undo", must(await history.take("undo")));
		history.fail("undo", must(await history.take("undo")), true);
		expect(history.fail("undo", must(await history.take("undo")))).toBe(true);
		expect(history.getSummary().canUndo).toBe(false);
	});
});
