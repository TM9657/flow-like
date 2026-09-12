import { describe, expect, test } from "bun:test";
import type { IBit } from "../schema/bit/bit";
import { DEPRECATED_TAG, isDeprecated, listableModels } from "./model-listing";

function bit(overrides: Partial<IBit> & { id: string }): IBit {
	return {
		dependencies: [],
		meta: { en: { name: overrides.id, tags: [] } },
		...overrides,
	} as unknown as IBit;
}

function named(id: string, name: string, tags: string[] = []): IBit {
	return bit({ id, meta: { en: { name, tags } } as unknown as IBit["meta"] });
}

describe("isDeprecated", () => {
	test("a model with no tags is offered", () => {
		expect(isDeprecated(named("a", "Granite - Nano"))).toBe(false);
	});

	test("the retirement tag withdraws a model", () => {
		expect(isDeprecated(named("a", "Jina - DE / EN", [DEPRECATED_TAG]))).toBe(
			true,
		);
	});

	test("the tag is matched regardless of case or padding", () => {
		expect(isDeprecated(named("a", "Jina", ["  Deprecated "]))).toBe(true);
	});

	test("an unrelated tag does not withdraw a model", () => {
		expect(isDeprecated(named("a", "Jina", ["deprecation-policy"]))).toBe(
			false,
		);
	});

	test("a tag on a non-English metadata row still counts", () => {
		const withGerman = bit({
			id: "a",
			meta: {
				de: { name: "Jina", tags: [DEPRECATED_TAG] },
			} as unknown as IBit["meta"],
		});
		expect(isDeprecated(withGerman)).toBe(true);
	});

	test("a bit carrying no metadata at all is not treated as retired", () => {
		expect(isDeprecated(bit({ id: "a", meta: undefined }))).toBe(false);
	});
});

describe("listableModels", () => {
	test("a retired model is withheld while its peers stay", () => {
		const bits = [
			named("current", "Granite - Nano"),
			named("retired", "Jina v2 - Code", [DEPRECATED_TAG]),
		];
		expect(listableModels(bits).map((b) => b.id)).toEqual(["current"]);
	});

	test("retirement is independent of the dependency and naming rules", () => {
		const bits = [
			named("parent", "Clip - Vision"),
			bit({
				id: "parent",
				meta: {
					en: { name: "Clip - Vision", tags: [] },
				} as unknown as IBit["meta"],
				dependencies: ["api.flow-like.com:half"],
			}),
			named("half", "text half"),
			named("unnamed", ""),
			named("retired", "Jina - EN / ZH", [DEPRECATED_TAG]),
		];
		// The component half, the nameless bit and the retired model all drop out.
		expect(new Set(listableModels(bits).map((b) => b.id))).toEqual(
			new Set(["parent"]),
		);
	});

	test("a retired model is still present in the input, so it stays resolvable", () => {
		const bits = [named("retired", "Jina - EN / ES", [DEPRECATED_TAG])];
		expect(listableModels(bits)).toHaveLength(0);
		expect(bits.find((b) => b.id === "retired")).toBeDefined();
	});
});
