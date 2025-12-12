"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	ArrowLeft,
	Cloud,
	CloudOff,
	FolderPlus,
	Loader2,
	Rocket,
	Sparkles,
} from "lucide-react";
import type * as React from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "../../lib/utils";
import { useSpotlightStore } from "../../state/spotlight-state";
import { Button } from "../ui/button";
import { Label } from "../ui/label";

interface QuickProjectCreateProps {
	onCreateProject: (
		name: string,
		isOffline: boolean,
	) => Promise<{ appId: string; boardId: string } | null>;
}

export function QuickProjectCreate({
	onCreateProject,
}: QuickProjectCreateProps) {
	const { setMode, close } = useSpotlightStore();
	const [name, setName] = useState("");
	const [isOffline, setIsOffline] = useState(true);
	const [isCreating, setIsCreating] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const inputRef = useRef<HTMLInputElement>(null);

	useEffect(() => {
		inputRef.current?.focus();
	}, []);

	const handleCreate = useCallback(async () => {
		if (!name.trim()) {
			setError("Please enter a project name");
			return;
		}

		setIsCreating(true);
		setError(null);

		try {
			const result = await onCreateProject(name.trim(), isOffline);
			if (result) {
				close();
			} else {
				setError("Failed to create project");
			}
		} catch (err) {
			setError("Failed to create project. Please try again.");
		} finally {
			setIsCreating(false);
		}
	}, [name, isOffline, onCreateProject, close]);

	const handleKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			handleCreate();
		}
		if (e.key === "Escape") {
			setMode("search");
		}
	};

	return (
		<div className="flex flex-col">
			<div className="flex items-center gap-2 px-4 py-3 border-b border-border/40">
				<Button
					variant="ghost"
					size="icon"
					className="h-8 w-8"
					onClick={() => setMode("search")}
				>
					<ArrowLeft className="h-4 w-4" />
				</Button>
				<div className="flex items-center gap-2">
					<div className="h-8 w-8 rounded-lg bg-linear-to-br from-emerald-500/20 to-teal-500/20 flex items-center justify-center">
						<FolderPlus className="h-4 w-4 text-emerald-500" />
					</div>
					<div>
						<h3 className="text-sm font-semibold">Quick Create</h3>
						<p className="text-[10px] text-muted-foreground">New Project</p>
					</div>
				</div>
			</div>

			<div className="p-4 space-y-4">
				<motion.div
					initial={{ opacity: 0, y: 10 }}
					animate={{ opacity: 1, y: 0 }}
					className="space-y-4"
				>
					<div className="space-y-2">
						<Label htmlFor="project-name" className="text-xs font-medium">
							Project Name
						</Label>
						<input
							ref={inputRef}
							id="project-name"
							type="text"
							value={name}
							onChange={(e) => {
								setName(e.target.value);
								setError(null);
							}}
							onKeyDown={handleKeyDown}
							placeholder="My awesome project..."
							disabled={isCreating}
							className="w-full h-11 px-4 rounded-lg bg-muted/50 text-sm outline-none placeholder:text-muted-foreground/60 focus:ring-2 focus:ring-primary/20 transition-all disabled:opacity-50"
						/>
					</div>

					<div className="space-y-2">
						<Label className="text-xs font-medium">Mode</Label>
						<div className="grid grid-cols-2 gap-2">
							<button
								type="button"
								onClick={() => setIsOffline(true)}
								disabled={isCreating}
								className={cn(
									"flex flex-col items-center gap-2 p-4 rounded-xl border-2 transition-all",
									isOffline
										? "border-primary bg-primary/5"
										: "border-border/40 hover:border-border",
									isCreating && "opacity-50 cursor-not-allowed",
								)}
							>
								<CloudOff
									className={cn(
										"h-6 w-6",
										isOffline ? "text-primary" : "text-muted-foreground",
									)}
								/>
								<div className="text-center">
									<p
										className={cn(
											"text-sm font-medium",
											isOffline && "text-primary",
										)}
									>
										Offline
									</p>
									<p className="text-[10px] text-muted-foreground">
										Local only, private
									</p>
								</div>
							</button>

							<button
								type="button"
								onClick={() => setIsOffline(false)}
								disabled={isCreating}
								className={cn(
									"flex flex-col items-center gap-2 p-4 rounded-xl border-2 transition-all",
									!isOffline
										? "border-primary bg-primary/5"
										: "border-border/40 hover:border-border",
									isCreating && "opacity-50 cursor-not-allowed",
								)}
							>
								<Cloud
									className={cn(
										"h-6 w-6",
										!isOffline ? "text-primary" : "text-muted-foreground",
									)}
								/>
								<div className="text-center">
									<p
										className={cn(
											"text-sm font-medium",
											!isOffline && "text-primary",
										)}
									>
										Online
									</p>
									<p className="text-[10px] text-muted-foreground">
										Sync & collaborate
									</p>
								</div>
							</button>
						</div>
					</div>

					<AnimatePresence>
						{error && (
							<motion.p
								initial={{ opacity: 0, y: -5 }}
								animate={{ opacity: 1, y: 0 }}
								exit={{ opacity: 0, y: -5 }}
								className="text-xs text-destructive"
							>
								{error}
							</motion.p>
						)}
					</AnimatePresence>

					<Button
						className="w-full h-11 gap-2"
						onClick={handleCreate}
						disabled={isCreating || !name.trim()}
					>
						{isCreating ? (
							<>
								<Loader2 className="h-4 w-4 animate-spin" />
								Creating...
							</>
						) : (
							<>
								<Rocket className="h-4 w-4" />
								Create & Open Flow
							</>
						)}
					</Button>
				</motion.div>

				<div className="flex items-center gap-2 text-[10px] text-muted-foreground/60 justify-center">
					<Sparkles className="h-3 w-3" />
					<span>Project will open directly in the flow editor</span>
				</div>
			</div>
		</div>
	);
}
