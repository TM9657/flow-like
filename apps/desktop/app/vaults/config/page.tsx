"use client";

import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
	Button,
	Input,
	IVault,
	Label,
	Separator,
	Textarea,
	useInvoke,
	VerificationDialog,
} from "@tm9657/flow-like-ui";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

export default function Id() {
	const searchParams = useSearchParams();
	const queryClient = useQueryClient();
	const router = useRouter();
	const id = searchParams.get("id");
	const vault = useInvoke<IVault | undefined>(
		"get_vault",
		{ vaultId: id },
		[id ?? ""],
		typeof id === "string",
	);
	const isReady = useInvoke<boolean>(
		"vault_configured",
		{ vaultId: id },
		[id ?? ""],
		typeof id === "string",
	);
	const vaultSize = useInvoke<number>(
		"get_vault_size",
		{ vaultId: id },
		[id ?? ""],
		typeof id === "string",
	);
	const [localTags, setLocalTags] = useState({
		loaded: false,
		value: "",
	});

	useEffect(() => {
		if (!vault.data) return;
		if (localTags.loaded) {
			updateVault({
				...vault.data,
				tags: localTags.value.split(",").map((tag) => tag.trim().toLowerCase()),
			});
			return;
		}

		setLocalTags({
			loaded: true,
			value: vault.data.tags.join(", "),
		});
	}, [vault.data, localTags]);

	async function updateVault(changedVault: IVault) {
		await invoke("update_vault", { vault: changedVault });
		await vault.refetch();
		await isReady.refetch();
		await vaultSize.refetch();
		await queryClient.invalidateQueries({
			queryKey: "get_vaults".split("_"),
		});
	}

	async function deleteVault() {
		await invoke("delete_vault", { vaultId: id });
		await queryClient.invalidateQueries({
			queryKey: "get_vaults".split("_"),
		});
		router.push("/vaults");
	}

	return (
		<main className="justify-start flex flex-col items-start w-full flex-1 flex-grow gap-4">
			<div className="grid w-full max-w-sm items-center gap-1.5">
				<Label htmlFor="name">Name</Label>
				<Input
					id="name"
					placeholder="Name"
					value={vault.data?.name}
					onChange={(e) => {
						if (vault.data)
							updateVault({ ...vault.data, name: e.target.value });
					}}
				/>
			</div>
			<div className="grid w-full max-w-sm items-center gap-1.5">
				<Label htmlFor="description">Description</Label>
				<Textarea
					cols={5}
					id="description"
					placeholder="Description"
					value={vault.data?.description}
					onChange={(e) => {
						if (vault.data)
							updateVault({ ...vault.data, description: e.target.value });
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
			<div className="grid w-full max-w-sm items-center gap-1.5">
				<Label htmlFor="author">Author</Label>
				<Input
					id="author"
					placeholder="Author"
					value={vault.data?.author}
					onChange={(e) => {
						if (vault.data)
							updateVault({ ...vault.data, author: e.target.value });
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
				<Button>Delete Vault</Button>
			</VerificationDialog>
		</main>
	);
}
