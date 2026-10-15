import { expect, test } from "bun:test";
import { isScriptNavigationUrl } from "./navigation-url";

test("rejects script and data schemes after browser URL normalization", () => {
	for (const value of [
		"javascript:alert(1)",
		" JaVaScRiPt:alert(1)",
		"\u0000javascript:alert(1)",
		"java\nscript:alert(1)",
		"java\tscript:alert(1)",
		"vbscript:msgbox(1)",
		"\rVB\tScript:msgbox(1)",
		"data:text/html,<script>alert(1)</script>",
		" \u0000DaTa:text/html;base64,PHNjcmlwdD4=",
		"da\nta:image/svg+xml,<svg onload='alert(1)'/>",
		"data:image/png;base64,AAAA",
	]) {
		expect(isScriptNavigationUrl(value)).toBe(true);
	}
});

test("preserves app routes, ordinary links and custom protocol handlers", () => {
	for (const value of [
		"/use?id=abc",
		"../settings?label=data:text/html",
		"//example.com/path",
		"#data:text/html",
		"https://example.com/javascript:docs",
		"http://localhost:8080/",
		"flow-like://trigger/app/route",
		"mailto:help@example.com",
		"tel:+491234",
		"asset://localhost/image.png",
		"blob:https://example.com/id",
		"https://example.com/?url=data:text/html",
	]) {
		expect(isScriptNavigationUrl(value)).toBe(false);
	}
});
