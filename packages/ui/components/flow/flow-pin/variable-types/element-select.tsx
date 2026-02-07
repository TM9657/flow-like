import { ChevronDown, Layers } from "lucide-react";
import { useEffect, useState } from "react";
import { useBackend } from "../../../..";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
} from "../../../../components/ui/select";
import type { IPin } from "../../../../lib/schema/flow/pin";
import {
	convertJsonToUint8Array,
	parseUint8ArrayToJson,
} from "../../../../lib/uint8";
import type { SurfaceComponent } from "../../../a2ui/types";

interface ElementSelectProps {
	readonly pin: IPin;
	readonly value: number[] | undefined | null;
	readonly appId: string;
	readonly setValue: (value: unknown) => void;
}

interface ElementOption {
	id: string;
	rawId: string;
	type: string;
	label: string;
	pageName?: string;
	pagePath?: string;
}

function flattenElements(components: SurfaceComponent[]): ElementOption[] {
	const elements: ElementOption[] = [];

	for (const component of components) {
		const componentObj = component.component;
		if (typeof componentObj === "object" && componentObj !== null) {
			const type =
				((componentObj as unknown as Record<string, unknown>).type as string) ||
				"unknown";
			elements.push({
				id: component.id,
				type,
				label: component.id,
				rawId: component.id,
			});
		}
	}

	return elements;
}

export function ElementSelect({
	pin,
	value,
	appId,
	setValue,
}: ElementSelectProps) {
	const backend = useBackend();
	const [elements, setElements] = useState<ElementOption[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		async function loadElements() {
			setLoading(true);
			try {
				const [routes, events, pages] = await Promise.all([
					backend.routeState.getRoutes(appId),
					backend.eventState.getEvents(appId),
					backend.pageState.getPages(appId),
				]);
				const eventsMap = new Map(events.map((e) => [e.id, e]));
				const pagesById = new Map(pages.map((page) => [page.pageId, page]));
				const allElements: ElementOption[] = [];
				const seenIds = new Set<string>();

				const addPageElements = async (
					pageId: string,
					pageName?: string,
					pagePath?: string,
					boardId?: string,
				) => {
					try {
						const page = await backend.pageState.getPage(
							appId,
							pageId,
							boardId,
						);
						if (page?.components) {
							const pageElements = flattenElements(page.components);
							for (const el of pageElements) {
								const optionId = `${pageId}/${el.id}`;
								if (!seenIds.has(optionId)) {
									seenIds.add(optionId);
									allElements.push({
										...el,
										id: optionId,
										rawId: el.id,
										label: pageName
											? `${pageName} / ${el.label}`
											: el.label,
										pageName,
										pagePath,
									});
								}
							}
						}
					} catch {
						// Skip pages that fail to load
					}
				};

				for (const route of routes) {
					const event = eventsMap.get(route.eventId);
					const pageId = event?.default_page_id;
					if (pageId) {
						const pageInfo = pagesById.get(pageId);
						await addPageElements(
							pageId,
							pageInfo?.name,
							route.path,
							pageInfo?.boardId,
						);
					}
				}

				for (const pageInfo of pages) {
					await addPageElements(
						pageInfo.pageId,
						pageInfo.name,
						undefined,
						pageInfo.boardId,
					);
				}

				setElements(allElements);
			} catch (error) {
				console.error("Failed to load page elements:", error);
			} finally {
				setLoading(false);
			}
		}

		loadElements();
	}, [backend, appId]);

	const currentValue = parseUint8ArrayToJson(value);
	const selectedElement = elements.find(
		(el) => el.id === currentValue || el.rawId === currentValue,
	);

	return (
		<div className="flex flex-row items-center justify-start w-fit max-w-full ml-1 overflow-hidden">
			<Select
				defaultValue={currentValue}
				value={currentValue}
				onValueChange={(val) => setValue(convertJsonToUint8Array(val))}
			>
				<SelectTrigger
					noChevron
					size="sm"
					className="w-fit! max-w-full! p-0 border-0 text-xs bg-card! text-start max-h-fit h-4 gap-0.5 flex-row items-center overflow-hidden"
				>
					<Layers className="size-2 min-w-2 min-h-2 text-muted-foreground mr-0.5 shrink-0" />
					<small className="text-start text-[10px] m-0! truncate">
						{loading && "Loading..."}
						{!loading && (selectedElement?.label ?? "No Element Selected")}
					</small>
					<ChevronDown className="size-2 min-w-2 min-h-2 text-card-foreground shrink-0" />
				</SelectTrigger>
				<SelectContent className="bg-background max-h-60 overflow-y-auto">
					<SelectGroup>
						<SelectLabel>{pin.friendly_name}</SelectLabel>
						{elements.map((element) => (
							<SelectItem key={element.id} value={element.id}>
								<div className="flex items-center gap-2">
									<span>{element.label}</span>
									<span className="text-xs text-muted-foreground">
										{element.type}
									</span>
								</div>
							</SelectItem>
						))}
					</SelectGroup>
				</SelectContent>
			</Select>
		</div>
	);
}
