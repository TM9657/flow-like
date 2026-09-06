import { describe, expect, test } from "vitest";

import { COMPONENT_PROPS } from "./component-prop-manifest";
import {
	getRegisteredTypes,
	registerComponentType,
} from "./component-type-registry";
import { validateComponents } from "../flowpilot/validateComponents";
import type { SurfaceComponent } from "./types";

describe("pure A2UI component type registry", () => {
	test("starts with every compile-time component manifest type", () => {
		const registered = getRegisteredTypes().sort();
		expect(registered).toEqual(Object.keys(COMPONENT_PROPS).sort());
	});

	test("makes a dynamically registered type immediately available to validation", () => {
		const type = "testDynamicComponent";
		registerComponentType(type);
		registerComponentType(type);

		expect(
			getRegisteredTypes().filter((candidate) => candidate === type),
		).toHaveLength(1);
		const result = validateComponents([
			{
				id: "dynamic",
				component: { type },
			},
		] as unknown as SurfaceComponent[]);
		expect(result.components).toHaveLength(1);
		expect(result.components[0]?.component.type).toBe(type);
	});
});
