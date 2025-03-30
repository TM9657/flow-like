"use client";

import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
	Button,
	type IApp,
	Input,
	Label,
	Separator,
	Textarea,
	VerificationDialog,
	useBackend,
	useInvalidateInvoke,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { useTauriInvoke } from "../../../components/useInvoke";

export default function Id() {
	const backend = useBackend();
	const invalidate = useInvalidateInvoke();
	const searchParams = useSearchParams();
	const queryClient = useQueryClient();
	const router = useRouter();
	const id = searchParams.get("id");
	const app = useInvoke(backend.getApp, [id ?? ""], typeof id === "string");
	const isReady = useTauriInvoke<boolean>(
		"app_configured",
		{ appId: id },
		[id ?? ""],
		typeof id === "string",
	);
	const appSize = useTauriInvoke<number>(
		"get_app_size",
		{ appId: id },
		[id ?? ""],
		typeof id === "string",
	);
	const [localTags, setLocalTags] = useState({
		loaded: false,
		value: "",
	});

	useEffect(() => {
		if (!app.data) return;
		if (localTags.loaded) {
			updateApp({
				...app.data,
				meta: {
					...app.data.meta,
					en: {
						...app.data.meta.en,
						tags: localTags.value
							.split(",")
							.map((tag) => tag.trim().toLowerCase()),
					},
				},
			});
			return;
		}

		setLocalTags({
			loaded: true,
			value: app.data.meta.en.tags.join(", "),
		});
	}, [app.data, localTags]);

	async function updateApp(changedApp: IApp) {
		await invoke("update_app", { app: changedApp });
		await app.refetch();
		await isReady.refetch();
		await appSize.refetch();
		await invalidate(backend.getApps, []);
	}

	async function deleteVault() {
		await invoke("delete_app", { appId: id });
		await invalidate(backend.getApps, []);
		router.push("/store/yours");
	}

	return (
		<main className="justify-start flex flex-col items-start w-full flex-1 flex-grow gap-4">
			<div className="grid w-full max-w-sm items-center gap-1.5">
				<Label htmlFor="name">Name</Label>
				<Input
					id="name"
					placeholder="Name"
					value={app.data?.meta.en.name}
					onChange={(e) => {
						if (app.data)
							updateApp({
								...app.data,
								meta: {
									...app.data.meta,
									en: { ...app.data.meta.en, name: e.target.value },
								},
							});
					}}
				/>
			</div>
			<div className="grid w-full max-w-sm items-center gap-1.5">
				<Label htmlFor="description">Description</Label>
				<Textarea
					cols={5}
					id="description"
					placeholder="Description"
					value={app.data?.meta.en.description}
					onChange={(e) => {
						if (app.data)
							updateApp({
								...app.data,
								meta: {
									...app.data.meta,
									en: { ...app.data.meta.en, description: e.target.value },
								},
							});
					}}
				/>
			</div>
			<div className="grid w-full max-w-sm items-center gap-1.5">
				<Label htmlFor="tags">Tags</Label>
				<Input
					id="tags"
					placeholder="Tags"
					value={localTags.value}
					onChange={(e) => {
						setLocalTags({ loaded: true, value: e.target.value });
					}}
				/>
			</div>
			<br />
			<Separator className="w-full my-4" />
			<h3>Danger Zone</h3>
			<VerificationDialog
				dialog="You cannot undo this action. This will remove the Vault from your System!"
				onConfirm={async () => {
					await deleteVault();
				}}
			>
				<Button>Delete App</Button>
			</VerificationDialog>
		</main>
	);
}
