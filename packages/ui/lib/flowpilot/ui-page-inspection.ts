import type { IPage } from "../../state/backend-state/page-state";

interface UiInspectPageRef {
	pageId: string;
	name: string;
	boardId?: string;
}

/**
 * Size of the app-wide stylesheet layered above every page. `null` means the caller
 * did not resolve that layer, which is not the same claim as an app with no CSS —
 * reporting `0` there is what makes an agent paste an app sheet into a single page.
 */
export function appCustomCssChars(appCustomCss: string | null | undefined) {
	return appCustomCss === undefined ? null : (appCustomCss?.length ?? 0);
}

export function summarizeUiInspectPage(
	page: IPage,
	appCustomCss?: string | null,
) {
	const customCss = page.canvasSettings?.customCss ?? "";
	return {
		page_id: page.id,
		name: page.name,
		route: page.route,
		on_load_event_id: page.onLoadEventId,
		on_interval_event_id: page.onIntervalEventId,
		// The UI specialist receives the stylesheet; the orchestrator only needs its size.
		custom_css_chars: customCss.length,
		app_custom_css_chars: appCustomCssChars(appCustomCss),
		element_refs: (page.components ?? []).map(
			(component) => `${page.id}/${component.id}`,
		),
	};
}

/** Preserve listed identities when a detail read fails, without claiming an empty page. */
export function inspectUiPageList(
	pages: readonly UiInspectPageRef[],
	readPage: (page: UiInspectPageRef) => Promise<IPage>,
	appCustomCss?: string | null,
) {
	return Promise.all(
		pages.map(async (page) => {
			try {
				return summarizeUiInspectPage(await readPage(page), appCustomCss);
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
