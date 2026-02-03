"use client";

import { invoke } from "@tauri-apps/api/core";
import {
	type ISettingsProfile,
	IThemes,
	useBackend,
	useInvalidateInvoke,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { ProfileSettingsPage } from "@tm9657/flow-like-ui/components/settings/profile/profile-settings-page";
import { useDebounce } from "@uidotdev/usehooks";
import { useCallback, useEffect, useMemo, useState } from "react";
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

const THEME_TRANSLATION: Record<IThemes, unknown> = {
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

export default function SettingsProfilesPage() {
	const backend = useBackend();
	const invalidate = useInvalidateInvoke();
	const profiles = useTauriInvoke<Record<string, ISettingsProfile>>("get_profiles", {});

	const currentProfile = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
	);

	const [localProfile, setLocalProfile] = useState<ISettingsProfile | null>(
		null,
	);
	const debouncedLocalProfile = useDebounce(localProfile, 500);
	const [hasChanges, setHasChanges] = useState(false);

	useEffect(() => {
		if (currentProfile.data) {
			setLocalProfile(currentProfile.data);
			setHasChanges(false);
		}
	}, [currentProfile.data]);

	const isCustomTheme = useMemo(() => {
		const id = localProfile?.hub_profile?.theme?.id;
		return !!id && !Object.values(IThemes).includes(id as IThemes);
	}, [localProfile]);

	useEffect(() => {
		if (debouncedLocalProfile) {
			upsertProfile(debouncedLocalProfile);
			setHasChanges(false);
		}
	}, [debouncedLocalProfile]);

	const updateProfile = useCallback(
		(updates: Partial<ISettingsProfile>) => {
			if (!localProfile) return;
			const newProfile = { ...localProfile, ...updates };
			setLocalProfile(newProfile);
			setHasChanges(true);
		},
		[localProfile],
	);

	const upsertProfile = useCallback(
		async (profile: ISettingsProfile) => {
			await invoke("upsert_profile", { profile });
			await profiles.refetch();
			await invalidate(backend.userState.getProfile, []);
			await currentProfile.refetch();
		},
		[backend.userState.getProfile, currentProfile, invalidate, profiles],
	);

	const handleProfileImageChange = useCallback(async () => {
		if (!localProfile) return;
		await invoke("change_profile_image", { profile: localProfile });
		await profiles.refetch();
		await invalidate(backend.userState.getProfile, []);
		await currentProfile.refetch();
	}, [
		localProfile,
		profiles,
		invalidate,
		backend.userState.getProfile,
		currentProfile,
	]);

	if (!localProfile) {
		return (
			<main className="flex flex-col items-center justify-center w-full flex-1 min-h-0 py-12">
				<div className="animate-spin rounded-full h-32 w-32 border-b-2 border-primary" />
			</main>
		);
	}

	return (
		<ProfileSettingsPage
			profile={localProfile}
			isCustomTheme={isCustomTheme}
			hasChanges={hasChanges}
			themeTranslation={THEME_TRANSLATION}
			onProfileUpdate={updateProfile}
			onProfileImageChange={handleProfileImageChange}
		/>
	);
}
