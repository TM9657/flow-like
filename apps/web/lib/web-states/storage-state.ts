import type { IStorageItem, IStorageState } from "@tm9657/flow-like-ui";
import type { IStorageItemActionResult } from "@tm9657/flow-like-ui/state/backend-state/types";
import { apiDelete, apiPost, apiPut, type WebBackendRef } from "./api-utils";

export class WebStorageState implements IStorageState {
	constructor(private readonly backend: WebBackendRef) {}

	async listStorageItems(appId: string, prefix: string): Promise<IStorageItem[]> {
		try {
			return await apiPost<IStorageItem[]>(
				`apps/${appId}/data/list`,
				{ prefix },
				this.backend.auth,
			);
		} catch {
			return [];
		}
	}

	async deleteStorageItems(appId: string, prefixes: string[]): Promise<void> {
		await apiDelete(
			`apps/${appId}/data`,
			this.backend.auth,
			{ prefixes },
		);
	}

	async downloadStorageItems(
		appId: string,
		prefixes: string[],
	): Promise<IStorageItemActionResult[]> {
		try {
			return await apiPost<IStorageItemActionResult[]>(
				`apps/${appId}/data/download`,
				{ prefixes },
				this.backend.auth,
			);
		} catch {
			return prefixes.map((prefix) => ({ prefix, error: "Download failed" }));
		}
	}

	async uploadStorageItems(
		appId: string,
		prefix: string,
		files: File[],
		onProgress?: (progress: number) => void,
	): Promise<void> {
		const totalFiles = files.length;
		let completedFiles = 0;

		const buildFilePath = (file: File): string => {
			const path = (file.webkitRelativePath ?? "") === ""
				? file.name
				: file.webkitRelativePath;
			// Avoid leading slash when prefix is empty
			return prefix ? `${prefix}/${path}` : path;
		};

		// Build file path lookup
		const fileLookup = new Map(
			files.map((file) => [buildFilePath(file), file]),
		);

		// Get signed URLs for all files
		const signedUrls = await apiPut<IStorageItemActionResult[]>(
			`apps/${appId}/data`,
			{ prefixes: files.map(buildFilePath) },
			this.backend.auth,
		);

		// Upload each file to its signed URL
		for (const urlInfo of signedUrls) {
			const signedUrl = urlInfo.url;
			if (urlInfo.error || !signedUrl) {
				console.warn(`Failed to get signed URL for ${urlInfo.prefix}: ${urlInfo.error}`);
				completedFiles++;
				continue;
			}

			const file = fileLookup.get(urlInfo.prefix);
			if (!file) {
				console.warn(`File not found for prefix: ${urlInfo.prefix}`);
				completedFiles++;
				continue;
			}

			await new Promise<void>((resolve, reject) => {
				const xhr = new XMLHttpRequest();

				xhr.upload.addEventListener("progress", (event) => {
					if (event.lengthComputable) {
						const fileProgress = event.loaded / event.total;
						const totalProgress = ((completedFiles + fileProgress) / totalFiles) * 100;
						onProgress?.(totalProgress);
					}
				});

				xhr.addEventListener("load", () => {
					if (xhr.status >= 200 && xhr.status < 300) {
						completedFiles++;
						resolve();
					} else {
						reject(new Error(`Upload failed with status ${xhr.status}: ${xhr.statusText}`));
					}
				});

				xhr.addEventListener("error", () => {
					// Network error - could be CORS, connection refused, etc.
					reject(new Error(`Upload failed: Network error (possible CORS issue)`));
				});

				xhr.open("PUT", signedUrl);

				// Set Content-Type header - required for cloud storage providers
				xhr.setRequestHeader("Content-Type", file.type || "application/octet-stream");

				// Azure Blob Storage requires x-ms-blob-type header
				// This header is ignored by other providers (S3, GCS) so it's safe to always set
				if (signedUrl.includes(".blob.core.windows.net")) {
					xhr.setRequestHeader("x-ms-blob-type", "BlockBlob");
				}

				xhr.send(file);
			});
		}

		onProgress?.(100);
	}

	async writeStorageItems(items: IStorageItemActionResult[]): Promise<void> {
		// In web mode, items are stored directly on the server
		// This method is primarily for desktop local file writing
	}
}
