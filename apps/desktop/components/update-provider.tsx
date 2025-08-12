"use client";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { useEffect } from "react";

export function UpdateProvider() {
	async function update() {
		const skippedForNow = sessionStorage.getItem("skippedForNow");
		if (skippedForNow) {
			return;
		}

		try {
			const updateAvailable = await check();
			if (!updateAvailable) {
				return;
			}

			const shouldUpdate = await confirm(
				"An update is available. Would you like to update now?",
			);
			if (!shouldUpdate) {
				sessionStorage.setItem("skippedForNow", "true");
				return;
			}

			await invoke("update");
		} catch (e) {
			console.error("Error checking for updates:", e);
			return;
		}
	}

	useEffect(() => {
		update();
	}, []);

	return null;
}
