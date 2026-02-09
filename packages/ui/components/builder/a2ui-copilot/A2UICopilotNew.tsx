"use client";

import type { SurfaceComponent } from "../../a2ui/types";
import { FlowPilot } from "../../flowpilot";

export interface A2UICopilotProps {
	currentComponents: SurfaceComponent[];
	selectedComponentIds: string[];
	onComponentsGenerated?: (components: SurfaceComponent[]) => void;
	onApplyComponents?: (
		components: SurfaceComponent[],
		canvasSettings?: {
			backgroundColor?: string;
			padding?: string;
			customCss?: string;
		},
	) => void;
	className?: string;
	onClose?: () => void;
	/** Custom function to capture a screenshot for context */
	captureScreenshot?: () => Promise<string | null>;
}

/**
 * A2UICopilot - AI assistant for UI generation
 *
 * This is a wrapper around the unified FlowPilot component
 * configured for UI mode (agentMode="ui")
 */
export function A2UICopilot({
	currentComponents,
	selectedComponentIds,
	onComponentsGenerated,
	onApplyComponents,
	className,
	onClose,
	captureScreenshot,
}: A2UICopilotProps) {
	return (
		<FlowPilot
			agentMode="ui"
			title="FlowPilot"
			className={className}
			onClose={onClose}
			currentComponents={currentComponents}
			selectedComponentIds={selectedComponentIds}
			onComponentsGenerated={onComponentsGenerated}
			onApplyComponents={onApplyComponents}
			captureScreenshot={captureScreenshot}
		/>
	);
}

export default A2UICopilot;
