"use client";

import { useTranslation } from "@flow-like/locales";
import { Workflow } from "lucide-react";
import { useId } from "react";
import { useBoardRenderer } from "../../hooks/use-board-renderer";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../ui/card";
import { Label } from "../ui/label";
import { Switch } from "../ui/switch";

export function BoardRendererCard() {
	const { t } = useTranslation("settings");
	const { renderer, setRenderer, ready } = useBoardRenderer();
	const id = useId();

	return (
		<Card id="board-renderer-card">
			<CardHeader>
				<CardTitle className="flex items-center gap-2">
					<Workflow className="h-5 w-5" aria-hidden="true" />
					{t("workflowEditor", "Workflow editor")}
				</CardTitle>
				<CardDescription id={`${id}-description`}>
					{t(
						"boardRendererDefaultDescription",
						"WASM is the default board renderer. Turn it off to use React Flow.",
					)}
				</CardDescription>
			</CardHeader>
			<CardContent>
				<div className="flex items-center justify-between gap-4">
					<div className="space-y-0.5">
						<Label htmlFor={id}>
							{t("useWasmBoardPreview", "Use WASM board (preview)")}
						</Label>
						<p id={`${id}-scope`} className="text-sm text-muted-foreground">
							{t(
								"boardRendererLocalPreference",
								"Your choice is saved automatically for this browser or desktop installation.",
							)}
						</p>
					</div>
					<Switch
						id={id}
						aria-describedby={`${id}-description ${id}-scope`}
						checked={renderer === "egui"}
						disabled={!ready}
						onCheckedChange={(checked) =>
							setRenderer(checked ? "egui" : "react-flow")
						}
					/>
				</div>
			</CardContent>
		</Card>
	);
}
