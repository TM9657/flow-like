import { useEffect, useRef } from "react";
import { CraftMcpSession, type CraftToolTarget } from "./craft-mcp-session";

export function useCraftMcpSession(options: {
	target?: CraftToolTarget;
	sessionId: string;
	editor: string;
	channel: string;
	frame: () => Window | null;
	post: (message: Record<string, unknown>) => void;
	ready: boolean;
	editable: boolean;
	editing: boolean;
	busy: boolean;
}) {
	const latest = useRef(options);
	latest.current = options;
	const { target, sessionId, editor, channel, ready } = options;
	const appId = target?.appId;
	const surfaceId = target?.surfaceId;
	const componentId = target?.componentId;
	useEffect(() => {
		if (!ready || !appId || !surfaceId || !componentId) return;
		const session = new CraftMcpSession({
			target: { appId, surfaceId, componentId },
			sessionId,
			editor,
			channel,
			origin: window.location.origin,
			frame: () => latest.current.frame(),
			post: (message) => latest.current.post(message),
			state: () => {
				const current = latest.current;
				return {
					...current,
					ready:
						current.ready &&
						current.sessionId === sessionId &&
						current.editor === editor &&
						current.target?.appId === appId &&
						current.target.surfaceId === surfaceId &&
						current.target.componentId === componentId,
				};
			},
		});
		const receive = (event: MessageEvent) => session.receive(event);
		window.addEventListener("message", receive);
		return () => {
			window.removeEventListener("message", receive);
			session.dispose();
		};
	}, [appId, surfaceId, componentId, sessionId, editor, channel, ready]);
}
