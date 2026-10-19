"use client";

import type { Dispatch, SetStateAction } from "react";
import type { ForecastingAssetDraft } from "../../lib/bit/forecasting-model";
import type { IBit } from "../../lib/schema/bit/bit";
import { Button } from "../ui/button";
import { EditorField } from "./bit-editor-fields";
import { ForecastingConfiguration } from "./forecasting-configuration";

export function ForecastingAssetsConfiguration({
	bit,
	setBit,
	assets,
	setAssets,
	createAssetBit,
	disabled,
}: {
	bit: IBit;
	setBit: Dispatch<SetStateAction<IBit>>;
	assets: ForecastingAssetDraft[];
	setAssets: Dispatch<SetStateAction<ForecastingAssetDraft[]>>;
	createAssetBit: () => IBit;
	disabled: boolean;
}) {
	return (
		<fieldset
			disabled={disabled}
			className="w-full max-w-screen-lg space-y-4 rounded-lg border p-4"
		>
			<ForecastingConfiguration
				parameters={bit.parameters}
				showAssets={false}
				onChange={(parameters) =>
					setBit((current) => ({ ...current, parameters }))
				}
			/>
			<p className="text-sm text-muted-foreground">
				Add config.json from the same checkpoint revision. For a sharded model,
				use model.safetensors.index.json as the primary file and add every shard
				named in its weight map.
			</p>
			{assets.map((asset) => (
				<div key={asset.key} className="space-y-3 rounded-lg border p-4">
					<EditorField
						label="Checkpoint asset file name"
						value={asset.relativePath}
						placeholder="config.json"
						onChange={(relativePath) =>
							setAssets((current) =>
								current.map((item) =>
									item.key === asset.key
										? {
												...item,
												relativePath,
												bit: { ...item.bit, file_name: relativePath },
											}
										: item,
								),
							)
						}
					/>
					<EditorField
						label="Checkpoint asset download URL"
						value={asset.bit.download_link ?? ""}
						placeholder="https://huggingface.co/owner/model/resolve/commit/config.json"
						onChange={(download_link) =>
							setAssets((current) =>
								current.map((item) =>
									item.key === asset.key
										? { ...item, bit: { ...item.bit, download_link } }
										: item,
								),
							)
						}
					/>
					<Button
						variant="ghost"
						size="sm"
						onClick={() =>
							setAssets((current) =>
								current.filter((item) => item.key !== asset.key),
							)
						}
					>
						Remove asset
					</Button>
				</div>
			))}
			<Button
				variant="outline"
				onClick={() => {
					const assetBit = createAssetBit();
					setAssets((current) => [
						...current,
						{
							key: assetBit.id,
							bit: assetBit,
							relativePath: current.length ? "" : "config.json",
						},
					]);
				}}
			>
				Add checkpoint asset
			</Button>
		</fieldset>
	);
}
