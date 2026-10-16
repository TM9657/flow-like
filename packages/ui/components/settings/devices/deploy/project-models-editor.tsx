"use client";

import { useTranslation } from "@flow-like/locales";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useRef, useState } from "react";
import { injectData } from "../../../../hooks/use-invoke";
import { getErrorMessage } from "../../../../lib/error-message";
import type { IApp } from "../../../../lib/schema/app/app";
import { type IBit, IBitTypes } from "../../../../lib/schema/bit/bit";
import { useBackend } from "../../../../state/backend-state";
import { DvButton } from "../primitives/dv-button";
import { CheckField, DvInput, Field } from "../primitives/form-fields";
import { InlineResult } from "../primitives/inline-result";

const MODEL_TYPES = [
	IBitTypes.Llm,
	IBitTypes.Vlm,
	IBitTypes.SystemOne,
	IBitTypes.Embedding,
	IBitTypes.ImageEmbedding,
	IBitTypes.ImageGeneration,
	IBitTypes.VideoGeneration,
	IBitTypes.Stt,
	IBitTypes.Tts,
	IBitTypes.ObjectDetection,
];

const nameOf = (bit: IBit) =>
	bit.meta?.en?.name || Object.values(bit.meta ?? {})[0]?.name || bit.id;
const sameBits = (left: readonly string[], right: readonly string[]) =>
	JSON.stringify([...left].sort()) === JSON.stringify([...right].sort());

interface ModelChoice {
	bit: IBit;
	references: string[];
}
const modelKey = (bit: IBit) => (bit.hub ? `${bit.hub}:${bit.id}` : bit.id);

/** Existing references keep their spelling, including a hub-qualified prefix. */
function addChoice(
	choices: Map<string, ModelChoice>,
	bit: IBit,
	reference?: string,
) {
	if (!MODEL_TYPES.includes(bit.type)) return;
	const key = modelKey(bit);
	const current = choices.get(key);
	choices.set(key, {
		bit,
		references: reference
			? [...new Set([...(current?.references ?? []), reference])]
			: (current?.references ?? []),
	});
}

/** Save only the model choices against a fresh app, keeping concurrent changes to other fields. */
export async function saveProjectModels(
	appState: Pick<
		ReturnType<typeof useBackend>["appState"],
		"getAppAuthoritative" | "updateAppAuthoritative"
	>,
	base: IApp,
	bits: readonly string[],
): Promise<IApp> {
	const fresh = await appState.getAppAuthoritative(base.id);
	if (JSON.stringify(fresh.bits) !== JSON.stringify(base.bits))
		throw new Error(
			"The app's models changed elsewhere. Cancel and reopen the editor to review them.",
		);
	const next = { ...fresh, bits: [...new Set(bits)] };
	await appState.updateAppAuthoritative(next);
	return next;
}

export function ProjectModelsEditor({
	appId,
	onSaved,
	onEditingChange,
}: Readonly<{
	appId: string;
	onSaved?(app: IApp): void;
	onEditingChange?(editing: boolean): void;
}>) {
	const { t } = useTranslation("devices");
	const backend = useBackend();
	const queryClient = useQueryClient();
	const id = useId();
	const [editing, setEditing] = useState(false);
	const [app, setApp] = useState<IApp | null>(null);
	const [models, setModels] = useState<Map<string, ModelChoice>>(new Map());
	const [selected, setSelected] = useState<string[]>([]);
	const [search, setSearch] = useState("");
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState("");
	const [saved, setSaved] = useState(false);
	const lifecycle = useRef({ editing, onEditingChange });
	lifecycle.current = { editing, onEditingChange };
	useEffect(
		() => () => {
			if (lifecycle.current.editing) lifecycle.current.onEditingChange?.(false);
		},
		[],
	);
	useEffect(() => {
		if (!editing) return;
		let active = true;
		setBusy(true);
		setError("");
		setApp(null);
		(async () => {
			const current = await backend.appState.getAppAuthoritative(appId);
			const results = await Promise.allSettled([
				Promise.resolve().then(() => backend.bitState.getProfileBits()),
				Promise.resolve().then(() =>
					backend.bitState.searchBits({ bit_types: MODEL_TYPES, limit: 100 }),
				),
				Promise.resolve().then(() => backend.bitState.listCustomBits()),
				...current.bits.map(async (reference) => {
					const split = reference.lastIndexOf(":");
					return [
						await backend.bitState.getBit(
							split < 0 ? reference : reference.slice(split + 1),
							split < 0 ? undefined : reference.slice(0, split),
						),
					];
				}),
			]);
			if (!active) return;
			const choices = new Map<string, ModelChoice>();
			for (const [index, result] of results.entries())
				if (result.status === "fulfilled")
					for (const bit of result.value)
						addChoice(
							choices,
							bit,
							index >= 3 ? current.bits[index - 3] : undefined,
						);
			if (results.slice(0, 3).every((result) => result.status === "rejected"))
				setError(
					"The model catalog could not be read. Existing model choices are shown where available.",
				);
			setApp(current);
			setSelected([...current.bits]);
			setModels(choices);
		})()
			.catch((cause) => active && setError(getErrorMessage(cause)))
			.finally(() => active && setBusy(false));
		return () => {
			active = false;
		};
	}, [editing, appId, backend]);
	useEffect(() => {
		if (!editing || !app || !search.trim()) return;
		let active = true;
		const timer = setTimeout(() => {
			backend.bitState
				.searchBits({ bit_types: MODEL_TYPES, limit: 100, search })
				.then((bits) => {
					if (!active) return;
					setModels((current) => {
						const next = new Map(current);
						for (const bit of bits) addChoice(next, bit);
						return next;
					});
				})
				.catch((cause) => active && setError(getErrorMessage(cause)));
		}, 200);
		return () => {
			active = false;
			clearTimeout(timer);
		};
	}, [editing, app, search, backend]);
	const close = () => {
		setEditing(false);
		onEditingChange?.(false);
	};
	const save = async () => {
		if (!app || busy) return;
		setBusy(true);
		setError("");
		try {
			const ordered = [
				...app.bits.filter((reference) => selected.includes(reference)),
				...selected.filter((reference) => !app.bits.includes(reference)),
			];
			const next = await saveProjectModels(backend.appState, app, ordered);
			injectData(queryClient, backend.appState.getApp, [appId], next);
			onSaved?.(next);
			setSaved(true);
			close();
		} catch (cause) {
			setError(getErrorMessage(cause));
		} finally {
			setBusy(false);
		}
	};
	const choices = [...models.entries()].filter(([key, { bit }]) => {
		return `${nameOf(bit)} ${key}`.toLowerCase().includes(search.toLowerCase());
	});
	return (
		<section
			className="flex min-w-0 flex-col gap-3 rounded-lg border border-border p-4"
			aria-label={t("deploy.models.title", "Project models")}
		>
			<h3 className="text-sm font-medium">
				{t("deploy.models.title", "Project models")}
			</h3>
			<p className="text-ui text-muted-foreground">
				{t(
					"deploy.models.scope",
					"Save model choices to this app, then prepare a new version for the selected services. Running services keep their current models until the update.",
				)}
			</p>
			<p className="text-xs text-muted-foreground">
				{t(
					"deploy.models.approval",
					"Hosted models also need approval in each service's Cloud access tab. Saving models keeps existing approvals and spending limits.",
				)}
			</p>
			{!editing ? (
				<>
					{saved ? (
						<InlineResult tone="good">
							{t(
								"deploy.models.saved",
								"Models saved. The update will use the new choices.",
							)}
						</InlineResult>
					) : null}
					<DvButton
						className="self-start"
						onClick={() => {
							setSaved(false);
							setSearch("");
							setEditing(true);
							onEditingChange?.(true);
						}}
					>
						{t("deploy.models.edit", "Edit models")}
					</DvButton>
				</>
			) : (
				<>
					<Field
						id={`${id}-search`}
						label={t("deploy.models.searchLabel", "Search models")}
					>
						<DvInput
							value={search}
							disabled={busy}
							onChange={(event) => setSearch(event.target.value)}
						/>
					</Field>
					<div className="flex max-h-72 flex-col gap-2 overflow-y-auto">
						{choices.map(([key, { bit, references }]) => (
							<CheckField
								key={key}
								id={`${id}-${key}`}
								checked={[key, ...references].some((reference) =>
									selected.includes(reference),
								)}
								disabled={busy}
								onCheckedChange={(checked) =>
									setSelected((current) =>
										checked
											? [...current, references[0] ?? key]
											: current.filter(
													(value) =>
														value !== key && !references.includes(value),
												),
									)
								}
							>
								{nameOf(bit)}
							</CheckField>
						))}
					</div>
					{busy ? (
						<p className="text-ui">
							{t("deploy.models.working", "Reading or saving models…")}
						</p>
					) : null}
					{error ? <InlineResult tone="critical">{error}</InlineResult> : null}
					<div className="flex gap-2">
						<DvButton
							disabled={busy || !app || sameBits(app.bits, selected)}
							onClick={() => void save()}
						>
							{t("deploy.models.save", "Save models")}
						</DvButton>
						<DvButton disabled={busy} onClick={close}>
							{t("deploy.models.cancel", "Cancel model changes")}
						</DvButton>
					</div>
				</>
			)}
		</section>
	);
}
