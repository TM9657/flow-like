"use client";

import { createId } from "@paralleldrive/cuid2";
import { useState } from "react";
import { toast } from "sonner";
import { useInvalidateInvoke } from "../../../hooks";
import {
	createForecastingParameters,
	validateForecastingBit,
} from "../../../lib/bit/forecasting-model";
import { type IBit, IBitTypes } from "../../../lib/schema/bit/bit";
import { useBackend } from "../../../state/backend-state";
import { EditorField } from "../../bits/bit-editor-fields";
import { emptyMetadata } from "../../bits/bit-editor-model";
import { ForecastingConfiguration } from "../../bits/forecasting-configuration";
import {
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../ui";

export function AddForecastingModelDialog({
	open,
	onOpenChange,
	onBack,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onBack: () => void;
}) {
	const backend = useBackend();
	const invalidate = useInvalidateInvoke();
	const [bit, setBit] = useState<IBit>(() => ({
		id: createId(),
		type: IBitTypes.Forecasting,
		hub: "",
		hash: "",
		dependencies: [],
		dependency_tree_hash: "",
		authors: [],
		created: new Date().toISOString(),
		updated: new Date().toISOString(),
		file_name: "model.safetensors",
		download_link: "",
		size: 0,
		meta: { en: { ...emptyMetadata(), name: "" } },
		parameters: createForecastingParameters(),
	}));
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState<string | null>(null);
	async function save() {
		if (busy) return;
		const validation = !bit.meta.en.name.trim()
			? "Enter a display name."
			: validateForecastingBit(bit, "custom");
		if (validation) {
			setError(validation);
			return;
		}
		setBusy(true);
		setError(null);
		try {
			const candidate = {
				...bit,
				dependencies: bit.parameters.assets.map((asset: { bit: string }) =>
					asset.bit.trim(),
				),
			};
			const saved = await backend.bitState.upsertCustomBit(candidate);
			try {
				await backend.bitState.addBit(
					saved,
					await backend.userState.getSettingsProfile(),
				);
			} catch {
				toast.error(
					"Model saved. Add it to your profile from the model catalog.",
				);
			}
			await Promise.all([
				invalidate(backend.bitState.listCustomBits, []),
				invalidate(backend.bitState.getProfileBits, []),
				invalidate(backend.userState.getSettingsProfile, []),
				invalidate(backend.userState.getProfile, []),
			]);
			toast.success("Forecasting model saved");
			onOpenChange(false);
		} catch (error) {
			setError(
				error instanceof Error
					? error.message
					: "Could not save the forecasting model.",
			);
		} finally {
			setBusy(false);
		}
	}
	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				if (!busy) onOpenChange(next);
			}}
		>
			<DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
				<DialogHeader>
					<DialogTitle>Add forecasting model</DialogTitle>
					<DialogDescription>
						Use a public Hugging Face checkpoint with published config and shard
						Bits. The forecasting nodes run this model on the workflow runtime.
					</DialogDescription>
				</DialogHeader>
				<fieldset disabled={busy} className="space-y-4">
					<EditorField
						label="Display name"
						value={bit.meta.en.name}
						onChange={(name) =>
							setBit((current) => ({
								...current,
								meta: { ...current.meta, en: { ...current.meta.en, name } },
							}))
						}
					/>
					<ForecastingConfiguration
						parameters={bit.parameters}
						onChange={(parameters) =>
							setBit((current) => ({ ...current, parameters }))
						}
					/>
					<EditorField
						label="Primary checkpoint download URL"
						value={bit.download_link ?? ""}
						placeholder="https://huggingface.co/owner/model/resolve/commit/model.safetensors"
						hint="Pin the download to a full commit SHA from the same revision as every asset."
						onChange={(download_link) =>
							setBit((current) => ({ ...current, download_link }))
						}
					/>
					<EditorField
						label="Primary file name"
						value={bit.file_name ?? ""}
						hint="model.safetensors or model.safetensors.index.json"
						onChange={(file_name) =>
							setBit((current) => ({ ...current, file_name }))
						}
					/>
					<EditorField
						label="Primary file size (bytes)"
						type="number"
						value={bit.size ?? 0}
						onChange={(value) =>
							setBit((current) => ({ ...current, size: Number(value) }))
						}
					/>
				</fieldset>
				{error && (
					<p role="alert" className="text-sm text-destructive">
						{error}
					</p>
				)}
				<DialogFooter>
					<Button variant="ghost" disabled={busy} onClick={onBack}>
						Back
					</Button>
					<Button disabled={busy} onClick={save}>
						{busy ? "Saving…" : "Save forecasting model"}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
