export interface CraftToolTarget {
	appId: string;
	surfaceId: string;
	componentId: string;
}

export interface CraftMcpContext {
	appId: string;
	signal?: AbortSignal;
	deadline?: number;
}

type JsonObject = Record<string, unknown>;
const MAX_REQUEST_BYTES = 1024 * 1024;
const MAX_RESPONSE_BYTES = 32 * 1024 * 1024;
const CANCELLATION_TTL_MS = 120_000;
const MAX_CANCELLATIONS = 512;
const MAX_CANCELLATION_BYTES = 1024 * 1024;
const sessions = new Map<string, CraftMcpSession>();
const sessionKey = (appId: string, elementRef: string) =>
	JSON.stringify([appId, elementRef]);
const isObject = (value: unknown): value is JsonObject =>
	!!value && typeof value === "object" && !Array.isArray(value);

class CraftMcpError extends Error {
	constructor(
		public code: string,
		message: string,
	) {
		super(message);
		this.name = "CraftMcpError";
	}
}

function jsonBytes(value: unknown): number {
	try {
		return new TextEncoder().encode(JSON.stringify(value)).byteLength;
	} catch {
		throw new CraftMcpError(
			"invalid_json",
			"Editor messages must contain valid JSON values.",
		);
	}
}

interface SessionOptions {
	target: CraftToolTarget;
	sessionId: string;
	editor: string;
	channel: string;
	origin: string;
	frame: () => Window | null;
	state: () => {
		ready: boolean;
		editable: boolean;
		editing: boolean;
		busy: boolean;
	};
	post: (message: JsonObject) => void;
}

interface PendingRequest {
	message: JsonObject;
	resolve: (value: unknown) => void;
	reject: (error: Error) => void;
	cleanup: () => void;
}

/** Owns RPC requests for one mounted document, including its unsaved edits. */
export class CraftMcpSession {
	private readonly pending = new Map<string, PendingRequest>();
	private readonly cancellations = new Map<
		string | number,
		{ expires: number; bytes: number }
	>();
	private cancellationBytes = 0;
	private cancellationOverflowUntil = 0;
	private readonly readOnlyTools = new Set<string>();
	private disposed = false;
	readonly elementRef: string;

	constructor(readonly options: SessionOptions) {
		this.elementRef = `${options.target.surfaceId}/${options.target.componentId}`;
		const key = sessionKey(options.target.appId, this.elementRef);
		sessions.get(key)?.dispose();
		sessions.set(key, this);
	}

	private assertAvailable(context: CraftMcpContext) {
		if (this.disposed || !this.options.state().ready || !this.options.frame())
			throw new CraftMcpError(
				"session_closed",
				"The editor document is no longer open.",
			);
		if (context.appId !== this.options.target.appId)
			throw new CraftMcpError(
				"scope_mismatch",
				"The editor belongs to another app.",
			);
		if (context.signal?.aborted)
			throw new CraftMcpError("cancelled", "The editor request was cancelled.");
		if (context.deadline !== undefined && context.deadline <= Date.now())
			throw new CraftMcpError("expired", "The editor request has expired.");
	}

	describe(context: CraftMcpContext) {
		this.assertAvailable(context);
		return { sessionId: this.options.sessionId, editor: this.options.editor };
	}

	private cancel(requestId: string) {
		try {
			this.options.post({ type: "mcp-cancel", requestId });
		} catch {
			// Closing the iframe can make cancellation delivery impossible.
		}
	}

	private pruneCancellations(now: number) {
		for (const [id, entry] of this.cancellations) {
			if (entry.expires > now) continue;
			this.cancellations.delete(id);
			this.cancellationBytes -= entry.bytes;
		}
	}

	private rememberCancellation(id: string | number, now: number) {
		const existing = this.cancellations.get(id);
		const bytes =
			typeof id === "string" ? new TextEncoder().encode(id).byteLength : 8;
		const expires = now + CANCELLATION_TTL_MS;
		if (
			(!existing && this.cancellations.size >= MAX_CANCELLATIONS) ||
			this.cancellationBytes - (existing?.bytes ?? 0) + bytes >
				MAX_CANCELLATION_BYTES
		) {
			// Keep delayed calls blocked when the cancellation cache reaches its limit.
			this.cancellationOverflowUntil = Math.max(
				this.cancellationOverflowUntil,
				expires,
			);
			return;
		}
		this.cancellationBytes += bytes - (existing?.bytes ?? 0);
		this.cancellations.set(id, { expires, bytes });
	}

	async call(message: unknown, context: CraftMcpContext): Promise<unknown> {
		this.assertAvailable(context);
		if (
			!isObject(message) ||
			message.jsonrpc !== "2.0" ||
			typeof message.method !== "string" ||
			(message.id !== undefined &&
				typeof message.id !== "string" &&
				!(typeof message.id === "number" && Number.isSafeInteger(message.id)))
		)
			throw new CraftMcpError(
				"invalid_request",
				"A JSON-RPC 2.0 request is required.",
			);
		if (jsonBytes(message) > MAX_REQUEST_BYTES)
			throw new CraftMcpError(
				"payload_too_large",
				"The editor request exceeds 1 MiB.",
			);
		const now = Date.now();
		this.pruneCancellations(now);
		const method = message.method;
		if (method === "notifications/cancelled" && message.id === undefined) {
			const rpcId = isObject(message.params)
				? message.params.requestId
				: undefined;
			if (
				typeof rpcId === "string" ||
				(typeof rpcId === "number" && Number.isSafeInteger(rpcId))
			) {
				this.rememberCancellation(rpcId, now);
				for (const [requestId, pending] of this.pending) {
					if (pending.message.id !== rpcId) continue;
					this.pending.delete(requestId);
					pending.cleanup();
					this.cancel(requestId);
					pending.reject(
						new CraftMcpError("cancelled", "The editor request was cancelled."),
					);
				}
			}
			return null;
		}
		if (message.id !== undefined) {
			if (this.cancellations.has(message.id as string | number))
				throw new CraftMcpError(
					"cancelled",
					"The editor request was cancelled.",
				);
			if (this.cancellationOverflowUntil > now)
				throw new CraftMcpError(
					"busy",
					"Wait for cancelled editor requests to expire before sending more requests.",
				);
		}
		const readOnly =
			[
				"initialize",
				"notifications/initialized",
				"notifications/cancelled",
				"ping",
				"tools/list",
				"resources/list",
				"resources/templates/list",
				"resources/read",
				"prompts/list",
				"prompts/get",
			].includes(method) ||
			(method === "tools/call" &&
				isObject(message.params) &&
				typeof message.params.name === "string" &&
				this.readOnlyTools.has(message.params.name));
		const state = this.options.state();
		if (!readOnly && (!state.editable || !state.editing))
			throw new CraftMcpError(
				"read_only",
				"Enable editing before changing this document.",
			);
		if (!readOnly && state.busy)
			throw new CraftMcpError(
				"busy",
				"Wait for the document save to finish before changing it.",
			);
		if (this.pending.size >= 8)
			throw new CraftMcpError(
				"busy",
				"The editor is processing too many requests.",
			);
		const requestId = crypto.randomUUID();
		const deadlineAtMs = Math.min(
			context.deadline ?? Number.POSITIVE_INFINITY,
			Date.now() + 120_000,
		);
		return new Promise((resolve, reject) => {
			const fail = (error: Error) => {
				const pending = this.pending.get(requestId);
				if (!pending) return;
				this.pending.delete(requestId);
				pending.cleanup();
				this.cancel(requestId);
				reject(error);
			};
			const abort = () =>
				fail(
					new CraftMcpError("cancelled", "The editor request was cancelled."),
				);
			const timer = setTimeout(
				() =>
					fail(
						new CraftMcpError(
							"timeout",
							"The editor did not answer before the request expired. Inspect the document before retrying a change.",
						),
					),
				Math.max(0, deadlineAtMs - Date.now()),
			);
			this.pending.set(requestId, {
				message,
				resolve,
				reject,
				cleanup: () => {
					clearTimeout(timer);
					context.signal?.removeEventListener("abort", abort);
				},
			});
			context.signal?.addEventListener("abort", abort, { once: true });
			try {
				this.assertAvailable(context);
				this.options.post({
					type: "mcp-request",
					requestId,
					message,
					deadlineAtMs,
				});
			} catch (error) {
				fail(error instanceof Error ? error : new Error(String(error)));
			}
		});
	}

	receive(event: MessageEvent): void {
		const message = event.data;
		if (
			this.disposed ||
			!this.options.state().ready ||
			!this.options.frame() ||
			event.source !== this.options.frame() ||
			event.origin !== this.options.origin ||
			!isObject(message) ||
			message.channel !== this.options.channel ||
			message.session !== this.options.sessionId ||
			typeof message.requestId !== "string" ||
			!["mcp-response", "error"].includes(String(message.type))
		)
			return;
		const pending = this.pending.get(message.requestId);
		if (!pending) return;
		this.pending.delete(message.requestId);
		pending.cleanup();
		if (message.type === "error") {
			pending.reject(
				new CraftMcpError(
					"editor_error",
					String(message.error ?? "The editor request failed."),
				),
			);
			return;
		}
		const response = message.response;
		const notification = pending.message.id === undefined;
		if (
			(notification && response !== null) ||
			(!notification &&
				(!isObject(response) ||
					response.jsonrpc !== "2.0" ||
					response.id !== pending.message.id ||
					Object.hasOwn(response, "result") ===
						Object.hasOwn(response, "error")))
		) {
			pending.reject(
				new CraftMcpError(
					"invalid_response",
					"The editor returned an invalid JSON-RPC response.",
				),
			);
			return;
		}
		try {
			if (jsonBytes(response) > MAX_RESPONSE_BYTES)
				throw new CraftMcpError(
					"payload_too_large",
					"The editor response exceeds 32 MiB.",
				);
		} catch (error) {
			pending.reject(error instanceof Error ? error : new Error(String(error)));
			return;
		}
		if (
			pending.message.method === "tools/list" &&
			isObject(response) &&
			isObject(response.result) &&
			Array.isArray(response.result.tools)
		) {
			if (!isObject(pending.message.params) || !pending.message.params.cursor)
				this.readOnlyTools.clear();
			for (const tool of response.result.tools) {
				if (!isObject(tool) || typeof tool.name !== "string") continue;
				if (
					isObject(tool.annotations) &&
					tool.annotations.readOnlyHint === true
				)
					this.readOnlyTools.add(tool.name);
				else this.readOnlyTools.delete(tool.name);
			}
		}
		pending.resolve(response);
	}

	dispose() {
		if (this.disposed) return;
		this.disposed = true;
		const key = sessionKey(this.options.target.appId, this.elementRef);
		if (sessions.get(key) === this) sessions.delete(key);
		for (const [requestId, pending] of this.pending) {
			pending.cleanup();
			this.cancel(requestId);
			pending.reject(
				new CraftMcpError(
					"session_closed",
					"The editor document was closed or replaced.",
				),
			);
		}
		this.pending.clear();
		this.cancellations.clear();
		this.cancellationBytes = 0;
		this.cancellationOverflowUntil = 0;
	}
}

export async function executeCraftCommand(
	command: string,
	args: JsonObject,
	context: CraftMcpContext,
): Promise<unknown> {
	if (typeof args.elementRef !== "string" || !args.elementRef.includes("/"))
		throw new CraftMcpError(
			"invalid_target",
			"Use the full surface/component reference of the open editor.",
		);
	const session = sessions.get(sessionKey(context.appId, args.elementRef));
	if (!session)
		throw new CraftMcpError(
			"session_unavailable",
			"No open editor matches this app and element reference.",
		);
	if (command === "craft.session") return session.describe(context);
	if (command !== "craft.mcp")
		throw new CraftMcpError("unsupported", "Unknown editor command.");
	if (
		args.sessionId !== session.options.sessionId ||
		args.editor !== session.options.editor
	)
		throw new CraftMcpError(
			"session_changed",
			"The editor document changed. Register its tools again before continuing.",
		);
	return session.call(args.message, context);
}
