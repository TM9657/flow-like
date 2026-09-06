import { expect, test } from "bun:test";
import {
	BOARD_FORMAT_HEADER,
	CURRENT_BOARD_FORMAT_VERSION,
} from "@flow-like/flow-like-ui/lib/board-format";
import type { AuthContextProps } from "react-oidc-context";
import { apiFetch } from "./api-utils";

test("board requests advertise the client format alongside authentication", async () => {
	const originalFetch = globalThis.fetch;
	let sentHeaders: Headers | undefined;
	globalThis.fetch = (async (_url, options) => {
		sentHeaders = new Headers(options?.headers);
		return Response.json({ board_format_version: 2 });
	}) as typeof fetch;
	try {
		const result = await apiFetch(
			"apps/example/board/capabilities",
			{ headers: { "x-request-id": "format-negotiation" } },
			{ user: { access_token: "client-token" } } as AuthContextProps,
		);
		expect(sentHeaders?.get(BOARD_FORMAT_HEADER)).toBe(
			String(CURRENT_BOARD_FORMAT_VERSION),
		);
		expect(sentHeaders?.get("Authorization")).toBe("Bearer client-token");
		expect(sentHeaders?.get("x-request-id")).toBe("format-negotiation");
		expect(sentHeaders?.has("x-flow-like-capabilities")).toBe(false);
		expect(result).toEqual({ board_format_version: 2 });
	} finally {
		globalThis.fetch = originalFetch;
	}
});
