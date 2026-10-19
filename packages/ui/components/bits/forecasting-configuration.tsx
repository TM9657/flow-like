"use client";

import { Plus, Trash2 } from "lucide-react";
import { useId } from "react";
import {
	FORECASTING_MODELS,
	createForecastingParameters,
} from "../../lib/bit/forecasting-model";
import type { IForecastingAssetRef } from "../../lib/schema/bit/bit/forecasting-parameters";
import { Button } from "../ui/button";
import { EditorField } from "./bit-editor-fields";

export function ForecastingConfiguration({
	parameters,
	onChange,
	showAssets = true,
}: {
	parameters: Record<string, unknown>;
	onChange: (parameters: unknown) => void;
	showAssets?: boolean;
}) {
	const id = useId();
	const assets = Array.isArray(parameters.assets)
		? parameters.assets.map(
				(asset): IForecastingAssetRef => ({
					bit: typeof asset?.bit === "string" ? asset.bit : "",
					relative_path:
						typeof asset?.relative_path === "string" ? asset.relative_path : "",
				}),
			)
		: [];
	const update = (changes: Record<string, unknown>) =>
		onChange({
			...createForecastingParameters(),
			...parameters,
			...changes,
			provider: { provider_name: "local:forecast" },
		});
	return (
		<div className="space-y-4">
			<div className="space-y-2">
				<label htmlFor={id} className="text-sm font-medium">
					Forecasting model family
				</label>
				<select
					id={id}
					className="h-9 w-full rounded-md border bg-background px-3 text-sm"
					value={
						typeof parameters.model_type === "string"
							? parameters.model_type
							: ""
					}
					onChange={(event) => update({ model_type: event.target.value })}
				>
					<option value="" disabled>
						Choose a model family
					</option>
					{FORECASTING_MODELS.map((model) => (
						<option key={model.value} value={model.value}>
							{model.label}
						</option>
					))}
				</select>
				<p className="text-xs text-muted-foreground">
					Runs with the native forecasting runtime. Use this model with the
					forecasting nodes to predict future values from a table.
				</p>
			</div>
			{showAssets && (
				<div className="space-y-3">
					<p className="text-sm font-medium">Required checkpoint assets</p>
					<p className="text-xs text-muted-foreground">
						Reference published file Bits for config.json and every shard listed
						in model.safetensors.index.json. Include LICENSE or NOTICE when
						supplied with the weights. All files must come from the same
						checkpoint revision.
					</p>
					{assets.map((asset, index) => (
						<div
							// biome-ignore lint/suspicious/noArrayIndexKey: Controlled asset fields follow their list position.
							key={`${id}-${index}`}
							className="flex items-end gap-2 rounded-lg border p-3"
						>
							<div className="grid flex-1 gap-3 sm:grid-cols-2">
								<EditorField
									label={`Asset ${index + 1} file name`}
									value={asset.relative_path ?? ""}
									placeholder="config.json"
									onChange={(relative_path) =>
										update({
											assets: assets.map((item, i) =>
												i === index ? { ...item, relative_path } : item,
											),
										})
									}
								/>
								<EditorField
									label={`Asset ${index + 1} Bit reference`}
									value={asset.bit ?? ""}
									placeholder="hub:bit-id"
									onChange={(bit) =>
										update({
											assets: assets.map((item, i) =>
												i === index ? { ...item, bit } : item,
											),
										})
									}
								/>
							</div>
							<Button
								variant="ghost"
								size="icon"
								aria-label={`Remove asset ${index + 1}`}
								onClick={() =>
									update({ assets: assets.filter((_, i) => i !== index) })
								}
							>
								<Trash2 className="size-4" />
							</Button>
						</div>
					))}
					<Button
						variant="outline"
						disabled={assets.length >= 512}
						size="sm"
						onClick={() =>
							update({
								assets: [
									...assets,
									{
										bit: "",
										relative_path: assets.length ? "" : "config.json",
									},
								],
							})
						}
					>
						<Plus className="size-4" />
						Add checkpoint asset
					</Button>
				</div>
			)}
		</div>
	);
}
