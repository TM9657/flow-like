import { describe, expect, it } from "vitest";
import type { IBoardVariables } from "../../../lib/schema/flow/board-summary";
import {
	IValueType,
	type IVariable,
	IVariableType,
} from "../../../lib/schema/flow/variable";
import { convertJsonToUint8Array } from "../../../lib/uint8";
import type { StoredRuntimeVariable } from "../../../state/runtime-variables-context";
import { deriveSetup } from "./use-app-setup";

function variable(id: string, overrides: Partial<IVariable> = {}): IVariable {
	return {
		id,
		name: id,
		data_type: IVariableType.String,
		value_type: IValueType.Normal,
		exposed: false,
		editable: true,
		secret: false,
		runtime_configured: false,
		...overrides,
	};
}

function board(
	id: string,
	name: string,
	variables: IVariable[],
): IBoardVariables {
	return {
		board_id: id,
		board_name: name,
		variables: Object.fromEntries(variables.map((v) => [v.id, v])),
		refs: {},
	};
}

function stored(
	variableId: string,
	value: unknown,
	overrides: Partial<StoredRuntimeVariable> = {},
): StoredRuntimeVariable {
	return {
		variableId,
		variableName: variableId,
		boardId: "b1",
		value: convertJsonToUint8Array(value) ?? [],
		isSecret: false,
		updatedAt: "2026-09-11T10:00:00Z",
		...overrides,
	};
}

const none = new Set<string>();

describe("deriveSetup", () => {
	it("routes each variable to the lane its flags imply", () => {
		const boards = [
			board("b1", "Intake", [
				variable("device_only", { runtime_configured: true }),
				variable("a_secret", { secret: true }),
				variable("shared_only", { exposed: true, editable: true }),
				variable("plain", {}),
			]),
		];

		const { requirements, sharedGroups } = deriveSetup(boards, [], none);

		expect(requirements.map((r) => r.variable.id).sort()).toEqual([
			"a_secret",
			"device_only",
		]);
		expect(sharedGroups[0].variables.map((v) => v.id)).toEqual(["shared_only"]);
	});

	it("gives a variable that is both shared and personal one row in each lane", () => {
		const both = variable("threshold", {
			exposed: true,
			editable: true,
			runtime_configured: true,
		});
		const boards = [board("b1", "Triage", [both])];

		const { requirements, sharedGroups } = deriveSetup(
			boards,
			[stored("threshold", "0.6")],
			none,
		);

		expect(requirements).toHaveLength(1);
		expect(requirements[0].alsoShared).toBe(true);
		expect(sharedGroups[0].variables).toHaveLength(1);
		// The shared row has to be able to say that a device value shadows it.
		expect(sharedGroups[0].overriddenIds.has("threshold")).toBe(true);
	});

	it("surfaces a locked variable instead of hiding it from every screen", () => {
		const boards = [
			board("b1", "Intake", [
				variable("model_name", { exposed: true, editable: false }),
			]),
		];

		const { sharedGroups } = deriveSetup(boards, [], none);

		expect(sharedGroups[0].variables).toHaveLength(0);
		expect(sharedGroups[0].locked.map((v) => v.id)).toEqual(["model_name"]);
	});

	it("treats a blank string as unconfigured rather than as a stored row", () => {
		const boards = [
			board("b1", "Intake", [variable("token", { runtime_configured: true })]),
		];

		const { blocking, satisfiedCount } = deriveSetup(
			boards,
			[stored("token", "   ")],
			none,
		);

		expect(satisfiedCount).toBe(0);
		expect(blocking.map((r) => r.variable.id)).toEqual(["token"]);
	});

	it("counts a stored boolean false as configured", () => {
		const boards = [
			board("b1", "Intake", [
				variable("dry_run", {
					runtime_configured: true,
					data_type: IVariableType.Boolean,
				}),
			]),
		];

		const { blocking, satisfiedCount } = deriveSetup(
			boards,
			[stored("dry_run", false)],
			none,
		);

		expect(satisfiedCount).toBe(1);
		expect(blocking).toHaveLength(0);
	});

	it("excludes orphaned rows from the counts instead of inflating them", () => {
		const boards = [
			board("b1", "Intake", [variable("kept", { runtime_configured: true })]),
		];
		const rows = [
			stored("kept", "value"),
			stored("gone_a", "value"),
			stored("gone_b", "value"),
		];

		const { orphans, satisfiedCount, requiredCount, progress } = deriveSetup(
			boards,
			rows,
			none,
		);

		expect(orphans.map((o) => o.variableId).sort()).toEqual([
			"gone_a",
			"gone_b",
		]);
		expect(satisfiedCount).toBe(1);
		expect(requiredCount).toBe(1);
		expect(progress).toBe(100);
	});

	it("keeps progress within the track", () => {
		const boards = [
			board("b1", "Intake", [
				variable("one", { runtime_configured: true }),
				variable("two", { runtime_configured: true }),
			]),
		];

		const { progress } = deriveSetup(boards, [stored("one", "set")], none);

		expect(progress).toBe(50);
		expect(progress).toBeLessThanOrEqual(100);
	});

	it("drops a waived secret from the blocking set and the denominator", () => {
		const boards = [
			board("b1", "Intake", [
				variable("api_key", { secret: true }),
				variable("tenant", { runtime_configured: true }),
			]),
		];

		const waived = deriveSetup(
			boards,
			[stored("tenant", "acme")],
			new Set(["api_key"]),
		);

		expect(waived.blocking).toHaveLength(0);
		expect(waived.requiredCount).toBe(1);
		expect(waived.progress).toBe(100);

		const notWaived = deriveSetup(boards, [stored("tenant", "acme")], none);
		expect(notWaived.blocking.map((r) => r.variable.id)).toEqual(["api_key"]);
		expect(notWaived.requiredCount).toBe(2);
	});

	it("reports nothing rather than orphans when the board read produced no data", () => {
		const { orphans, requirements, sharedGroups, progress } = deriveSetup(
			undefined,
			[stored("token", "value")],
			none,
		);

		expect(orphans).toEqual([]);
		expect(requirements).toEqual([]);
		expect(sharedGroups).toEqual([]);
		expect(progress).toBe(100);
	});

	it("omits a flow that exposes nothing from the shared lane", () => {
		const boards = [
			board("b1", "Intake", [variable("token", { runtime_configured: true })]),
			board("b2", "Sync", [variable("url", { exposed: true, editable: true })]),
		];

		const { sharedGroups } = deriveSetup(boards, [], none);

		expect(sharedGroups.map((g) => g.boardName)).toEqual(["Sync"]);
	});
});
