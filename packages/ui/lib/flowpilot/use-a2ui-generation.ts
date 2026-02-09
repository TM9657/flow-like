/**
 * FlowPilot A2UI Generation Hook
 * Provides streaming UI generation capabilities
 */

import { useCallback, useState } from "react";
import type { DataEntry, SurfaceComponent } from "../../components/a2ui/types";
import { buildA2UIPrompt, parseA2UIResponse } from "./a2ui-prompts";

export interface A2UIGenerationState {
	isGenerating: boolean;
	progress: number;
	error: string | null;
	preview: {
		rootComponentId: string | null;
		components: SurfaceComponent[];
		dataModel: DataEntry[];
	};
}

export interface UseA2UIGenerationOptions {
	onGenerate?: (request: string) => Promise<string>;
	onComplete?: (components: SurfaceComponent[], dataModel: DataEntry[]) => void;
	onError?: (error: string) => void;
	existingComponents?: string[];
	existingDataModel?: Record<string, unknown>;
}

export function useA2UIGeneration(options: UseA2UIGenerationOptions = {}) {
	const {
		onGenerate,
		onComplete,
		onError,
		existingComponents,
		existingDataModel,
	} = options;

	const [state, setState] = useState<A2UIGenerationState>({
		isGenerating: false,
		progress: 0,
		error: null,
		preview: {
			rootComponentId: null,
			components: [],
			dataModel: [],
		},
	});

	const generate = useCallback(
		async (userRequest: string) => {
			if (!onGenerate) {
				setState((prev) => ({
					...prev,
					error: "No generation handler provided",
				}));
				onError?.("No generation handler provided");
				return null;
			}

			setState((prev) => ({
				...prev,
				isGenerating: true,
				progress: 0,
				error: null,
			}));

			try {
				const prompt = buildA2UIPrompt(userRequest, {
					existingComponents,
					dataModel: existingDataModel,
				});

				setState((prev) => ({ ...prev, progress: 30 }));

				const response = await onGenerate(prompt);

				setState((prev) => ({ ...prev, progress: 70 }));

				const parsed = parseA2UIResponse(response);

				if (!parsed) {
					throw new Error("Failed to parse A2UI response");
				}

				const components = parsed.components as SurfaceComponent[];
				const dataModel = parsed.dataModel as DataEntry[];

				setState((prev) => ({
					...prev,
					progress: 100,
					preview: {
						rootComponentId: parsed.rootComponentId,
						components,
						dataModel,
					},
				}));

				onComplete?.(components, dataModel);

				return {
					rootComponentId: parsed.rootComponentId,
					components,
					dataModel,
				};
			} catch (error) {
				const errorMessage =
					error instanceof Error ? error.message : "Generation failed";
				setState((prev) => ({ ...prev, error: errorMessage }));
				onError?.(errorMessage);
				return null;
			} finally {
				setState((prev) => ({ ...prev, isGenerating: false }));
			}
		},
		[onGenerate, onComplete, onError, existingComponents, existingDataModel],
	);

	const clearPreview = useCallback(() => {
		setState((prev) => ({
			...prev,
			preview: {
				rootComponentId: null,
				components: [],
				dataModel: [],
			},
			error: null,
		}));
	}, []);

	const clearError = useCallback(() => {
		setState((prev) => ({ ...prev, error: null }));
	}, []);

	return {
		...state,
		generate,
		clearPreview,
		clearError,
	};
}

export interface StreamingGenerationOptions {
	onChunk?: (chunk: string) => void;
	onProgress?: (progress: number) => void;
}

export async function* streamA2UIGeneration(
	generator: AsyncGenerator<string, void, unknown>,
	options: StreamingGenerationOptions = {},
): AsyncGenerator<Partial<A2UIGenerationState>, void, unknown> {
	const { onChunk, onProgress } = options;
	let accumulated = "";
	let progress = 0;

	yield { isGenerating: true, progress: 0 };

	try {
		for await (const chunk of generator) {
			accumulated += chunk;
			progress = Math.min(progress + 5, 90);
			onChunk?.(chunk);
			onProgress?.(progress);

			yield { progress };

			const partialParsed = tryParsePartialA2UI(accumulated);
			if (partialParsed) {
				yield {
					preview: {
						rootComponentId: partialParsed.rootComponentId,
						components: partialParsed.components as SurfaceComponent[],
						dataModel: (partialParsed.dataModel || []) as DataEntry[],
					},
				};
			}
		}

		const finalParsed = parseA2UIResponse(accumulated);
		if (finalParsed) {
			yield {
				isGenerating: false,
				progress: 100,
				preview: {
					rootComponentId: finalParsed.rootComponentId,
					components: finalParsed.components as SurfaceComponent[],
					dataModel: finalParsed.dataModel as DataEntry[],
				},
			};
		} else {
			yield {
				isGenerating: false,
				error: "Failed to parse final A2UI response",
			};
		}
	} catch (error) {
		yield {
			isGenerating: false,
			error:
				error instanceof Error ? error.message : "Stream generation failed",
		};
	}
}

function tryParsePartialA2UI(text: string): {
	rootComponentId: string;
	components: unknown[];
	dataModel?: unknown[];
} | null {
	try {
		const jsonMatch = text.match(/```json\s*([\s\S]*?)(?:```|$)/);
		if (!jsonMatch) return null;

		let jsonStr = jsonMatch[1].trim();

		if (!jsonStr.endsWith("}")) {
			const lastBrace = jsonStr.lastIndexOf("}");
			if (lastBrace === -1) return null;
			jsonStr = jsonStr.substring(0, lastBrace + 1);
		}

		const parsed = JSON.parse(jsonStr);
		if (parsed.rootComponentId && Array.isArray(parsed.components)) {
			return parsed;
		}
		return null;
	} catch {
		return null;
	}
}
