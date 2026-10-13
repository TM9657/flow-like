import assert from "node:assert/strict";
import test from "node:test";
import { CHANNEL, MAX_DOCUMENT_BYTES, validateCommand } from "./protocol.mjs";

const session = "document-session";
const message = (type, values = {}) => ({
	channel: CHANNEL,
	session,
	type,
	...values,
});

test("ignores messages for another channel or document session", () => {
	assert.equal(
		validateCommand({ ...message("export"), channel: "other" }, session),
		null,
	);
	assert.equal(
		validateCommand(
			message("export", { session: "old", requestId: "1" }),
			session,
		),
		null,
	);
	assert.equal(validateCommand(null, session), null);
});

test("opens DOCX bytes read-only unless editing is explicit", () => {
	const bytes = new ArrayBuffer(4);
	assert.deepEqual(
		validateCommand(message("open", { name: "REPORT.DOCX", bytes }), session),
		{ type: "open", name: "REPORT.DOCX", bytes, readOnly: true },
	);
	assert.equal(
		validateCommand(
			message("open", { name: "report.docx", bytes, readOnly: false }),
			session,
		).readOnly,
		false,
	);
	assert.throws(
		() =>
			validateCommand(message("open", { name: "report.doc", bytes }), session),
		/Only DOCX/,
	);
	assert.throws(
		() =>
			validateCommand(
				message("open", { name: "report.docx", bytes: new ArrayBuffer(0) }),
				session,
			),
		/50 MiB/,
	);
	assert.throws(
		() =>
			validateCommand(
				message("open", {
					name: "report.docx",
					bytes: new ArrayBuffer(MAX_DOCUMENT_BYTES + 1),
				}),
				session,
			),
		/50 MiB/,
	);
});

test("exports are correlated and save acknowledgements require valid revisions", () => {
	assert.deepEqual(
		validateCommand(
			message("export", { requestId: "save-1", format: "txt" }),
			session,
		),
		{ type: "export", requestId: "save-1" },
	);
	assert.equal(validateCommand(message("export"), session), null);
	assert.deepEqual(
		validateCommand(message("saved", { revision: 5 }), session),
		{ type: "saved", revision: 5 },
	);
	for (const revision of [-1, 2.5, Number.NaN, Number.POSITIVE_INFINITY, "5"])
		assert.equal(
			validateCommand(message("saved", { revision }), session),
			null,
		);
});
