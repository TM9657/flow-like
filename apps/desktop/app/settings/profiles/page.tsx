"use client";

import { invoke } from "@tauri-apps/api/core";
import {
	Badge,
	IConnectionMode,
	type ISettingsProfile,
	IThemes,
	Input,
	type UseQueryResult,
	useBackend,
	useInvalidateInvoke,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { Button } from "@tm9657/flow-like-ui/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@tm9657/flow-like-ui/components/ui/card";
import { Label } from "@tm9657/flow-like-ui/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@tm9657/flow-like-ui/components/ui/select";
import { Switch } from "@tm9657/flow-like-ui/components/ui/switch";
import { Textarea } from "@tm9657/flow-like-ui/components/ui/textarea";
import {
	Calendar,
	Camera,
	Cpu,
	GitBranch,
	Save,
	Settings,
	Upload,
	User,
	X,
	Zap,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTauriInvoke } from "../../../components/useInvoke";
import AMBER_MINIMAL from "./themes/amber-minimal.json";
import AMETHYST_HAZE from "./themes/amethyst-haze.json";
import BOLD_TECH from "./themes/bold-tech.json";
import BUBBLEGUM from "./themes/bubblegum.json";
import CAFFEINE from "./themes/caffeine.json";
import CANDYLAND from "./themes/candyland.json";
import CATPPUCHIN from "./themes/catppuccin.json";
import CLAYMORPHISM from "./themes/claymorphism.json";
import CLEAN_SLATE from "./themes/clean-slate.json";
import COSMIC_NIGHT from "./themes/cosmic-night.json";
import CYBER_PUNK from "./themes/cyber-punk.json";
import DOOM from "./themes/doom.json";
import GRAPHITE from "./themes/graphite.json";
import KODAMA_GROVE from "./themes/kodama-grove.json";
import LUXURY from "./themes/luxury.json";
import MIDNIGHT_BLOOM from "./themes/midnight-bloom.json";
import MOCHA_MOUSSE from "./themes/mocha-mousse.json";
import MODERN_MINIMAL from "./themes/modern-minimal.json";

import { useDebounce } from "@uidotdev/usehooks";
import MONO from "./themes/mono.json";
import NATURE from "./themes/nature.json";
import NEO_BRUTALISM from "./themes/neo-brutalism.json";
import NORTHERN_LIGHTS from "./themes/northern-lights.json";
import NOTEBOOK from "./themes/notebook.json";
import OCEAN_BREEZE from "./themes/ocean-breeze.json";
import PASTEL_DREAMS from "./themes/pastel-dreams.json";
import PERPETUITY from "./themes/perpetuity.json";
import QUANTUM_ROSE from "./themes/quantum-rose.json";
import RETRO_ARCADE from "./themes/retro-arcade.json";
import SOFT_POP from "./themes/soft-pop.json";
import SOLAR_DUSK from "./themes/solar-dusk.json";
import STARRY_NIGHT from "./themes/starry-night.json";
import SUNSET_HORIZON from "./themes/sunset-horizon.json";
import TANGERINE from "./themes/tangerine.json";
import VINTAGE_PAPER from "./themes/vintage-paper.json";
import VIOLET_BLOOM from "./themes/violet-bloom.json";

const THEME_TRANSLATION: Record<IThemes, any> = {
	[IThemes.FLOW_LIKE]: undefined,
	[IThemes.AMBER_MINIMAL]: AMBER_MINIMAL,
	[IThemes.AMETHYST_HAZE]: AMETHYST_HAZE,
	[IThemes.BOLD_TECH]: BOLD_TECH,
	[IThemes.BUBBLEGUM]: BUBBLEGUM,
	[IThemes.CAFFEINE]: CAFFEINE,
	[IThemes.CANDYLAND]: CANDYLAND,
	[IThemes.CATPPUCCIN]: CATPPUCHIN,
	[IThemes.CLAYMORPHISM]: CLAYMORPHISM,
	[IThemes.CLEAN_SLATE]: CLEAN_SLATE,
	[IThemes.COSMIC_NIGHT]: COSMIC_NIGHT,
	[IThemes.CYBERPUNK]: CYBER_PUNK,
	[IThemes.DOOM_64]: DOOM,
	[IThemes.ELEGANT_LUXURY]: LUXURY,
	[IThemes.GRAPHITE]: GRAPHITE,
	[IThemes.KODAMA_GROVE]: KODAMA_GROVE,
	[IThemes.MIDNIGHT_BLOOM]: MIDNIGHT_BLOOM,
	[IThemes.MOCHA_MOUSSE]: MOCHA_MOUSSE,
	[IThemes.MODERN_MINIMAL]: MODERN_MINIMAL,
	[IThemes.MONO]: MONO,
	[IThemes.NATURE]: NATURE,
	[IThemes.NEO_BRUTALISM]: NEO_BRUTALISM,
	[IThemes.NORTHERN_LIGHTS]: NORTHERN_LIGHTS,
	[IThemes.NOTEBOOK]: NOTEBOOK,
	[IThemes.OCEAN_BREEZE]: OCEAN_BREEZE,
	[IThemes.PASTEL_DREAMS]: PASTEL_DREAMS,
	[IThemes.PERPETUITY]: PERPETUITY,
	[IThemes.QUANTUM_ROSE]: QUANTUM_ROSE,
	[IThemes.RETRO_ARCADE]: RETRO_ARCADE,
	[IThemes.SOLAR_DUSK]: SOLAR_DUSK,
	[IThemes.STARRY_NIGHT]: STARRY_NIGHT,
	[IThemes.SUNSET_HORIZON]: SUNSET_HORIZON,
	[IThemes.SOFT_POP]: SOFT_POP,
	[IThemes.TANGERINE]: TANGERINE,
	[IThemes.VIOLET_BLOOM]: VIOLET_BLOOM,
	[IThemes.VINTAGE_PAPER]: VINTAGE_PAPER,
};

// Parse tweakcn CSS → { id, light, dark } with camelCased keys
interface ImportedTheme {
	id: string;
	light: Record<string, string>;
	dark: Record<string, string>;
}

const toCamel = (name: string) =>
	name.replace(/^-+/, "").replace(/-([a-z0-9])/gi, (_, c) => c.toUpperCase());

const extractBlock = (input: string, selector: string) => {
	const re = new RegExp(`${selector}\\s*\\{([\\s\\S]*?)\\}`, "m");
	const m = input.match(re);
	return m?.[1] ?? "";
};

const parseVars = (block: string) => {
	const out: Record<string, string> = {};
	const re = /--([a-z0-9-]+)\s*:\s*([^;]+);/gi;
	let m: RegExpExecArray | null;
	while ((m = re.exec(block))) {
		const key = toCamel(m[1]);
		const val = m[2].trim();
		out[key] = val;
	}
	return out;
};

function parseTweakcnTheme(input: string, id = "Custom Theme"): ImportedTheme {
	const root = extractBlock(input, ":root");
	const dark = extractBlock(input, "\\.dark");
	const lightVars = parseVars(root);
	const darkVars = parseVars(dark);
	return { id, light: lightVars, dark: darkVars };
}

export default function SettingsPage() {
	const backend = useBackend();
	const invalidate = useInvalidateInvoke();
	const profiles: UseQueryResult<ISettingsProfile> = useTauriInvoke(
		"get_profiles",
		{},
	);

	const currentProfile = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
	);

	// Local state for editing
	const [localProfile, setLocalProfile] = useState<ISettingsProfile | null>(
		null,
	);
	const debouncedLocalProfile = useDebounce(localProfile, 500);
	const [hasChanges, setHasChanges] = useState(false);

	// Theme selection + custom import UI state
	const [themeSelectValue, setThemeSelectValue] = useState<string>(
		IThemes.FLOW_LIKE,
	);
	const [customCss, setCustomCss] = useState("");
	const [customThemeName, setCustomThemeName] = useState("Custom Theme");
	const [importError, setImportError] = useState<string | null>(null);

	// Initialize local state when profile loads
	useEffect(() => {
		if (currentProfile.data) {
			setLocalProfile(currentProfile.data);
			setHasChanges(false);
		}
	}, [currentProfile.data]);

	// Keep Select value in sync with actual theme id, mapping non-enum ids to CUSTOM
	useEffect(() => {
		if (!localProfile) return;
		const id = localProfile.hub_profile.theme?.id ?? IThemes.FLOW_LIKE;
		const isKnown = Object.values(IThemes).includes(id as IThemes);
		setThemeSelectValue(isKnown ? (id as string) : "CUSTOM");
	}, [localProfile]);

	const isCustomTheme = useMemo(() => {
		const id = localProfile?.hub_profile?.theme?.id;
		return !!id && !Object.values(IThemes).includes(id as IThemes);
	}, [localProfile]);

	useEffect(() => {
		if (debouncedLocalProfile) {
			upsertProfile(debouncedLocalProfile).then(() => {
				currentProfile.refetch();
			});
			setHasChanges(false);
		}
	}, [debouncedLocalProfile]);

	// Update local state and trigger debounced save
	const updateProfile = (updates: Partial<ISettingsProfile>) => {
		if (!localProfile) return;
		const newProfile = { ...localProfile, ...updates };
		setLocalProfile(newProfile);
		setHasChanges(true);
	};

	async function upsertProfile(profile: ISettingsProfile) {
		await invoke("upsert_profile", { profile });
		await profiles.refetch();
		await invalidate(backend.userState.getProfile, []);
		await currentProfile.refetch();
	}

	if (!localProfile) {
		return (
			<main className="flex min-h-dvh flex-col items-center justify-center w-full">
				<div className="animate-spin rounded-full h-32 w-32 border-b-2 border-primary"></div>
			</main>
		);
	}

	return (
		<main className="min-h-dvh bg-gradient-to-br from-background via-background to-muted/20 p-6 max-h-dvh overflow-y-auto pb-10">
			<div className="mx-auto max-w-6xl space-y-6">
				{/* Header */}
				<div className="flex items-center justify-between">
					<div className="space-y-1">
						<h1 className="text-4xl font-bold tracking-tight flex items-center gap-3">
							<User className="h-8 w-8 text-primary" />
							{localProfile.hub_profile.name || "Profile Settings"}
							{isCustomTheme && (
								<Badge variant="secondary" className="ml-2">
									Custom theme
								</Badge>
							)}
						</h1>
						<p className="text-muted-foreground">
							Manage your profile settings and preferences
						</p>
					</div>
					{hasChanges && (
						<div className="flex items-center gap-2 text-sm text-muted-foreground">
							<Save className="h-4 w-4" />
							Saving changes...
						</div>
					)}
				</div>

				<div className="grid gap-6 lg:grid-cols-3">
					{/* Profile Information */}
					<Card className="lg:col-span-2">
						<CardHeader>
							<CardTitle className="flex items-center gap-2">
								<User className="h-5 w-5" />
								Profile Information
							</CardTitle>
							<CardDescription>
								Basic information about your profile
							</CardDescription>
						</CardHeader>
						<CardContent className="space-y-6">
							<div className="flex gap-6">
								<div className="flex-shrink-0">
									<button className="relative group" onClick={async () => {
										await invoke("change_profile_image", { profile: localProfile });
										await profiles.refetch();
										await invalidate(backend.userState.getProfile, []);
										await currentProfile.refetch();
									}}>
										<img
											title={localProfile.hub_profile.icon ?? ""}
											className="rounded-lg border-2 border-border hover:border-primary transition-colors"
											width={224}
											height={224}
											src={
												localProfile.hub_profile.icon ??
												"/placeholder-thumbnail.webp"
											}
											alt="Profile thumbnail"
										/>
										<div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex items-center justify-center">
											<Camera className="h-8 w-8 text-white" />
										</div>
									</button>
								</div>
								<div className="flex-1 space-y-4">
									<div className="space-y-2">
										<Label htmlFor="name">Profile Name</Label>
										<Input
											id="name"
											placeholder="Enter profile name"
											value={localProfile.hub_profile.name}
											onChange={(e) =>
												updateProfile({
													hub_profile: {
														...localProfile.hub_profile,
														name: e.target.value,
													},
												})
											}
										/>
									</div>
									<div className="space-y-2">
										<Label htmlFor="description">Description</Label>
										<Textarea
											id="description"
											placeholder="Describe your profile..."
											value={localProfile.hub_profile.description ?? ""}
											onChange={(e) =>
												updateProfile({
													hub_profile: {
														...localProfile.hub_profile,
														description: e.target.value,
													},
												})
											}
											rows={3}
										/>
									</div>
									<div className="space-y-2">
										<Label htmlFor="hub">Current Hub</Label>
										<Input
											disabled
											id="hub"
											placeholder="Hub name or ID"
											value={localProfile.hub_profile.hub ?? ""}
										/>
									</div>
								</div>
							</div>

							{/* Tags Section */}
							<div className="space-y-2">
								<Label htmlFor="tags">Tags</Label>
								<div className="space-y-2">
									<Input
										id="tags"
										placeholder="Add tag and press Enter"
										onKeyDown={(e) => {
											if (e.key === "Enter") {
												const value = e.currentTarget.value.trim();
												if (
													value &&
													!localProfile.hub_profile.tags?.includes(value)
												) {
													updateProfile({
														hub_profile: {
															...localProfile.hub_profile,
															tags: [
																...(localProfile.hub_profile.tags ?? []),
																value,
															],
														},
													});
													e.currentTarget.value = "";
												}
											}
										}}
									/>
									<div className="flex flex-wrap gap-2">
										{localProfile.hub_profile.tags?.map((tag, index) => (
											<Badge
												key={index}
												variant="secondary"
												className="flex items-center gap-1"
											>
												{tag}
												<X
													className="h-3 w-3 cursor-pointer hover:text-destructive"
													onClick={() =>
														updateProfile({
															hub_profile: {
																...localProfile.hub_profile,
																tags: localProfile.hub_profile.tags?.filter(
																	(_, i) => i !== index,
																),
															},
														})
													}
												/>
											</Badge>
										))}
									</div>
								</div>
							</div>

							{/* Interests Section */}
							<div className="space-y-2">
								<Label htmlFor="interests">Interests</Label>
								<div className="space-y-2">
									<Input
										id="interests"
										placeholder="Add interest and press Enter"
										onKeyDown={(e) => {
											if (e.key === "Enter") {
												const value = e.currentTarget.value.trim();
												if (
													value &&
													!localProfile.hub_profile.interests?.includes(value)
												) {
													updateProfile({
														hub_profile: {
															...localProfile.hub_profile,
															interests: [
																...(localProfile.hub_profile.interests ?? []),
																value,
															],
														},
													});
													e.currentTarget.value = "";
												}
											}
										}}
									/>
									<div className="flex flex-wrap gap-2">
										{localProfile.hub_profile.interests?.map(
											(interest, index) => (
												<Badge
													key={index}
													variant="outline"
													className="flex items-center gap-1"
												>
													{interest}
													<X
														className="h-3 w-3 cursor-pointer hover:text-destructive"
														onClick={() =>
															updateProfile({
																hub_profile: {
																	...localProfile.hub_profile,
																	interests:
																		localProfile.hub_profile.interests?.filter(
																			(_, i) => i !== index,
																		),
																},
															})
														}
													/>
												</Badge>
											),
										)}
									</div>
								</div>
							</div>
						</CardContent>
					</Card>

					{/* Profile Stats */}
					<Card>
						<CardHeader>
							<CardTitle className="flex items-center gap-2">
								<Calendar className="h-5 w-5" />
								Profile Stats
							</CardTitle>
						</CardHeader>
						<CardContent className="space-y-4">
							<div className="space-y-2">
								<div className="flex justify-between text-sm">
									<span className="text-muted-foreground">Created</span>
									<span>
										{new Date(localProfile.created).toLocaleDateString()}
									</span>
								</div>
								<div className="flex justify-between text-sm">
									<span className="text-muted-foreground">Updated</span>
									<span>
										{new Date(localProfile.updated).toLocaleDateString()}
									</span>
								</div>
								<div className="flex justify-between text-sm">
									<span className="text-muted-foreground">Apps</span>
									<span className="font-medium">
										{localProfile.hub_profile.apps?.length ?? 0}
									</span>
								</div>
								<div className="flex justify-between text-sm">
									<span className="text-muted-foreground">Hubs</span>
									<span className="font-medium">
										{localProfile.hub_profile.hubs?.length ?? 0}
									</span>
								</div>
								<div className="flex justify-between text-sm">
									<span className="text-muted-foreground">Tags</span>
									<span className="font-medium">
										{localProfile.hub_profile.tags?.length ?? 0}
									</span>
								</div>
								<div className="flex justify-between text-sm">
									<span className="text-muted-foreground">Interests</span>
									<span className="font-medium">
										{localProfile.hub_profile.interests?.length ?? 0}
									</span>
								</div>
							</div>
						</CardContent>
					</Card>

					{/* Execution Settings */}
					<Card>
						<CardHeader>
							<CardTitle className="flex items-center gap-2">
								<Cpu className="h-5 w-5" />
								Execution Settings
							</CardTitle>
							<CardDescription>
								Configure performance and execution options
							</CardDescription>
						</CardHeader>
						<CardContent className="space-y-4">
							<div className="space-y-2">
								<Label htmlFor="context_size">Max Context Size</Label>
								<Input
									id="context_size"
									placeholder="8192"
									value={
										localProfile.execution_settings?.max_context_size || ""
									}
									type="number"
									onChange={(e) =>
										updateProfile({
											execution_settings: {
												...localProfile.execution_settings,
												max_context_size: Number.parseInt(e.target.value) || 0,
											},
										})
									}
								/>
							</div>
							<div className="flex items-center justify-between">
								<div className="space-y-0.5">
									<Label htmlFor="gpu" className="flex items-center gap-2">
										<Zap className="h-4 w-4" />
										GPU Mode
									</Label>
									<p className="text-sm text-muted-foreground">
										Enable GPU acceleration
									</p>
								</div>
								<Switch
									id="gpu"
									checked={localProfile.execution_settings?.gpu_mode ?? true}
									onCheckedChange={(checked) =>
										updateProfile({
											execution_settings: {
												...localProfile.execution_settings,
												gpu_mode: checked,
											},
										})
									}
								/>
							</div>
						</CardContent>
					</Card>

					{/* Theme Settings */}
					<Card>
						<CardHeader>
							<CardTitle className="flex items-center gap-2">
								<Settings className="h-5 w-5" />
								Theme Settings
								{isCustomTheme && <Badge className="ml-2">Custom</Badge>}
							</CardTitle>
							<CardDescription>
								Customize your visual experience
							</CardDescription>
						</CardHeader>
						<CardContent>
							<div className="space-y-3">
								<Label htmlFor="theme">Theme</Label>
								<Select
									value={themeSelectValue}
									onValueChange={(value: string) => {
										setThemeSelectValue(value);
										setImportError(null);
										if (Object.values(IThemes).includes(value as IThemes)) {
											updateProfile({
												hub_profile: {
													...localProfile.hub_profile,
													theme:
														THEME_TRANSLATION[value as IThemes] ??
														THEME_TRANSLATION[IThemes.FLOW_LIKE],
												},
											});
										}
									}}
								>
									<SelectTrigger>
										<SelectValue placeholder="Select theme" />
									</SelectTrigger>
									<SelectContent className="max-h-60">
										{Object.values(IThemes).map((theme) => (
											<SelectItem key={theme} value={theme}>
												{theme}
											</SelectItem>
										))}
										<SelectItem value="CUSTOM">Custom (import)</SelectItem>
									</SelectContent>
								</Select>

								{themeSelectValue === "CUSTOM" && (
									<div className="mt-3 space-y-3">
										<div className="grid gap-2">
											<Label htmlFor="customThemeName">Theme Name</Label>
											<Input
												id="customThemeName"
												placeholder="Custom Theme"
												value={customThemeName}
												onChange={(e) => setCustomThemeName(e.target.value)}
											/>
										</div>

										<div className="grid gap-2">
											<Label htmlFor="customTheme">Paste tweakcn export</Label>
											<Textarea
												id="customTheme"
												placeholder="Paste the CSS export from tweakcn here"
												rows={10}
												value={customCss}
												onChange={(e) => setCustomCss(e.target.value)}
											/>
										</div>

										<div className="flex items-center gap-2">
											<Button
												variant="default"
												onClick={() => {
													try {
														const parsed = parseTweakcnTheme(
															customCss,
															customThemeName || "Custom Theme",
														);
														if (
															!parsed.light?.background &&
															!parsed.dark?.background
														) {
															throw new Error("No valid variables found.");
														}
														updateProfile({
															hub_profile: {
																...localProfile.hub_profile,
																theme: parsed as any,
															},
														});
														setThemeSelectValue("CUSTOM");
														setImportError(null);
													} catch (err: any) {
														setImportError(
															err?.message ?? "Failed to import theme.",
														);
													}
												}}
												className="flex items-center gap-2"
											>
												<Upload className="h-4 w-4" />
												Import & Apply
											</Button>
											{importError && (
												<span className="text-sm text-destructive">
													{importError}
												</span>
											)}
										</div>
										<p className="text-xs text-muted-foreground">
											Paste the full CSS including :root and .dark blocks from
											tweakcn.com.
										</p>
									</div>
								)}

								<p className="text-xs text-muted-foreground">
									Credits to{" "}
									<a
										href="https://tweakcn.com/"
										target="_blank"
										className="underline font-bold"
										rel="noreferrer"
									>
										tweakcn.com
									</a>
								</p>
							</div>
						</CardContent>
					</Card>

					{/* Flow Settings */}
					<Card>
						<CardHeader>
							<CardTitle className="flex items-center gap-2">
								<GitBranch className="h-5 w-5" />
								Flow Settings
							</CardTitle>
							<CardDescription>
								Configure flow visualization preferences
							</CardDescription>
						</CardHeader>
						<CardContent>
							<div className="space-y-2">
								<Label htmlFor="connection_mode">Connection Mode</Label>
								<Select
									value={
										localProfile.hub_profile.settings?.connection_mode ??
										IConnectionMode.Default
									}
									onValueChange={(value: IConnectionMode) =>
										updateProfile({
											hub_profile: {
												...localProfile.hub_profile,
												settings: {
													...localProfile.hub_profile.settings,
													connection_mode: value,
												},
											},
										})
									}
								>
									<SelectTrigger>
										<SelectValue placeholder="Select connection mode" />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value={IConnectionMode.Default}>
											Default
										</SelectItem>
										<SelectItem value={IConnectionMode.Straight}>
											Straight
										</SelectItem>
										<SelectItem value={IConnectionMode.Step}>Step</SelectItem>
										<SelectItem value={IConnectionMode.Smoothstep}>
											Smooth Step
										</SelectItem>
										<SelectItem value={IConnectionMode.Simplebezier}>
											Simple Bezier
										</SelectItem>
									</SelectContent>
								</Select>
							</div>
						</CardContent>
					</Card>
				</div>
			</div>
		</main>
	);
}
