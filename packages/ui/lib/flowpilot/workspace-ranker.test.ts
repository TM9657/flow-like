import { describe, expect, it } from "vitest";
import {
	createWorkspaceIndex,
	rankWorkspaceDocuments,
} from "./workspace-ranker";
import type { WorkspaceDocument } from "./workspace-resource";

function document(
	id: string,
	symbol: string,
	content: string,
): WorkspaceDocument {
	return {
		resource_id: id,
		target: { kind: "workflow", app_id: "app", board_id: "board", id: symbol },
		revision: "a".repeat(64),
		title: symbol,
		symbol,
		signature: `function ${symbol}()`,
		content,
	};
}

describe("workspace query matching", () => {
	it("finds related body evidence even when a question contains unmatched prose", () => {
		const result = rankWorkspaceDocuments(
			[
				document(
					"shipping",
					"dispatch",
					"orders are shipped using carrier labels",
				),
				document("support", "support", "support tickets have assigned owners"),
			],
			"How does it track shipped orders?",
		);
		expect(result.hits.map((hit) => hit.resource_id)).toEqual(["shipping"]);
		expect(result.hits[0].match_quality).toBe("partial");
		expect(result.hits[0].query_coverage).toBeLessThan(1);
	});

	it("normalizes inflections on both indexed text and the query", () => {
		const index = createWorkspaceIndex([
			document("audit", "audit", "store records and process updates"),
		]);
		expect(
			index.search("stored records processed updates").hits[0]?.resource_id,
		).toBe("audit");
	});

	it("keeps an exact qualified identifier ahead of matching prose", () => {
		const result = rankWorkspaceDocuments(
			[
				document("named", "warehouse::ship", "return receipt"),
				document("prose", "notes", "warehouse::ship ".repeat(100)),
			],
			"warehouse::ship",
		);
		expect(result.hits[0].resource_id).toBe("named");
		expect(result.hits[0].match_quality).toBe("identifier");
	});

	it("does not discard a known identifier merely because it is also a stop word", () => {
		const result = rankWorkspaceDocuments(
			[document("getter", "get", "return receipt")],
			"get",
		);
		expect(result.hits[0]?.resource_id).toBe("getter");
	});

	it("does not relax missing acronyms into unrelated generic matches", () => {
		const result = rankWorkspaceDocuments(
			[
				document(
					"shipping",
					"dispatch",
					"shipped orders receive carrier labels",
				),
			],
			"SAML shipped orders",
		);
		expect(result.hits).toEqual([]);
	});

	it("retains the no-useful-match result for one incidental word in a capability request", () => {
		const result = rankWorkspaceDocuments(
			[document("lease", "lease", "renew the database connection")],
			"Redis distributed lease renewal",
		);
		expect(result.hits).toEqual([]);
	});

	it("requires both terms for short queries instead of returning one weak overlap", () => {
		expect(
			rankWorkspaceDocuments(
				[document("lease", "lease", "database connection")],
				"database satellite",
			).hits,
		).toEqual([]);
	});

	it("retains partial candidates alongside strict candidates for a focused exact read", () => {
		const result = rankWorkspaceDocuments(
			[
				document(
					"orchestrator",
					"process",
					"invoice reconciliation executed shipments",
				),
				document("query", "joinRecords", "invoice reconciliation shipments"),
			],
			"invoice reconciliation executed shipments",
		);
		expect(result.hits.map((hit) => hit.resource_id)).toContain("query");
		expect(result.mode).toBe("mixed");
	});

	it("retains identifier typo matching without fuzzy expansion of every prose word", () => {
		const result = rankWorkspaceDocuments(
			[document("amount", "calculateAmount", "return amount")],
			"calcluateAmount",
		);
		expect(result.hits[0]?.resource_id).toBe("amount");
		expect(result.mode).toBe("fuzzy");
	});

	it("reuses an immutable index and produces the same evidence as one-shot search", () => {
		const documents = [
			document("audit", "audit", "stored records updated timestamps"),
		];
		const index = createWorkspaceIndex(documents);
		const expected = rankWorkspaceDocuments(documents, "records timestamps");
		expect(index.search("records timestamps")).toEqual(expected);
		index.search("unrelated missing capability");
		expect(index.search("records timestamps")).toEqual(expected);
	});
});
