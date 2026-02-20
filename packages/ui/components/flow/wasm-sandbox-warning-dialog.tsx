"use client";
import {
	ExternalLinkIcon,
	KeyIcon,
	PackageIcon,
	ShieldAlertIcon,
} from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Checkbox } from "../ui/checkbox";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";
import { Label } from "../ui/label";
import { ScrollArea } from "../ui/scroll-area";
import { Separator } from "../ui/separator";

const PERMISSION_LABELS: Record<string, { label: string; icon: string }> = {
	"network:http": { label: "Network Access (HTTP)", icon: "🌐" },
	"network:websocket": { label: "WebSocket Connections", icon: "🔌" },
	"storage:read": { label: "Storage Read", icon: "📖" },
	"storage:write": { label: "Storage Write", icon: "💾" },
	"storage:node": { label: "Node Storage", icon: "📦" },
	"storage:user": { label: "User Storage", icon: "👤" },
	variables: { label: "Flow Variables", icon: "🔀" },
	cache: { label: "Execution Cache", icon: "⚡" },
	streaming: { label: "Streaming Output", icon: "📡" },
	models: { label: "AI Model Access", icon: "🤖" },
	a2ui: { label: "Dynamic UI (A2UI)", icon: "🖼️" },
	oauth: { label: "OAuth Authentication", icon: "🔑" },
	functions: { label: "Function Calls", icon: "⚙️" },
};

function formatPermission(perm: string): { label: string; icon: string } {
	return (
		PERMISSION_LABELS[perm] ?? {
			label: perm,
			icon: "🔒",
		}
	);
}

export type RememberChoice = "none" | "board" | "event" | "package";

export interface WasmSandboxWarningDialogProps {
	open: boolean;
	packageIds: string[];
	packagePermissions?: Record<string, string[]>;
	onConfirm: (rememberFor: RememberChoice) => void;
	onCancel: () => void;
}

export function WasmSandboxWarningDialog({
	open,
	packageIds,
	packagePermissions,
	onConfirm,
	onCancel,
}: WasmSandboxWarningDialogProps) {
	const [rememberChoice, setRememberChoice] = useState<RememberChoice>("none");

	const handleConfirm = useCallback(() => {
		onConfirm(rememberChoice);
	}, [onConfirm, rememberChoice]);

	const allPermissions = useMemo(() => {
		if (!packagePermissions) return new Map<string, string[]>();
		const result = new Map<string, string[]>();
		for (const pkgId of packageIds) {
			const perms = packagePermissions[pkgId];
			if (perms?.length) result.set(pkgId, perms);
		}
		return result;
	}, [packageIds, packagePermissions]);

	const hasPermissions = allPermissions.size > 0;

	return (
		<Dialog open={open} onOpenChange={(o) => !o && onCancel()}>
			<DialogContent className="max-w-lg">
				<DialogHeader>
					<div className="flex items-center gap-2">
						<ShieldAlertIcon className="w-5 h-5 text-amber-500" />
						<DialogTitle>Sideloaded WASM nodes detected</DialogTitle>
					</div>
					<DialogDescription>
						This workflow contains externally-loaded WebAssembly nodes. They run
						inside an isolated sandbox, but you should only run code you trust.{" "}
						<a
							href="https://docs.flow-like.com/dev/wasm-nodes/sandboxing/"
							target="_blank"
							rel="noopener noreferrer"
							className="inline-flex items-center gap-0.5 text-primary underline underline-offset-2 hover:text-primary/80"
						>
							Learn more
							<ExternalLinkIcon className="w-3 h-3" />
						</a>
					</DialogDescription>
				</DialogHeader>

				<ScrollArea className="max-h-[40vh]">
					<div className="flex flex-col gap-3 py-1 pr-3">
						{packageIds.map((id) => {
							const perms = allPermissions.get(id);
							return (
								<div
									key={id}
									className="flex flex-col gap-1.5 rounded-md border p-2.5"
								>
									<div className="flex items-center gap-1.5">
										<PackageIcon className="w-3.5 h-3.5 text-muted-foreground" />
										<span className="text-sm font-medium">{id}</span>
									</div>
									{perms && perms.length > 0 ? (
										<div className="flex flex-wrap gap-1">
											{perms.map((p) => {
												const { label, icon } = formatPermission(p);
												return (
													<Badge
														key={p}
														variant="outline"
														className="flex items-center gap-1 text-xs"
													>
														<span>{icon}</span>
														{label}
													</Badge>
												);
											})}
										</div>
									) : (
										<span className="text-xs text-muted-foreground">
											No additional permissions requested
										</span>
									)}
								</div>
							);
						})}
					</div>
				</ScrollArea>

				{hasPermissions && (
					<>
						<Separator />
						<div className="flex items-start gap-2 text-xs text-muted-foreground">
							<KeyIcon className="w-3.5 h-3.5 mt-0.5 shrink-0" />
							<span>
								Permissions are declared by each node and enforced by the
								sandbox at runtime.
							</span>
						</div>
					</>
				)}

				<div className="flex flex-col gap-3 pt-1">
					<p className="text-sm text-muted-foreground font-medium">
						Remember my choice:
					</p>
					<div className="flex flex-col gap-2">
						<label className="flex items-center gap-2 cursor-pointer">
							<Checkbox
								checked={rememberChoice === "event"}
								onCheckedChange={(c) => setRememberChoice(c ? "event" : "none")}
							/>
							<Label className="cursor-pointer">For this event</Label>
						</label>
						<label className="flex items-center gap-2 cursor-pointer">
							<Checkbox
								checked={rememberChoice === "board"}
								onCheckedChange={(c) => setRememberChoice(c ? "board" : "none")}
							/>
							<Label className="cursor-pointer">For this entire board</Label>
						</label>
						<label className="flex items-center gap-2 cursor-pointer">
							<Checkbox
								checked={rememberChoice === "package"}
								onCheckedChange={(c) =>
									setRememberChoice(c ? "package" : "none")
								}
							/>
							<Label className="cursor-pointer">
								Trust these packages everywhere
							</Label>
						</label>
					</div>
				</div>

				<DialogFooter>
					<Button variant="outline" onClick={onCancel}>
						Cancel
					</Button>
					<Button onClick={handleConfirm}>Run anyway</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
