import type { INode } from "../schema";
import { translateBpmn } from "./bpmn-translator";
import { translateDify } from "./dify-translator";
import { translateN8n } from "./n8n-translator";
import type { ImportDetection, TranslationResult } from "./types";

/**
 * Turns a detected document into a board fragment. One dispatch for every
 * caller — the import dialog and the canvas paste path — so a new format is
 * wired in once.
 */
export function translateImport(
	detection: ImportDetection,
	catalog?: INode[],
): TranslationResult | undefined {
	switch (detection.format) {
		case "bpmn":
			return translateBpmn(detection.parsed, catalog);
		case "n8n":
			return translateN8n(detection.parsed, catalog);
		case "dify":
			return translateDify(detection.parsed);
		default:
			return undefined;
	}
}
