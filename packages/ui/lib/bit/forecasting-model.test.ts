import { describe, expect, test } from "bun:test";
import { type IBit, IBitTypes } from "../schema/bit/bit";
import {
	type ForecastingAssetDraft,
	buildForecastingRootBit,
	createForecastingParameters,
	forecastingAssetReferenceError,
	forecastingDependencyRequests,
	registerForecastingAssets,
	selectProfileForecastingModels,
	validateForecastingBit,
	validateForecastingParameters,
} from "./forecasting-model";

function fixture(): IBit {
	return {
		id: "forecast",
		hub: "hub.example",
		type: IBitTypes.Forecasting,
		authors: [],
		created: "now",
		updated: "now",
		hash: "",
		dependency_tree_hash: "",
		dependencies: [],
		meta: {},
		size: 100,
		file_name: "model.safetensors",
		download_link: `https://huggingface.co/owner/model/resolve/${"a".repeat(40)}/model.safetensors?download=true`,
		parameters: {
			...createForecastingParameters(),
			assets: [{ bit: "hub.example:config", relative_path: "config.json" }],
		},
	};
}

describe("forecasting Bits", () => {
	test("license and notice assets remain dependencies of unsharded checkpoints", () => {
		const bit = fixture();
		bit.parameters.assets.push(
			{ bit: "hub.example:license", relative_path: "LICENSE" },
			{ bit: "hub.example:notice", relative_path: "NOTICE" },
		);
		expect(validateForecastingBit(bit, "custom")).toBeNull();
		expect(forecastingDependencyRequests(bit)).toEqual([
			{ id: "config", hub: "hub.example" },
			{ id: "license", hub: "hub.example" },
			{ id: "notice", hub: "hub.example" },
		]);
	});
	test("profile discovery requires activation from one matching hub", () => {
		const selected = fixture();
		const inactive = { ...fixture(), id: "inactive" };
		const wrongHub = { ...fixture(), hub: "other.example" };
		const file = { ...fixture(), id: "file", type: IBitTypes.File };
		expect(
			selectProfileForecastingModels(
				[selected, inactive, wrongHub, file, { ...selected }],
				["forecast", "https://HUB.example:443:forecast", "file"],
				"hub.example",
			),
		).toEqual([selected]);
		for (const references of [
			[],
			["other.example:forecast"],
			["hub.example:forecast", "other.example:forecast"],
			["hub.example:forecast", "https://hub.example/path:forecast"],
		]) {
			expect(
				selectProfileForecastingModels([selected], references, "hub.example"),
			).toEqual([]);
		}
		expect(
			selectProfileForecastingModels([selected], ["forecast"], undefined),
		).toEqual([]);
	});

	test("equivalent HTTPS origins match the private root and deduplicate asset lookups", () => {
		const bit = fixture();
		bit.parameters.assets.push({
			bit: "https://HUB.example:443:forecast",
			relative_path: "model.safetensors",
		});
		expect(validateForecastingBit(bit, "custom")).toBeNull();
		expect(forecastingDependencyRequests(bit)).toEqual([
			{ id: "config", hub: "hub.example" },
		]);
		bit.parameters.assets = [
			{ bit: "hub.example:config", relative_path: "config.json" },
			{
				bit: "https://HUB.example:443:config",
				relative_path: "model-1.safetensors",
			},
		];
		expect(forecastingDependencyRequests(bit)).toEqual([
			{ id: "config", hub: "hub.example" },
		]);
	});
	test("dependency lookup preserves hub schemes and ports and omits the primary self reference", () => {
		const bit = fixture();
		bit.parameters.assets = [
			{ bit: "config", relative_path: "config.json" },
			{
				bit: "https://other.example:8443:shard",
				relative_path: "model-1.safetensors",
			},
			{ bit: "hub.example:forecast", relative_path: "model.safetensors" },
		];
		expect(forecastingDependencyRequests(bit)).toEqual([
			{ id: "config", hub: "hub.example" },
			{ id: "shard", hub: "https://other.example:8443" },
		]);
	});
	test("native families need checkpoint files without chat API configuration", () => {
		for (const model_type of [
			"timestral",
			"chronos2",
			"toto2",
			"timesfm2_5",
			"tiny_time_mixer",
		]) {
			const bit = fixture();
			bit.parameters.model_type = model_type;
			expect(validateForecastingBit(bit, "custom")).toBeNull();
			bit.parameters.provider = undefined;
			expect(validateForecastingBit(bit, "custom")).toBeNull();
		}
		expect(
			validateForecastingParameters(createForecastingParameters()),
		).toContain("config.json");
		const bit = fixture();
		bit.parameters.provider = { provider_name: "custom:openai" };
		expect(validateForecastingBit(bit)).toContain("local:forecast");
		bit.parameters.provider = {
			provider_name: "local:forecast",
			params: { api_key: "secret" },
		};
		expect(validateForecastingBit(bit)).toContain("credentials");
	});

	test("asset validation rejects traversal, duplicate destinations, and unsafe references", () => {
		for (const path of [
			"../config.json",
			"folder/config.json",
			"C:\\config.json",
			"config.json\u0000",
			" config.json",
			"config file.json",
		]) {
			const bit = fixture();
			bit.parameters.assets[0].relative_path = path;
			expect(validateForecastingBit(bit)).not.toBeNull();
		}
		for (const reference of [
			"../id",
			"https://hub.example/path:id",
			"https://user:pass@hub.example:id",
			"hub.example:id/part",
			" hub.example:id",
		]) {
			expect(forecastingAssetReferenceError(reference)).not.toBeNull();
		}
		for (const reference of [
			"config",
			"hub.example:config",
			"https://hub.example:8443:config",
		]) {
			expect(forecastingAssetReferenceError(reference)).toBeNull();
		}
		const bit = fixture();
		bit.parameters.assets.push({ bit: "other", relative_path: "config.json" });
		expect(validateForecastingBit(bit)).toContain("unique");
	});

	test("custom checkpoints require pinned sources, matching filenames, size, and all shard references", () => {
		const bit = fixture();
		bit.download_link = bit.download_link?.replace("a".repeat(40), "main");
		expect(validateForecastingBit(bit, "custom")).toContain("commit");
		bit.download_link = fixture().download_link;
		bit.size = 0;
		expect(validateForecastingBit(bit, "custom")).toContain("size");
		bit.size = 100;
		bit.file_name = "model.safetensors.index.json";
		expect(validateForecastingBit(bit, "custom")).toContain("end with");
		bit.download_link = bit.download_link?.replace(
			"model.safetensors?",
			"model.safetensors.index.json?",
		);
		expect(validateForecastingBit(bit, "custom")).toContain(
			"every safetensors shard",
		);
		bit.parameters.assets.push({
			bit: "hub.example:shard",
			relative_path: "model-00001-of-00001.safetensors",
		});
		expect(validateForecastingBit(bit, "custom")).toBeNull();
		bit.parameters.assets[0].bit = bit.id;
		expect(validateForecastingBit(bit, "custom")).toContain("root Bit");
	});

	test("registration resumes completed assets and rebuilds references from registered IDs", async () => {
		const root = fixture();
		root.file_name = "model.safetensors.index.json";
		const assets: ForecastingAssetDraft[] = [
			"config.json",
			"model-00001-of-00001.safetensors",
		].map((relativePath, index) => ({
			key: String(index),
			relativePath,
			bit: {
				...fixture(),
				id: `draft-${index}`,
				type: IBitTypes.File,
				file_name: relativePath,
				download_link: `https://example.com/${relativePath}`,
			},
		}));
		const completed = new Map<string, string>();
		let fail = true;
		const uploads: string[] = [];
		const uploadBit = async (bit: IBit) => {
			uploads.push(bit.id);
			if (fail && bit.id === "draft-1") throw new Error("network");
			return {
				...bit,
				hub: "published.example",
				id: bit.id.replace("draft", "registered"),
			};
		};
		const options = {
			assets,
			completed,
			uploadBit,
			onProgress: () => {},
			onRegistered: (asset: ForecastingAssetDraft) => {
				assets[Number(asset.key)] = asset;
			},
		};
		await expect(registerForecastingAssets(options)).rejects.toThrow("network");
		fail = false;
		const registered = await registerForecastingAssets(options);
		expect(uploads).toEqual(["draft-0", "draft-1", "draft-1"]);
		const built = buildForecastingRootBit(root, registered);
		expect(built.dependencies).toEqual([
			"published.example:registered-0",
			"published.example:registered-1",
		]);
		expect(
			built.parameters.assets.map((asset: { bit: string }) => asset.bit),
		).toEqual(built.dependencies);
		expect(built.file_name).toBe("model.safetensors.index.json");
		expect(built.parameters.provider.provider_name).toBe("local:forecast");
	});
});
