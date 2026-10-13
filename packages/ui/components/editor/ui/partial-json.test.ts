import { describe, expect, test } from "bun:test";
import { parsePartialJson } from "./partial-json";

const progressive = (source: string) =>
	parsePartialJson(source, { allowPartialArrayItems: true });

/** Every displayed scalar must already equal the corresponding final value. */
function expectSubset(actual: unknown, final: unknown): void {
	if (Array.isArray(actual)) {
		expect(Array.isArray(final)).toBe(true);
		expect(actual.length).toBeLessThanOrEqual((final as unknown[]).length);
		actual.forEach((item, index) =>
			expectSubset(item, (final as unknown[])[index]),
		);
	} else if (actual && typeof actual === "object") {
		for (const [key, value] of Object.entries(actual)) {
			expect(Object.hasOwn(final as object, key)).toBe(true);
			expectSubset(value, (final as Record<string, unknown>)[key]);
		}
	} else expect(actual).toEqual(final);
}

describe("partial JSON keeps completed values", () => {
	test("keeps completed cards and drops the next incomplete card by default", () => {
		expect(
			parsePartialJson(
				'{"title":"Views","items":[{"src":"/a.jpg","alt":"A"},{"src":"/b',
			),
		).toEqual({ title: "Views", items: [{ src: "/a.jpg", alt: "A" }] });
	});

	test("optionally exposes completed fields from an unfinished card", () => {
		expect(progressive('{"items":[{"title":"A","description":"half')).toEqual({
			items: [{ title: "A" }],
		});
		expect(
			progressive('{"data":[{"type":"bar","x":["A","B"],"y":[40,420'),
		).toEqual({ data: [{ type: "bar", x: ["A", "B"], y: [40] }] });
	});

	test("does not guess unfinished numbers, strings, or literals", () => {
		for (const token of ["4", "40", "-", "-0", "0.", "0.25", "4e", "4e+20"])
			expect(progressive(`{"values":[${token}`)).toEqual({ values: [] });
		for (const token of ['"half', "t", "tru", "true", "false", "null"])
			expect(progressive(`{"value":${token}`)).toEqual({});
		expect(progressive('{"values":[40,')).toEqual({ values: [40] });
		expect(progressive('{"value":4e+20 ')).toEqual({ value: 4e20 });
		expect(progressive('{"value":true,')).toEqual({ value: true });
	});

	test("closing quotes establish a complete string even at the stream tail", () => {
		expect(progressive('{"title":"A \\"quoted\\" value"')).toEqual({
			title: 'A "quoted" value',
		});
	});

	const samples = [
		{
			title: 'Quotes " brackets ] } commas , and a backslash \\',
			items: [
				{ src: "/image.jpg", alt: "<script>alert(1)</script> ```" },
				{ src: "/second.jpg", alt: "New\nline\ttab\rcarriage" },
			],
		},
		{
			max: 400,
			items: [
				{ label: "A", value: 40 },
				{ label: "B", value: 4.025 },
			],
			values: [true, false, null, -420, 4e20, -0.025],
		},
		[{ empty: {}, list: [], nested: [[40, 420], [{ enabled: true }]] }],
	];
	for (const [index, value] of samples.entries()) {
		test(`every prefix of nested sample ${index + 1} preserves final scalar values`, () => {
			const source = JSON.stringify(value);
			for (let cut = 0; cut <= source.length; cut++) {
				for (const allowPartialArrayItems of [false, true]) {
					const parsed = parsePartialJson(source.slice(0, cut), {
						allowPartialArrayItems,
					});
					if (parsed !== undefined) expectSubset(parsed, value);
				}
			}
			expect(progressive(source)).toEqual(value);
			expect(parsePartialJson(source)).toEqual(value);
		});
	}

	test("unicode escape prefixes stay incomplete until the string closes", () => {
		const source = '{"title":"\\u005b \\uD83D\\uDE00 \\u0000"}';
		for (let cut = 0; cut <= source.length; cut++) {
			const parsed = progressive(source.slice(0, cut));
			if (parsed !== undefined) expectSubset(parsed, JSON.parse(source));
		}
		expect(progressive(source)).toEqual(JSON.parse(source));
	});
});

describe("partial JSON refuses invalid syntax", () => {
	for (const source of [
		'{"items":[{"title":"Good"},]}',
		'{"title":"Good",}',
		'{"title":"Good" "extra":1}',
		'{"items":[1,,2]}',
		'{"title":"bad\\q',
		'{"title":"bad\\u12x',
		'{"title":"raw\nnewline"}',
		'{"value":01',
		'{"value":1.e',
		'{"value":1e,',
		'{"value":truex',
		'{"value":undefined}',
		'{"value":NaN}',
		'{"value":Infinity}',
		'{"value":+1}',
		'{"value":1/* comment */}',
		'{"value":1]}',
		'{}{"another":true}',
		'```json\n{"value":1}',
	]) {
		test(source, () => {
			expect(parsePartialJson(source)).toBeUndefined();
			expect(progressive(source)).toBeUndefined();
		});
	}

	test("prototype-shaped keys remain own data properties", () => {
		const source = '{"__proto__":{"polluted":true},"constructor":"text"}';
		const parsed = progressive(source) as Record<string, unknown>;
		expect(parsed).toEqual(JSON.parse(source));
		expect(Object.getPrototypeOf(parsed)).toBe(Object.prototype);
		expect(Object.hasOwn(parsed, "__proto__")).toBe(true);
		expect(({} as Record<string, unknown>).polluted).toBeUndefined();
	});

	test("bounds recursion for malformed or oversized nesting", () => {
		expect(progressive("[".repeat(200))).toBeUndefined();
		expect(
			progressive(`${"[".repeat(200)}0${"]".repeat(200)}`),
		).toBeUndefined();
	});
});
