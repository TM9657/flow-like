"use client";

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
	Badge,
	Button,
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	EmptyState,
	Input,
	Label,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Skeleton,
	Tooltip,
	TooltipContent,
	TooltipTrigger,
	PackageStatusBadge,
} from "@tm9657/flow-like-ui";
import type {
	DeveloperProject,
	DeveloperSettings,
	PackageInspection,
} from "@tm9657/flow-like-ui/lib/schema/developer";
import {
	EDITOR_OPTIONS,
	TEMPLATE_LANGUAGES,
} from "@tm9657/flow-like-ui/lib/schema/developer";
import { usePackageStatus } from "../../hooks/use-package-status";
import {
	Bug,
	Code2,
	ExternalLink,
	FolderOpen,
	Loader2,
	Package,
	Pencil,
	Plus,
	RefreshCw,
	Search,
	Settings2,
	Sparkles,
	Trash2,
	Upload,
} from "lucide-react";
import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

function useIsVisible(ref: React.RefObject<HTMLElement | null>) {
	const [isVisible, setIsVisible] = useState(false);
	useEffect(() => {
		const el = ref.current;
		if (!el) return;
		const observer = new IntersectionObserver(
			([entry]) => {
				if (entry?.isIntersecting) {
					setIsVisible(true);
					observer.disconnect();
				}
			},
			{ rootMargin: "200px" },
		);
		observer.observe(el);
		return () => observer.disconnect();
	}, [ref]);
	return isVisible;
}

function LanguageBadge({ language }: { language: string }) {
	const info = TEMPLATE_LANGUAGES.find((t) => t.value === language);
	return (
		<Badge
			variant="secondary"
			className="gap-1.5 text-[10px] whitespace-nowrap rounded-full px-2 py-0.5 bg-muted/30 border-transparent text-foreground"
		>
			{info?.img ? (
				<img src={info.img} alt={info.label} className="w-5 h-5 rounded-full object-cover" />
			) : (
				<span>{info?.icon ?? "📦"}</span>
			)}
			{info?.label ?? language}
		</Badge>
	);
}

function ProjectCard({
	project,
	onRemove,
}: {
	project: DeveloperProject;
	onRemove: (id: string) => void;
}) {
	const [opening, setOpening] = useState(false);
	const [loading, setLoading] = useState(false);
	const [inspection, setInspection] = useState<PackageInspection | null>(null);
	const [inspecting, setInspecting] = useState(false);
	const compileStatus = usePackageStatus(`dev:${project.path}`);
	const cardRef = useRef<HTMLDivElement>(null);
	const isVisible = useIsVisible(cardRef);

	useEffect(() => {
		if (!isVisible || inspection || inspecting) return;
		let cancelled = false;
		setInspecting(true);
		invoke<PackageInspection>("developer_inspect_package", {
			projectPath: project.path,
		})
			.then((result) => {
				if (!cancelled) setInspection(result);
			})
			.catch(() => {})
			.finally(() => {
				if (!cancelled) setInspecting(false);
			});
		return () => { cancelled = true; };
	}, [isVisible, project.path, inspection, inspecting]);

	const openInEditor = async () => {
		setOpening(true);
		try {
			await invoke("developer_open_in_editor", {
				projectPath: project.path,
			});
		} catch (err) {
			toast.error(`${err}`);
		} finally {
			setOpening(false);
		}
	};

	const loadIntoCatalog = async () => {
		setLoading(true);
		try {
			const count = await invoke<number>("developer_load_into_catalog", {
				projectPath: project.path,
			});
			toast.success(`Loaded ${count} node(s) into catalog`);
		} catch (err) {
			toast.error(`${err}`);
		} finally {
			setLoading(false);
		}
	};

	const nodeCount = inspection?.nodes?.length ?? 0;

	return (
		<div ref={cardRef} className="rounded-xl border border-border/40 bg-card shadow-sm hover:bg-accent/30 p-4 transition-colors duration-150">
			<div className="flex items-start justify-between gap-3">
				<div className="flex items-center gap-2.5 min-w-0 flex-1">
					<div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-muted/20">
						<Code2 className="h-4 w-4 text-muted-foreground/70" />
					</div>
					<div className="min-w-0 flex-1">
						<div className="flex items-center gap-2">
							<span className="text-sm font-medium truncate">
								{project.name}
							</span>
							{inspection?.manifest?.version && (
								<span className="text-[10px] font-mono text-muted-foreground/50">
									v{inspection.manifest.version}
								</span>
							)}
						</div>
						<p className="text-[11px] text-muted-foreground truncate mt-0.5">
							{project.path}
						</p>
					</div>
				</div>
				<div className="flex items-center gap-1.5">
					{compileStatus && compileStatus !== "idle" && (
						<PackageStatusBadge status={compileStatus} />
					)}
					<LanguageBadge language={project.language} />
				</div>
			</div>

			{nodeCount > 0 && (
				<div className="flex flex-wrap gap-1 mt-3 pt-3 border-t border-border/10">
					<span className="text-[10px] text-muted-foreground mr-1 self-center">
						{nodeCount} {nodeCount === 1 ? "node" : "nodes"}
					</span>
					{inspection?.nodes.map((node) => (
						<Tooltip key={node.name}>
							<TooltipTrigger asChild>
								<Badge
									variant="secondary"
									className="text-[10px] gap-1 cursor-default rounded-full px-2 py-0 bg-muted/20 border-transparent text-foreground"
								>
									{node.icon && <span>{node.icon}</span>}
									{node.friendly_name}
								</Badge>
							</TooltipTrigger>
							<TooltipContent side="bottom" className="max-w-xs">
								<p className="font-medium">{node.friendly_name}</p>
								{node.description && (
									<p className="text-xs text-foreground">
										{node.description}
									</p>
								)}
								<p className="text-[10px] text-foreground mt-0.5">
									{node.category}
								</p>
							</TooltipContent>
						</Tooltip>
					))}
				</div>
			)}

			<div className="flex items-center justify-end gap-0.5 mt-3 pt-3 border-t border-border/10">
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							size="icon"
							variant="ghost"
							className="h-7 w-7 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
							onClick={openInEditor}
							disabled={opening}
						>
							{opening ? (
								<Loader2 className="h-3.5 w-3.5 animate-spin" />
							) : (
								<ExternalLink className="h-3.5 w-3.5" />
							)}
						</Button>
					</TooltipTrigger>
					<TooltipContent>Open in Editor</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							size="icon"
							variant="ghost"
							className="h-7 w-7 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
							onClick={loadIntoCatalog}
							disabled={loading}
						>
							{loading ? (
								<Loader2 className="h-3.5 w-3.5 animate-spin" />
							) : (
								<Upload className="h-3.5 w-3.5" />
							)}
						</Button>
					</TooltipTrigger>
					<TooltipContent>Load into Catalog</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger asChild>
						<Link
							href={`/developer/manifest?path=${encodeURIComponent(project.path)}`}
						>
							<Button
								size="icon"
								variant="ghost"
								className="h-7 w-7 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
							>
								<Pencil className="h-3.5 w-3.5" />
							</Button>
						</Link>
					</TooltipTrigger>
					<TooltipContent>Edit Manifest</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger asChild>
						<Link
							href={`/developer/debug?project=${encodeURIComponent(project.path)}`}
						>
							<Button
								size="icon"
								variant="ghost"
								className="h-7 w-7 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
							>
								<Bug className="h-3.5 w-3.5" />
							</Button>
						</Link>
					</TooltipTrigger>
					<TooltipContent>Debug &amp; Test</TooltipContent>
				</Tooltip>

				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							size="icon"
							variant="ghost"
							className="h-7 w-7 rounded-full text-destructive hover:text-destructive hover:bg-destructive/10"
							onClick={() => onRemove(project.id)}
						>
							<Trash2 className="h-3.5 w-3.5" />
						</Button>
					</TooltipTrigger>
					<TooltipContent>Remove</TooltipContent>
				</Tooltip>
			</div>
		</div>
	);
}

function SettingsDialog() {
	const [settings, setSettings] = useState<DeveloperSettings | null>(null);
	const [saving, setSaving] = useState(false);

	useEffect(() => {
		invoke<DeveloperSettings>("developer_get_settings")
			.then(setSettings)
			.catch(console.error);
	}, []);

	const handleSave = async () => {
		if (!settings) return;
		setSaving(true);
		try {
			await invoke("developer_save_settings", { devSettings: settings });
			toast.success("Settings saved");
		} catch (err) {
			toast.error(`${err}`);
		} finally {
			setSaving(false);
		}
	};

	if (!settings) return null;

	return (
		<div className="space-y-4">
			<div className="space-y-2">
				<Label>Preferred Editor</Label>
				<Select
					value={settings.preferredEditor}
					onValueChange={(v) =>
						setSettings({ ...settings, preferredEditor: v })
					}
				>
					<SelectTrigger>
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{EDITOR_OPTIONS.map((e) => (
							<SelectItem key={e.value} value={e.value}>
								{e.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</div>
			<DialogFooter>
				<DialogClose asChild>
					<Button variant="outline">Cancel</Button>
				</DialogClose>
				<Button onClick={handleSave} disabled={saving}>
					{saving && <Loader2 className="h-4 w-4 animate-spin mr-1" />}
					Save
				</Button>
			</DialogFooter>
		</div>
	);
}

export default function DeveloperPage() {
	const [projects, setProjects] = useState<DeveloperProject[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [isAdding, setIsAdding] = useState(false);
	const [search, setSearch] = useState("");

	const fetchProjects = useCallback(async () => {
		setIsLoading(true);
		try {
			const list = await invoke<DeveloperProject[]>(
				"developer_list_projects",
			);
			setProjects(list);
		} catch (err) {
			console.error("Failed to list projects:", err);
		} finally {
			setIsLoading(false);
		}
	}, []);

	useEffect(() => {
		fetchProjects();
	}, [fetchProjects]);

	const filtered = useMemo(() => {
		if (!search.trim()) return projects;
		const q = search.toLowerCase();
		return projects.filter(
			(p) =>
				p.name.toLowerCase().includes(q) ||
				p.path.toLowerCase().includes(q) ||
				p.language.toLowerCase().includes(q),
		);
	}, [projects, search]);

	const handleAddExisting = async () => {
		try {
			const selected = await open({ directory: true, multiple: false });
			if (!selected) return;

			setIsAdding(true);

			let projectName = selected.split("/").pop() ?? "Untitled";
			try {
				const manifest = await invoke<Record<string, unknown>>(
					"developer_get_manifest",
					{ projectPath: selected },
				);
				const pkg = manifest.package as Record<string, unknown>;
				if (pkg?.name) projectName = pkg.name as string;
			} catch {
				// no manifest
			}

			let detectedLang = "rust";
			const { exists } = await import("@tauri-apps/plugin-fs");
			const detectionMap: [string, string][] = [
				["Cargo.toml", "rust"],
				["package.json", "typescript"],
				["tsconfig.json", "typescript"],
				["go.mod", "go"],
				["build.zig", "zig"],
				["moon.mod.json", "moonbit"],
				["nimble.nimble", "nim"],
				[".csproj", "csharp"],
				["build.gradle.kts", "kotlin"],
				["CMakeLists.txt", "cpp"],
				["requirements.txt", "python"],
				["pyproject.toml", "python"],
			];
			for (const [file, lang] of detectionMap) {
				try {
					if (await exists(`${selected}/${file}`)) {
						detectedLang = lang;
						break;
					}
				} catch {
					// skip
				}
			}

			await invoke("developer_add_project", {
				input: { path: selected, language: detectedLang, name: projectName },
			});
			toast.success(`Added ${projectName}`);
			await fetchProjects();
		} catch (err) {
			toast.error(`${err}`);
		} finally {
			setIsAdding(false);
		}
	};

	const handleRemove = async (id: string) => {
		try {
			await invoke("developer_remove_project", { projectId: id });
			toast.success("Project removed");
			await fetchProjects();
		} catch (err) {
			toast.error(`${err}`);
		}
	};

	return (
		<div className="flex flex-col h-full">
			<div className="flex items-center gap-4 px-1 pb-4">
				<div className="flex-1 min-w-0">
					<h2 className="text-2xl font-semibold tracking-tight">My Nodes</h2>
					<p className="text-sm text-muted-foreground/70">
						Local WASM node development projects
					</p>
				</div>

				<div className="relative">
					<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground/40" />
					<Input
						placeholder="Search projects…"
						value={search}
						onChange={(e) => setSearch(e.target.value)}
						className="w-52 h-8 pl-9 text-sm rounded-full bg-muted/30 border-transparent focus:border-border/40 focus:bg-muted/50"
					/>
				</div>

				<div className="flex items-center gap-1">
					<Dialog>
						<Tooltip>
							<TooltipTrigger asChild>
								<DialogTrigger asChild>
									<Button
										size="icon"
										variant="ghost"
										className="h-8 w-8 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
									>
										<Settings2 className="h-4 w-4" />
									</Button>
								</DialogTrigger>
							</TooltipTrigger>
							<TooltipContent>Settings</TooltipContent>
						</Tooltip>
						<DialogContent className="max-w-sm">
							<DialogHeader>
								<DialogTitle>Developer Settings</DialogTitle>
								<DialogDescription>
									Configure your development environment
								</DialogDescription>
							</DialogHeader>
							<SettingsDialog />
						</DialogContent>
					</Dialog>

					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								size="icon"
								variant="ghost"
								className="h-8 w-8 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
								onClick={handleAddExisting}
								disabled={isAdding}
							>
								{isAdding ? (
									<Loader2 className="h-4 w-4 animate-spin" />
								) : (
									<FolderOpen className="h-4 w-4" />
								)}
							</Button>
						</TooltipTrigger>
						<TooltipContent>Add Existing</TooltipContent>
					</Tooltip>

					<Tooltip>
						<TooltipTrigger asChild>
							<Link href="/developer/new">
								<Button
									size="icon"
									variant="ghost"
									className="h-8 w-8 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
								>
									<Plus className="h-4 w-4" />
								</Button>
							</Link>
						</TooltipTrigger>
						<TooltipContent>New Project</TooltipContent>
					</Tooltip>

					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								size="icon"
								variant="ghost"
								className="h-8 w-8 rounded-full text-muted-foreground/60 hover:text-foreground/80 hover:bg-muted/30"
								onClick={fetchProjects}
								disabled={isLoading}
							>
								<RefreshCw
									className={`h-4 w-4 ${isLoading ? "animate-spin" : ""}`}
								/>
							</Button>
						</TooltipTrigger>
						<TooltipContent>Refresh</TooltipContent>
					</Tooltip>
				</div>
			</div>

			<div className="border-t border-border/10" />

			<div className="flex-1 overflow-y-auto pt-4">
				{isLoading ? (
					<div className="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-3">
						{Array.from({ length: 3 }).map((_, i) => (
							<div
								key={`skel-${i}`}
								className="rounded-xl border border-border/10 bg-muted/5 p-4 space-y-3"
							>
								<div className="flex items-center gap-2.5">
									<Skeleton className="h-8 w-8 rounded-lg" />
									<div className="flex-1 space-y-1.5">
										<Skeleton className="h-4 w-28" />
										<Skeleton className="h-3 w-full" />
									</div>
									<Skeleton className="h-5 w-16 rounded-full" />
								</div>
								<div className="border-t border-border/10 pt-3">
									<Skeleton className="h-6 w-full rounded-lg" />
								</div>
							</div>
						))}
					</div>
				) : filtered.length > 0 ? (
					<div className="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-3">
						{filtered.map((p) => (
							<ProjectCard
								key={p.id}
								project={p}
								onRemove={handleRemove}
							/>
						))}
					</div>
				) : projects.length > 0 ? (
					<div className="flex flex-col items-center justify-center py-20">
						<p className="text-sm text-muted-foreground/50">
							No projects match "{search}"
						</p>
					</div>
				) : (
					<div className="flex flex-col items-center justify-center py-16">
						<EmptyState
							icons={[Code2, Sparkles, Package]}
							title="No node projects yet"
							description="Create a new node project from a template, or add an existing one from disk."
							action={[
								{
									label: "New Project",
									onClick: () => {
										window.location.href = "/developer/new";
									},
								},
								{
									label: "Add Existing",
									onClick: handleAddExisting,
								},
							]}
							className="border border-dashed border-border/30 rounded-2xl bg-muted/5"
						/>
					</div>
				)}
			</div>
		</div>
	);
}
