import { describe, expect, test } from "bun:test";
import { QueryClient } from "@tanstack/react-query";
import type { IBoard } from "../schema/flow/board";
import { markBoardSnapshot } from "./identity";
import { setSnapshotQueryData, shareBoardSnapshot } from "./query";

describe("board snapshot query sharing", () => {
	test("ordinary data retains deep structural sharing", () => {
		const first = { items: [{ id: "one" }] };
		expect(shareBoardSnapshot(first, { items: [{ id: "one" }] })).toBe(first);
	});

	test("incremental snapshots bypass graph enumeration and keep their exact identity", () => {
		const nodes = new Proxy(
			{},
			{
				ownKeys() {
					throw new Error("graph should not be traversed");
				},
			},
		);
		const first = markBoardSnapshot({ nodes } as IBoard);
		const second = markBoardSnapshot({ nodes, name: "Edited" } as IBoard);
		expect(shareBoardSnapshot(first, second)).toBe(second);
		const client = new QueryClient({
			defaultOptions: { queries: { gcTime: Infinity } },
		});
		const key = ["getBoard", "app", "board"];
		try {
			setSnapshotQueryData(client, key, first);
			expect(client.getQueryData<IBoard>(key)).toBe(first);
			setSnapshotQueryData(client, key, second);
			expect(client.getQueryData<IBoard>(key)).toBe(second);
		} finally {
			client.clear();
		}
	});
});
