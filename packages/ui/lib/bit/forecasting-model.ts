import { type IBit, IBitTypes } from "../schema/bit/bit";
import {
	type IForecastingAssetRef,
	type IForecastingModelParameters,
	IForecastingModelType,
} from "../schema/bit/bit/forecasting-parameters";

export const FORECASTING_MODELS = [
	{ value: IForecastingModelType.Timestral, label: "Timestral" },
	{ value: IForecastingModelType.Chronos2, label: "Chronos 2" },
	{ value: IForecastingModelType.Toto2, label: "Toto 2" },
	{ value: IForecastingModelType.Timesfm25, label: "TimesFM 2.5" },
	{
		value: IForecastingModelType.TinyTimeMixer,
		label: "IBM Granite TinyTimeMixer",
	},
] as const;

export interface ForecastingAssetDraft {
	key: string;
	relativePath: string;
	bit: IBit;
}

export function createForecastingParameters(): IForecastingModelParameters {
	return {
		model_type: IForecastingModelType.Timestral,
		provider: { provider_name: "local:forecast" },
		assets: [],
	};
}

export function forecastingAssetPathError(path: string): string | null {
	if (
		!path ||
		path.length > 255 ||
		!/^[a-zA-Z0-9_.-]+$/.test(path) ||
		path === "." ||
		path === ".."
	)
		return "Use a file name in the checkpoint folder, without folders or parent paths.";
	if (
		path !== "config.json" &&
		path !== "LICENSE" &&
		path !== "NOTICE" &&
		path !== "model.safetensors.index.json" &&
		!path.endsWith(".safetensors")
	)
		return "Checkpoint assets must be config.json, LICENSE, NOTICE, or safetensors shard files.";
	return null;
}

export function forecastingAssetReferenceError(
	reference: string,
): string | null {
	const separator = reference.lastIndexOf(":");
	const id = separator < 0 ? reference : reference.slice(separator + 1);
	if (!/^[a-zA-Z0-9_.-]{1,128}$/.test(id) || id === "." || id === "..")
		return "Invalid asset Bit ID.";
	if (separator >= 0) {
		const hub = reference.slice(0, separator);
		try {
			const url = new URL(hub.startsWith("https://") ? hub : `https://${hub}`);
			if (
				hub.trim() !== hub ||
				url.protocol !== "https:" ||
				!url.hostname ||
				url.username ||
				url.password ||
				url.search ||
				url.hash ||
				url.pathname !== "/"
			)
				return "Asset Bit hubs must be HTTPS origins.";
		} catch {
			return "Asset Bit hubs must be HTTPS origins.";
		}
	}
	return null;
}

function record(value: unknown): Record<string, unknown> {
	return value !== null && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}

function hubOrigin(hub: string): string {
	return hub
		? new URL(hub.startsWith("https://") ? hub : `https://${hub}`).origin
		: "";
}

function referenceParts(reference: string, defaultHub: string) {
	const separator = reference.lastIndexOf(":");
	const id = separator < 0 ? reference : reference.slice(separator + 1);
	const hub = separator < 0 ? defaultHub : reference.slice(0, separator);
	return { id, hub, key: `${hubOrigin(hub)}:${id}` };
}

/** Match the profile activation and hub identity enforced by forecast execution. */
export function selectProfileForecastingModels(
	bits: readonly IBit[] | undefined,
	references: readonly string[] | undefined,
	defaultHub: string | undefined,
): IBit[] {
	const activated = new Map<string, Set<string | null>>();
	for (const reference of references ?? []) {
		const separator = reference.lastIndexOf(":");
		const id = separator < 0 ? reference : reference.slice(separator + 1);
		const hub =
			separator < 0 ? (defaultHub ?? "") : reference.slice(0, separator);
		const keys = activated.get(id) ?? new Set<string | null>();
		keys.add(
			forecastingAssetReferenceError(`${hub}:${id}`)
				? null
				: referenceParts(reference, defaultHub ?? "").key,
		);
		activated.set(id, keys);
	}
	const seen = new Set<string>();
	return (bits ?? []).filter((bit) => {
		if (
			bit.type !== IBitTypes.Forecasting ||
			forecastingAssetReferenceError(`${bit.hub}:${bit.id}`)
		)
			return false;
		const key = `${hubOrigin(bit.hub)}:${bit.id}`;
		const keys = activated.get(bit.id);
		if (keys?.size !== 1 || !keys.has(key) || seen.has(key)) return false;
		seen.add(key);
		return true;
	});
}

export function validateForecastingParameters(value: unknown): string | null {
	const params = record(value);
	if (!FORECASTING_MODELS.some(({ value }) => value === params.model_type))
		return "Choose a supported forecasting model family.";
	if (params.provider !== undefined) {
		const provider = record(params.provider);
		if (provider.provider_name !== "local:forecast")
			return "Forecasting models use the local:forecast runtime.";
		if (
			provider.params != null &&
			(typeof provider.params !== "object" ||
				Array.isArray(provider.params) ||
				Object.keys(provider.params).length > 0)
		)
			return "Forecasting models do not use provider API settings or credentials.";
		if (provider.api_surface != null)
			return "Forecasting models do not use a chat API surface.";
	}
	if (
		!Array.isArray(params.assets) ||
		params.assets.length === 0 ||
		params.assets.length > 512
	)
		return "Add config.json as a required checkpoint asset.";
	const paths = new Set<string>();
	const refs = new Set<string>();
	for (const raw of params.assets) {
		const asset = record(raw);
		if (
			typeof asset.bit !== "string" ||
			forecastingAssetReferenceError(asset.bit)
		)
			return "Each checkpoint asset needs a published Bit reference.";
		if (typeof asset.relative_path !== "string")
			return "Each checkpoint asset needs a file name.";
		const pathError = forecastingAssetPathError(asset.relative_path);
		if (pathError) return pathError;
		if (paths.has(asset.relative_path) || refs.has(asset.bit.trim()))
			return "Each checkpoint asset needs a unique file name and Bit reference.";
		paths.add(asset.relative_path);
		refs.add(asset.bit.trim());
	}
	if (!paths.has("config.json"))
		return "Add config.json as a required checkpoint asset.";
	return null;
}

function downloadUrlError(
	value: unknown,
	pinnedHuggingFace: boolean,
): string | null {
	try {
		if (typeof value !== "string") throw new Error();
		const url = new URL(value);
		if (url.protocol !== "https:" || url.username || url.password || url.hash)
			throw new Error();
		if (value.trim() !== value) throw new Error();
		if (pinnedHuggingFace) {
			const parts = url.pathname.split("/").slice(1).map(decodeURIComponent);
			if (
				url.hostname !== "huggingface.co" ||
				url.port ||
				(url.search && url.search !== "?download=true") ||
				parts.length < 5 ||
				![parts[0], parts[1]].every((part) => /^[\w.-]+$/.test(part)) ||
				parts[2] !== "resolve" ||
				!/^[a-f0-9]{40,64}$/i.test(parts[3]) ||
				!parts.every(
					(part) =>
						part &&
						part !== "." &&
						part !== ".." &&
						!/[\\/]/.test(part) &&
						[...part].every((character) => {
							const code = character.charCodeAt(0);
							return code >= 32 && (code < 127 || code > 159);
						}),
				)
			)
				return "Use a public Hugging Face download URL pinned to a full commit SHA for the primary checkpoint file.";
		}
		return null;
	} catch {
		return "Provide an HTTPS download URL without credentials or a fragment.";
	}
}

export function validateForecastingBit(
	bit: IBit,
	scope: "admin" | "custom" = "admin",
): string | null {
	const error = validateForecastingParameters(bit.parameters);
	if (error) return error;
	if (
		!["model.safetensors", "model.safetensors.index.json"].includes(
			bit.file_name ?? "",
		)
	)
		return "The primary file must be model.safetensors or model.safetensors.index.json.";
	const urlError = downloadUrlError(bit.download_link, scope === "custom");
	if (urlError) return urlError;
	if (
		scope === "custom" &&
		(!Number.isSafeInteger(bit.size) || (bit.size ?? 0) <= 0)
	)
		return "Enter the primary checkpoint file size in bytes.";
	if (
		scope === "custom" &&
		decodeURIComponent(
			new URL(bit.download_link as string).pathname.split("/").pop() ?? "",
		) !== bit.file_name
	)
		return "The download URL must end with the primary checkpoint file name.";
	const assets = bit.parameters.assets as IForecastingAssetRef[];
	for (const asset of assets) {
		const { id, hub } = referenceParts(asset.bit, bit.hub);
		const isRoot = id === bit.id && hubOrigin(hub) === hubOrigin(bit.hub);
		if ((asset.relative_path === bit.file_name) !== isRoot)
			return "Only the root Bit can supply the primary checkpoint file; use published file Bits for its assets.";
		if (
			bit.file_name === "model.safetensors" &&
			asset.relative_path !== "config.json" &&
			asset.relative_path !== "LICENSE" &&
			asset.relative_path !== "NOTICE" &&
			asset.relative_path !== bit.file_name
		)
			return "An unsharded checkpoint accepts model.safetensors, config.json, LICENSE, and NOTICE.";
	}
	if (
		bit.file_name === "model.safetensors.index.json" &&
		!assets.some((asset) => asset.relative_path.endsWith(".safetensors"))
	)
		return "Add every safetensors shard referenced by the checkpoint index.";
	return null;
}

export function forecastingAssetRefs(
	assets: ForecastingAssetDraft[],
): IForecastingAssetRef[] {
	return assets.map(({ bit, relativePath }) => ({
		bit: bit.hub ? `${bit.hub}:${bit.id}` : bit.id,
		relative_path: relativePath,
	}));
}

export function forecastingDependencyRequests(
	bit: IBit,
): { id: string; hub?: string }[] {
	const error = validateForecastingParameters(bit.parameters);
	if (error) throw new Error(error);
	const requests = new Map<string, { id: string; hub?: string }>();
	for (const asset of bit.parameters.assets as IForecastingAssetRef[]) {
		const { id, hub, key } = referenceParts(asset.bit, bit.hub);
		if (id === bit.id && hubOrigin(hub) === hubOrigin(bit.hub)) continue;
		if (!requests.has(key)) requests.set(key, { id, hub: hub || undefined });
	}
	return [...requests.values()];
}

export function buildForecastingRootBit(
	bit: IBit,
	assets: ForecastingAssetDraft[],
): IBit {
	const refs = forecastingAssetRefs(assets);
	return {
		...bit,
		type: IBitTypes.Forecasting,
		dependencies: refs.map((asset) => asset.bit),
		dependency_tree_hash: "",
		parameters: { ...bit.parameters, assets: refs },
	};
}

export function validateForecastingAssets(
	assets: ForecastingAssetDraft[],
): string | null {
	for (const asset of assets) {
		const error = downloadUrlError(asset.bit.download_link, false);
		if (error) return `${asset.relativePath || "Checkpoint asset"}: ${error}`;
	}
	return validateForecastingParameters({
		...createForecastingParameters(),
		assets: forecastingAssetRefs(assets),
	});
}

export async function registerForecastingAssets({
	assets,
	completed,
	uploadBit,
	onRegistered,
	onProgress,
}: {
	assets: ForecastingAssetDraft[];
	completed: Map<string, string>;
	uploadBit: (bit: IBit) => Promise<IBit>;
	onRegistered: (asset: ForecastingAssetDraft) => void;
	onProgress: (asset: ForecastingAssetDraft) => void;
}): Promise<ForecastingAssetDraft[]> {
	const error = validateForecastingAssets(assets);
	if (error) throw new Error(error);
	const registered: ForecastingAssetDraft[] = [];
	for (const asset of assets) {
		if (completed.get(asset.key) === JSON.stringify(asset.bit)) {
			registered.push(asset);
			continue;
		}
		onProgress(asset);
		const bit = await uploadBit({
			...asset.bit,
			type: IBitTypes.File,
			file_name: asset.relativePath,
		});
		const next = { ...asset, bit };
		completed.set(asset.key, JSON.stringify(bit));
		registered.push(next);
		onRegistered(next);
	}
	return registered;
}
