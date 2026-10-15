import { expect, test } from "bun:test";
import { isScriptNavigationUrl } from "./navigation-url";

test("rejects script schemes after browser URL normalization", () => {
	for (const value of [
		"javascript:alert(1)",
		" JaVaScRiPt:alert(1)",
		"\u0000javascript:alert(1)",
		"java\nscript:alert(1)",
		"java\tscript:alert(1)",
		"vbscript:msgbox(1)",
	]) {
		expect(isScriptNavigationUrl(value)).toBe(true);
	}
});

test("preserves app routes, ordinary links and custom protocol handlers", () => {
	for (const value of [
		"/use?id=abc",
		"https://example.com/javascript:docs",
		"http://localhost:8080/",
		"flow-like://trigger/app/route",
		"mailto:help@example.com",
		"tel:+491234",
		"asset://localhost/image.png",
		"blob:https://example.com/id",
		"data:image/png;base64,AAAA",
	]) {
		expect(isScriptNavigationUrl(value)).toBe(false);
	}
});
