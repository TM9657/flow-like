import { CheckIcon, CopyIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "../ui/button";
import type { CraftToolTarget } from "./craft-mcp-session";

export function CraftMcpTargetButton({
	target,
	onError,
}: {
	target: CraftToolTarget;
	onError: (message: string) => void;
}) {
	const elementRef = `${target.surfaceId}/${target.componentId}`;
	const [copied, setCopied] = useState(false);
	const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
	useEffect(() => () => clearTimeout(timer.current), []);
	return (
		<Button
			variant="ghost"
			size="sm"
			className="shrink-0"
			aria-label="Copy agent target"
			title={`Copy agent target: ${elementRef}`}
			onClick={async () => {
				try {
					await navigator.clipboard.writeText(elementRef);
					setCopied(true);
					clearTimeout(timer.current);
					timer.current = setTimeout(() => setCopied(false), 2000);
				} catch {
					onError(`Could not copy the agent target. Use ${elementRef}.`);
				}
			}}
		>
			{copied ? <CheckIcon /> : <CopyIcon />}
			<span className="hidden sm:inline">
				{copied ? "Copied" : "Agent target"}
			</span>
		</Button>
	);
}
