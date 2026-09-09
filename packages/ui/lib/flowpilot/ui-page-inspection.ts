import type { IPage } from "../../state/backend-state/page-state";

interface UiInspectPageRef {
	pageId: string;
	name: string;
	boardId?: string;
}

export function summarizeUiInspectPage(page: IPage) {
	const customCss = page.canvasSettings?.customCss ?? "";
	return {
		page_id: page.id,
		name: page.name,
		route: page.route,
		on_load_event_id: page.onLoadEventId,
		on_interval_event_id: page.onIntervalEventId,
		// The UI specialist receives the stylesheet; the orchestrator only needs its size.
		custom_css_chars: customCss.length,
		element_refs: (page.components ?? []).map(
			(component) => `${page.id}/${component.id}`,
		),
	};
}

/** Preserve listed identities when a detail read fails, without claiming an empty page. */
export function inspectUiPageList(
	pages: readonly UiInspectPageRef[],
	readPage: (page: UiInspectPageRef) => Promise<IPage>,
) {
	return Promise.all(
		pages.map(async (page) => {
			try {
				return summarizeUiInspectPage(await readPage(page));
			} catch {
				return {
					page_id: page.pageId,
					name: page.name,
					element_refs: [],
					error: "Page details could not be loaded.",
				};
			}
		}),
	);
}
