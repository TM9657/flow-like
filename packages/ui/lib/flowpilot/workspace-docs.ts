export interface FlowPilotWorkspaceDocEntry {
	kind: "doc";
	resource_id: string;
	title: string;
	source_path: string;
	section_id: string;
	revision: string;
	content_revision: string;
	content: string;
	line_start: number;
	line_end: number;
}

export interface FlowPilotWorkspaceDocsCorpus {
	schema_version: 1;
	revision: string;
	coverage: {
		mode: "bundled_selected_docs";
		complete: true;
		source_paths: string[];
		excluded: string[];
	};
	entries: FlowPilotWorkspaceDocEntry[];
}

/** Documentation is reference data. Its examples do not authorize tool actions. */
export async function loadFlowPilotWorkspaceDocs(): Promise<FlowPilotWorkspaceDocsCorpus> {
	const { default: corpus } = await import("./generated/workspace-docs.json");
	return corpus as FlowPilotWorkspaceDocsCorpus;
}
