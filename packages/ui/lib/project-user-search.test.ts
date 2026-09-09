import { describe, expect, test } from "bun:test";
import type { IUserLookup } from "../state/backend-state/types";
import {
	createProjectUserSearch,
	mergeProjectUserResults,
} from "./project-user-search";

function account(id: string, fields: Partial<IUserLookup> = {}): IUserLookup {
	return { id, created_at: "", ...fields };
}

describe("createProjectUserSearch", () => {
	test("accepts nullable profile fields returned by the directory", () => {
		const user = {
			id: "server-user",
			name: null,
			email: null,
			username: null,
			preferred_username: null,
			avatar_url: null,
			created_at: "",
		} as unknown as IUserLookup;
		const search = createProjectUserSearch([user]);
		expect(search.search("")).toEqual([user]);
		expect(search.search("server-user")).toEqual([user]);
		expect(search.search("alice")).toEqual([]);
		expect(mergeProjectUserResults([user], [user], "server-user")).toEqual([
			{ user, fromProject: true },
		]);
	});

	test("matches accents, case, reversed names, and tokens across profile fields", () => {
		const user = account("1", {
			name: "José Groß",
			preferred_username: "jose.design",
			email: "jose@example.com",
		});
		const search = createProjectUserSearch([
			user,
			account("2", { name: "Joseph Green" }),
		]);
		for (const query of [
			"JOSE GROSS",
			"gross jose",
			"  gro   jos  ",
			"design gross",
			"gross example",
		]) {
			expect(search.search(query)).toEqual([user]);
		}
	});

	test("finds handles and exact email or account ID without unrelated names", () => {
		const user = account("account-00001", {
			name: "Alex Doe",
			username: "alex.doe",
			preferred_username: "alex",
			email: "alex@example.com",
		});
		const search = createProjectUserSearch([
			account("other", { name: "Alex" }),
			user,
		]);
		for (const query of [
			"@ALEX",
			"@alex.d",
			"ALEX@EXAMPLE.COM",
			"account-00001",
		]) {
			expect(search.search(query)).toEqual([user]);
		}
		expect(search.search("alex@different.com")).toEqual([]);
		expect(search.search("@")).toEqual([]);
	});

	test("tolerates one typo per word but requires every query word to match", () => {
		const alice = account("1", { name: "Alice Smith" });
		const search = createProjectUserSearch([
			alice,
			account("2", { name: "Alina Jones" }),
		]);
		for (const query of ["alcie", "alce", "alicce", "alixe", "smiht alcie"]) {
			expect(search.search(query)).toEqual([alice]);
		}
		expect(search.search("alice jones")).toEqual([]);
		expect(search.search("axxce")).toEqual([]);
		expect(search.search("xli")).toEqual([]);
	});

	test("ranks exact matches above prefixes, substrings, and typos", () => {
		const search = createProjectUserSearch([
			account("typo", { name: "Alixe" }),
			account("substring", { name: "Malice" }),
			account("prefix", { name: "Alicea" }),
			account("name", { name: "Alice" }),
			account("handle", { preferred_username: "alice" }),
		]);
		expect(search.search("alice").map((user) => user.id)).toEqual([
			"handle",
			"name",
			"prefix",
			"substring",
			"typo",
		]);
	});

	test("deduplicates colleagues and bounds stable suggestions and matches", () => {
		const users = Array.from({ length: 100 }, (_, index) =>
			account(String(index), { name: "Alice Example" }),
		);
		const updated = account("0", { name: "Alice Updated" });
		const search = createProjectUserSearch([...users, updated]);
		expect(search.search(" ")).toEqual([updated, ...users.slice(1, 25)]);
		expect(search.search("alice")).toEqual([updated, ...users.slice(1, 25)]);
		expect(search.search("alice", 2)).toEqual([updated, users[1]]);
		expect(search.search("alice", 0)).toEqual([]);
		expect(search.search("alice", -1)).toEqual([]);
		expect(search.search("alice", 1.9)).toEqual([updated]);
	});

	test("searches 10,000 contacts repeatedly without re-reading profile fields", () => {
		let profileReads = 0;
		const users = Array.from({ length: 10_000 }, (_, index) => ({
			id: String(index),
			created_at: "",
			get name() {
				profileReads++;
				return `Colleague Person${index}`;
			},
		}));
		const search = createProjectUserSearch(users);
		profileReads = 0;
		for (const query of [
			"c",
			"col",
			"colleague",
			"pers",
			"colleague person9",
			"person9999 colleague",
		]) {
			const results = search.search(query);
			expect(results.length).toBeGreaterThan(0);
			expect(results.length).toBeLessThanOrEqual(25);
		}
		expect(search.search("person9999 colleague", 1)[0]?.id).toBe("9999");
		expect(profileReads).toBe(0);
	});
});

describe("mergeProjectUserResults", () => {
	test("boosts comparable colleague matches and excludes unrelated colleagues", () => {
		const colleague = account("known", { name: "Alice Smith" });
		const stranger = account("new", { name: "Alice Jones" });
		expect(
			mergeProjectUserResults(
				[colleague, account("unrelated", { name: "Bob Green" })],
				[stranger],
				"alice",
			),
		).toEqual([
			{ user: colleague, fromProject: true },
			{ user: stranger, fromProject: false },
		]);
	});

	test("exact remote identities outrank weaker colleague matches", () => {
		const cases = [
			{
				query: "@alex",
				local: { preferred_username: "alexander" },
				remote: { preferred_username: "alex" },
			},
			{
				query: "alex@example.com",
				local: { email: "alex@example.computer" },
				remote: { email: "alex@example.com" },
			},
			{
				query: "account-00001",
				local: { name: "Account 00001" },
				remote: { id: "account-00001" },
			},
		];
		for (const { query, local, remote } of cases) {
			const known = account("known", local);
			const fresh = account("fresh", remote);
			expect(mergeProjectUserResults([known], [fresh], query)).toEqual([
				{ user: fresh, fromProject: false },
				{ user: known, fromProject: true },
			]);
		}
	});

	test("deduplicates IDs, keeps colleague provenance, and uses the remote profile", () => {
		const stale = account("same", { name: "Alice Smith" });
		const current = account("same", {
			name: "Alice Taylor",
			avatar_url: "new-avatar.png",
		});
		expect(
			mergeProjectUserResults([stale, stale], [current, current], "alice"),
		).toEqual([{ user: current, fromProject: true }]);
	});

	test("preserves exact remote matches when the searched identity is private", () => {
		const colleague = account("known", { email: "alex@example.computer" });
		const exact = account("fresh", {
			name: "Alex",
			email: null as unknown as string,
			exact_match: true,
		});
		expect(
			mergeProjectUserResults([colleague], [exact], "alex@example.com"),
		).toEqual([
			{ user: exact, fromProject: false },
			{ user: colleague, fromProject: true },
		]);
	});

	test("keeps the exact match priority and current profile of a duplicate colleague", () => {
		const stale = account("known", { name: "Alex Old" });
		const current = account("known", {
			name: "Alex Current",
			exact_match: true,
		});
		const other = account("other", { email: "alex@example.com" });
		expect(
			mergeProjectUserResults([stale], [other, current], "alex@example.com"),
		).toEqual([
			{ user: current, fromProject: true },
			{ user: other, fromProject: false },
		]);
	});

	test("preserves server matches and caps the combined results", () => {
		const remote = Array.from({ length: 50 }, (_, index) =>
			account(String(index)),
		);
		expect(mergeProjectUserResults([], remote, "server-only-field")).toEqual(
			remote.slice(0, 25).map((user) => ({ user, fromProject: false })),
		);
		expect(mergeProjectUserResults([], remote, "query", 0)).toEqual([]);
		expect(mergeProjectUserResults([], remote, "query", 3)).toHaveLength(3);
	});
});
