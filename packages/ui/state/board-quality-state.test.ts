import { describe, expect, test } from "bun:test";
import type { IQualityMark } from "../lib/board-quality";
import { emptyQualityReport } from "../lib/board-quality";
import { mergeMarks, useBoardQualityStore } from "./board-quality-state";

function mark(signature: string): IQualityMark {
	return {
		severity: "warning",
		counts: { error: 0, warning: 1, info: 0 },
		findings: [],
		nested: 0,
		signature,
	};
}

describe("mergeMarks", () => {
	test("keeps the previous object for every unchanged signature", () => {
		const previous = { a: mark("a1"), b: mark("b1") };
		const next = { a: mark("a1"), b: mark("b2"), c: mark("c1") };
		const merged = mergeMarks(previous, next);
		expect(merged.a).toBe(previous.a);
		expect(merged.b).toBe(next.b);
		expect(merged.c).toBe(next.c);
	});

	test("returns the previous map itself when nothing moved", () => {
		const previous = { a: mark("a1") };
		expect(mergeMarks(previous, { a: mark("a1") })).toBe(previous);
	});

	test("a removed mark produces a new map without it", () => {
		const previous = { a: mark("a1"), b: mark("b1") };
		const merged = mergeMarks(previous, { a: mark("a1") });
		expect(merged).not.toBe(previous);
		expect(Object.keys(merged)).toEqual(["a"]);
	});
});

describe("useBoardQualityStore", () => {
	test("a node's selector is referentially stable across reports that do not touch it", () => {
		const store = useBoardQualityStore.getState();
		store.setReport("board", {
			...emptyQualityReport(),
			marks: { n1: mark("s1"), n2: mark("s2") },
		});
		const first = useBoardQualityStore.getState().marks.board?.n1;
		store.setReport("board", {
			...emptyQualityReport(),
			marks: { n1: mark("s1"), n2: mark("s3") },
		});
		const state = useBoardQualityStore.getState();
		expect(state.marks.board?.n1).toBe(first as IQualityMark);
		expect(state.marks.board?.n2?.signature).toBe("s3");
		store.clear("board");
		expect(useBoardQualityStore.getState().marks.board).toBeUndefined();
	});
});
