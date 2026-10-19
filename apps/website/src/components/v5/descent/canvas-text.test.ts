import { describe, expect, test } from "bun:test";
import {
	breakRuns,
	ellipsize,
	fitLine,
	fitLines,
	wrapLines,
} from "./canvas-text";

// Monospace stand-in: Latin glyphs 0.6em, CJK 1em.
const width = (s: string, size = 10) => {
	let w = 0;
	for (const ch of s) w += (/[　-鿿＀-￯]/.test(ch) ? 1 : 0.6) * size;
	return w;
};
const m = (s: string) => width(s);

describe("breakRuns", () => {
	test("Latin breaks after spaces and hyphens", () => {
		expect(breakRuns("Prepare the follow-ups.")).toEqual([
			"Prepare ",
			"the ",
			"follow-",
			"ups.",
		]);
	});
	test("Japanese breaks between words, never before 、 or 。", () => {
		const runs = breakRuns(
			"Q3のサプライヤーレビューを準備し、フォローアップをスケジュールして。",
			"ja",
		);
		expect(runs.join("")).toBe(
			"Q3のサプライヤーレビューを準備し、フォローアップをスケジュールして。",
		);
		for (const r of runs) expect(/^[、。]/.test(r)).toBe(false);
		expect(runs).toContain("サプライヤー");
	});
	test("Chinese breaks between words", () => {
		const runs = breakRuns("准备 Q3 供应商评审并安排后续跟进。", "zh");
		expect(runs.length).toBeGreaterThan(4);
		expect(runs.at(-1)?.endsWith("。")).toBe(true);
	});
});

describe("wrapLines", () => {
	test("never exceeds the width", () => {
		const text = "Prepare the Q3 supplier review and schedule follow-ups.";
		const lines = wrapLines(m, text, 120);
		for (const l of lines) expect(m(l)).toBeLessThanOrEqual(120);
		expect(lines.join(" ")).toBe(text);
	});
	test("CJK wraps without losing characters", () => {
		const text =
			"Q3のサプライヤーレビューを準備し、フォローアップをスケジュールして。";
		const lines = wrapLines(m, text, 100, "ja");
		for (const l of lines) expect(m(l)).toBeLessThanOrEqual(100);
		expect(lines.join("")).toBe(text);
		for (const l of lines.slice(1)) expect(/^[、。]/.test(l)).toBe(false);
	});
	test("a word wider than the line is split", () => {
		const lines = wrapLines(m, "Unternehmensstammdatenanreicherung", 60);
		for (const l of lines) expect(m(l)).toBeLessThanOrEqual(60);
		expect(lines.join("")).toBe("Unternehmensstammdatenanreicherung");
	});
});

describe("fit", () => {
	test("fitLine shrinks before it ellipsizes", () => {
		const r = fitLine(width, "Finance Approvals", 95, { size: 10, floor: 8 });
		expect(r.text).toBe("Finance Approvals");
		expect(width(r.text, r.size)).toBeLessThanOrEqual(95);
		expect(r.size).toBeGreaterThanOrEqual(8);
	});
	test("fitLine ellipsizes at the floor", () => {
		const r = fitLine(width, "FlowPilot Workspace", 60, { size: 10, floor: 8 });
		expect(r.size).toBe(8);
		expect(r.text.endsWith("…")).toBe(true);
		expect(width(r.text, 8)).toBeLessThanOrEqual(60);
	});
	test("fitLines keeps to maxLines", () => {
		const r = fitLines(
			width,
			"Prepare the Q3 supplier review and schedule follow-ups.",
			100,
			{ size: 10, floor: 9, maxLines: 3 },
		);
		expect(r.lines.length).toBeLessThanOrEqual(3);
		for (const l of r.lines) expect(width(l, r.size)).toBeLessThanOrEqual(100);
	});
	test("ellipsize keeps graphemes whole", () => {
		const e = ellipsize(m, "👩‍💻👩‍💻👩‍💻👩‍💻👩‍💻 team", 20);
		expect(e.endsWith("…")).toBe(true);
	});
});
