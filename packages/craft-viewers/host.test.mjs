import assert from "node:assert/strict";
import test from "node:test";
import { CHANNEL, validateCommand } from "./protocol.mjs";
const session = "session";
const manifest = { extensions: ["xlsx", "xls"], maxBytes: 4 };
const command = (type, fields = {}) => ({
	channel: CHANNEL,
	session,
	type,
	...fields,
});
test("creates a named document without import bytes and distinguishes snapshot downloads", () => {
	assert.deepEqual(
		validateCommand(
			command("create", { name: "Untitled.xlsx", readOnly: false }),
			session,
			manifest,
		),
		{
			type: "create",
			name: "Untitled.xlsx",
			readOnly: false,
		},
	);
	assert.equal(
		validateCommand(
			command("create", { name: "Untitled.xlsx" }),
			session,
			manifest,
		).readOnly,
		true,
	);
	assert.throws(
		() =>
			validateCommand(
				command("create", { name: "Untitled.exe" }),
				session,
				manifest,
			),
		/not supported/,
	);
	assert.deepEqual(
		validateCommand(
			command("export", {
				requestId: "copy",
				format: "xlsx",
				purpose: "download",
			}),
			session,
			manifest,
		),
		{
			type: "export",
			requestId: "copy",
			format: "xlsx",
			purpose: "download",
		},
	);
});
test("validates session, imported extension, binary payload and per-editor size", () => {
	assert.equal(
		validateCommand({ ...command("open"), session: "old" }, session, manifest),
		null,
	);
	const bytes = new ArrayBuffer(4);
	assert.deepEqual(
		validateCommand(
			command("open", { name: "TABLE.XLS", bytes }),
			session,
			manifest,
		),
		{ type: "open", name: "TABLE.XLS", bytes, readOnly: true },
	);
	assert.throws(
		() =>
			validateCommand(
				command("open", { name: "a.xlsx", bytes: new ArrayBuffer(5) }),
				session,
				manifest,
			),
		/MiB/,
	);
	assert.throws(
		() =>
			validateCommand(
				command("open", { name: "a.xlsm", bytes }),
				session,
				manifest,
			),
		/not supported/,
	);
});
test("requires explicit export format and correlates save acknowledgements", () => {
	assert.equal(
		validateCommand(command("export", { requestId: "1" }), session, manifest),
		null,
	);
	assert.equal(
		validateCommand(
			command("export", { requestId: "1", format: "../xlsx" }),
			session,
			manifest,
		),
		null,
	);
	assert.deepEqual(
		validateCommand(
			command("export", { requestId: "1", format: "xlsx" }),
			session,
			manifest,
		),
		{ type: "export", requestId: "1", format: "xlsx" },
	);
	assert.equal(
		validateCommand(command("saved", { revision: -1 }), session, manifest),
		null,
	);
	assert.deepEqual(
		validateCommand(command("saved", { revision: 2 }), session, manifest),
		{ type: "saved", revision: 2 },
	);
});
