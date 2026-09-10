import {
	type LivePageHandle,
	isLivePageComponentEffectivelyHidden,
	livePageComponentChildIds,
} from "@flow-like/flow-like-ui/components/a2ui/live-page-registry";

/** Read the reserved result from this Page's visible DOM after its workflow settles. */
export function readIntakeRenderedQueue(
	handle: Pick<
		LivePageHandle,
		"pageId" | "getSurface" | "getContainer" | "resolveBoundValue"
	>,
): string {
	const componentId = "queue_result";
	const surface = handle.getSurface();
	const container = handle.getContainer?.();
	if (!surface || surface.id !== handle.pageId || !container?.isConnected)
		throw new Error("The intake result has no connected Page surface.");
	const reachable = new Set<string>();
	const pending = [surface.rootComponentId];
	while (pending.length) {
		const id = pending.pop();
		if (!id || reachable.has(id)) continue;
		const component = surface.components[id]?.component;
		if (!component) continue;
		reachable.add(id);
		pending.push(...livePageComponentChildIds(component));
	}
	if (
		!reachable.has(componentId) ||
		isLivePageComponentEffectivelyHidden(surface, componentId, (value) =>
			handle.resolveBoundValue ? handle.resolveBoundValue(value) : value,
		)
	)
		throw new Error(
			"queue_result is hidden or unreachable from the Page root.",
		);

	const reference = `${handle.pageId}/${componentId}`;
	const matches = Array.from(
		container.querySelectorAll<HTMLElement>("[data-a2ui-element-ref]"),
	).filter(
		(element) => element.getAttribute("data-a2ui-element-ref") === reference,
	);
	if (matches.length !== 1)
		throw new Error(
			"queue_result must render exactly one DOM element on this Page.",
		);
	const element = matches[0];
	const view = element.ownerDocument.defaultView;
	if (
		!view ||
		!element.isConnected ||
		!Array.from(element.getClientRects()).some(
			(rect) => rect.width > 0 && rect.height > 0,
		)
	)
		throw new Error("queue_result has no visible DOM geometry.");
	for (
		let current: HTMLElement | null = element;
		current;
		current = current.parentElement
	) {
		const style = view.getComputedStyle(current);
		if (
			current.hidden ||
			style.display === "none" ||
			style.visibility === "hidden" ||
			style.visibility === "collapse" ||
			style.contentVisibility === "hidden" ||
			Number.parseFloat(style.opacity) === 0
		)
			throw new Error(
				"queue_result is hidden by its rendered DOM or an ancestor.",
			);
	}
	return element.innerText.trim();
}
