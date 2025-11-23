"use client";

import { Activity, Loader2, MessageSquare } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { useExecutionEngine } from "../state/execution-engine-context";
import { Badge } from "./ui/badge";
import { Card } from "./ui/card";
import { TextEditor } from "./ui/text-editor";

export function RunningTasksIndicator() {
	const engine = useExecutionEngine();
	const router = useRouter();
	const [tasks, setTasks] = useState<
		{
			streamId: string;
			path?: string;
			title?: string;
			preview?: string;
			interfaceType?: string;
		}[]
	>([]);

	useEffect(() => {
		const updateTasks = () => {
			const backgroundStreams = engine.getBackgroundStreams();
			setTasks(backgroundStreams);
		};

		// Initial update
		updateTasks();

		// Subscribe to updates
		const unsubscribe = engine.subscribeToGlobalUpdates(updateTasks);

		return () => {
			unsubscribe();
		};
	}, [engine]);

	if (tasks.length === 0) {
		return null;
	}

	const getIcon = (interfaceType?: string) => {
		switch (interfaceType) {
			case "chat":
				return <MessageSquare className="h-4 w-4 text-primary" />;
			default:
				return <Activity className="h-4 w-4 text-primary" />;
		}
	};

	return (
		<div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm w-full">
			{tasks.map((task) => (
				<Card
					key={task.streamId}
					className="p-3 cursor-pointer hover:bg-accent/50 transition-colors shadow-lg border-primary/20 backdrop-blur-sm bg-background/80"
					onClick={() => {
						if (task.path) {
							router.push(task.path);
						}
					}}
				>
					<div className="flex items-start gap-3">
						<div className="mt-1 relative">
							<Loader2 className="h-4 w-4 animate-spin text-primary absolute opacity-30" />
							<div className="relative z-10">{getIcon(task.interfaceType)}</div>
						</div>
						<div className="flex-1 min-w-0">
							<div className="flex items-center justify-between gap-2 mb-1">
								<h4 className="text-sm font-medium truncate">
									{task.title || "Running Task"}
								</h4>
								<Badge variant="outline" className="text-[10px] h-5 px-1.5">
									Running
								</Badge>
							</div>
							{task.preview && (
								<div className="text-xs text-muted-foreground line-clamp-2 font-mono bg-muted/50 p-1 rounded">
									<TextEditor
										initialContent={task.preview}
										isMarkdown={true}
										editable={false}
										minimal={true}
									/>
								</div>
							)}
						</div>
					</div>
				</Card>
			))}
		</div>
	);
}
