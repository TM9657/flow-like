import type { SurfaceComponent } from "../../components/a2ui/types";
import { createCopilotStreamParser } from "../../components/flowpilot/copilot-stream-parser";
import { validateComponents } from "../../components/flowpilot/validateComponents";
import type { IBoardState } from "../../state/backend-state/board-state";

export interface BuildWidgetGenerationContext {
	appId: string;
	requestId: string;
	parentRequestId: string;
	conversationId?: string;
	runId?: string;
	sourceUserPrompt?: string;
	modelId?: string;
	reasoningEffort?: string;
	signal?: AbortSignal;
	assertActive(): void;
}

/** A standalone widget uses the existing UI specialist without creating a helper page. */
export async function generateAppBuildWidget(
	boards: IBoardState,
	instruction: string,
	context: BuildWidgetGenerationContext,
): Promise<SurfaceComponent[]> {
	context.assertActive();
	context.signal?.throwIfAborted();
	const parser = createCopilotStreamParser();
	const streamed: SurfaceComponent[] = [];
	const consume = (events: ReturnType<typeof parser.push>) => {
		for (const event of events) {
			if (event.type === "components" && Array.isArray(event.data)) {
				streamed.push(...(event.data as SurfaceComponent[]));
			}
		}
	};
	const cancel = () => {
		void boards.cancelCopilotChat?.(context.requestId).catch(() => {});
	};
	context.signal?.addEventListener("abort", cancel, { once: true });
	try {
		const response = await boards.copilot_chat(
			"Frontend",
			null,
			undefined,
			[],
			[],
			null,
			[],
			instruction +
				"\nReturn one complete reusable widget whose root component has id 'root'. Do not create pages or workflows.",
			[],
			undefined,
			(chunk) => consume(parser.push(chunk)),
			context.modelId,
			context.reasoningEffort,
			undefined,
			undefined,
			undefined,
			true,
			false,
			{
				appId: context.appId,
				parentRequestId: context.parentRequestId,
				conversationId: context.conversationId,
				runId: context.runId,
				sourceUserPrompt: context.sourceUserPrompt,
			},
			context.requestId,
			context.sourceUserPrompt,
			context.appId,
		);
		consume(parser.flush());
		context.assertActive();
		context.signal?.throwIfAborted();
		const raw = response.components?.length ? response.components : streamed;
		const validated = validateComponents(raw);
		if (validated.components.length !== raw.length)
			throw new Error("Widget contains invalid components.");
		const rootId = response.root_component_id || "root";
		const root = validated.components.find(
			(component) => component.id === rootId,
		);
		if (!root)
			throw new Error("UI specialist did not return the declared widget root.");
		return [
			root,
			...validated.components.filter((component) => component.id !== rootId),
		];
	} finally {
		context.signal?.removeEventListener("abort", cancel);
	}
}
