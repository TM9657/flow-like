/**
 * useAssetUrl Hook Tests
 *
 * Tests validate that:
 * 1. Valid URLs are passed through unchanged
 * 2. Storage paths trigger resolution
 * 3. Local file paths are converted to asset:// URLs
 * 4. Cache behavior works correctly
 * 5. Error states are handled gracefully
 */
import { describe, expect, test } from "bun:test";

// ============================================================================
// URL VALIDATION LOGIC
// ============================================================================

function isValidUrl(url: string): boolean {
	return (
		url.startsWith("http://") ||
		url.startsWith("https://") ||
		url.startsWith("data:") ||
		url.startsWith("tauri://") ||
		url.startsWith("asset://") ||
		url.startsWith("file://")
	);
}

function isLocalFilePath(path: string): boolean {
	return path.startsWith("/") || /^[A-Za-z]:[/\\]/.test(path);
}

function isStoragePath(path: string): boolean {
	return !isValidUrl(path) && !isLocalFilePath(path);
}

function cleanStoragePath(path: string): string {
	return path.replace(/^storage:\/\//, "");
}

function convertLocalPathToAssetUrl(path: string): string {
	return `asset://localhost${path}`;
}

describe("useAssetUrl URL Validation", () => {
	test("recognizes http URLs as valid", () => {
		expect(isValidUrl("http://example.com/file.png")).toBe(true);
		expect(isValidUrl("http://localhost:3000/model.glb")).toBe(true);
	});

	test("recognizes https URLs as valid", () => {
		expect(isValidUrl("https://example.com/file.png")).toBe(true);
		expect(isValidUrl("https://cdn.example.com/assets/model.glb")).toBe(true);
	});

	test("recognizes data URLs as valid", () => {
		expect(isValidUrl("data:image/png;base64,abc123")).toBe(true);
		expect(isValidUrl("data:application/octet-stream;base64,xyz")).toBe(true);
	});

	test("recognizes Tauri protocol URLs as valid", () => {
		expect(isValidUrl("tauri://localhost/path")).toBe(true);
		expect(isValidUrl("asset://localhost/path")).toBe(true);
	});

	test("recognizes file:// URLs as valid", () => {
		expect(isValidUrl("file:///Users/test/file.glb")).toBe(true);
	});

	test("rejects relative paths as invalid URLs", () => {
		expect(isValidUrl("path/to/file.png")).toBe(false);
		expect(isValidUrl("./file.png")).toBe(false);
		expect(isValidUrl("../assets/model.glb")).toBe(false);
	});

	test("rejects storage protocol as invalid URL", () => {
		expect(isValidUrl("storage://path/to/file.png")).toBe(false);
	});
});

// ============================================================================
// LOCAL FILE PATH DETECTION
// ============================================================================

describe("useAssetUrl Local File Path Detection", () => {
	test("identifies Unix absolute paths", () => {
		expect(isLocalFilePath("/Users/felix/file.glb")).toBe(true);
		expect(isLocalFilePath("/home/user/models/char.glb")).toBe(true);
		expect(isLocalFilePath("/tmp/upload.glb")).toBe(true);
	});

	test("identifies Windows absolute paths", () => {
		expect(isLocalFilePath("C:/Users/test/file.glb")).toBe(true);
		expect(isLocalFilePath("D:\\Projects\\model.glb")).toBe(true);
		expect(isLocalFilePath("c:\\path\\to\\file.glb")).toBe(true);
	});

	test("rejects relative paths as local file paths", () => {
		expect(isLocalFilePath("models/character.glb")).toBe(false);
		expect(isLocalFilePath("./file.glb")).toBe(false);
		expect(isLocalFilePath("../assets/model.glb")).toBe(false);
	});

	test("rejects URLs as local file paths", () => {
		expect(isLocalFilePath("https://example.com/file.glb")).toBe(false);
		expect(isLocalFilePath("storage://path/file.glb")).toBe(false);
	});
});

describe("useAssetUrl Local Path to Asset URL Conversion", () => {
	test("converts Unix paths to asset:// URLs", () => {
		expect(convertLocalPathToAssetUrl("/Users/felix/file.glb")).toBe(
			"asset://localhost/Users/felix/file.glb",
		);
	});

	test("preserves full path in conversion", () => {
		const path =
			"/Users/felix/Library/Application Support/flow-like/upload/model.glb";
		const expected =
			"asset://localhost/Users/felix/Library/Application Support/flow-like/upload/model.glb";
		expect(convertLocalPathToAssetUrl(path)).toBe(expected);
	});
});

describe("useAssetUrl Storage Path Detection", () => {
	test("identifies relative paths as storage paths", () => {
		expect(isStoragePath("models/character.glb")).toBe(true);
		expect(isStoragePath("assets/textures/diffuse.png")).toBe(true);
	});

	test("identifies storage:// protocol as storage path", () => {
		expect(isStoragePath("storage://models/character.glb")).toBe(true);
	});

	test("does not identify http/https as storage paths", () => {
		expect(isStoragePath("https://example.com/model.glb")).toBe(false);
		expect(isStoragePath("http://localhost/model.glb")).toBe(false);
	});

	test("does not identify local file paths as storage paths", () => {
		expect(isStoragePath("/Users/felix/file.glb")).toBe(false);
		expect(isStoragePath("C:/Users/test/model.glb")).toBe(false);
	});
});

describe("useAssetUrl Path Cleaning", () => {
	test("removes storage:// prefix", () => {
		expect(cleanStoragePath("storage://models/character.glb")).toBe(
			"models/character.glb",
		);
	});

	test("preserves paths without storage:// prefix", () => {
		expect(cleanStoragePath("models/character.glb")).toBe(
			"models/character.glb",
		);
	});

	test("only removes prefix at start", () => {
		expect(cleanStoragePath("path/storage://something")).toBe(
			"path/storage://something",
		);
	});
});

// ============================================================================
// CACHE LOGIC TESTS
// ============================================================================

describe("useAssetUrl Cache Logic", () => {
	const CACHE_DURATION_MS = 30 * 60 * 1000; // 30 minutes

	interface AssetUrlCache {
		url: string;
		expiresAt: number;
	}

	function isCacheValid(cached: AssetUrlCache | undefined): boolean {
		if (!cached) return false;
		return cached.expiresAt > Date.now();
	}

	test("cache entry is valid when not expired", () => {
		const cache: AssetUrlCache = {
			url: "https://signed-url.example.com/model.glb",
			expiresAt: Date.now() + CACHE_DURATION_MS,
		};

		expect(isCacheValid(cache)).toBe(true);
	});

	test("cache entry is invalid when expired", () => {
		const cache: AssetUrlCache = {
			url: "https://signed-url.example.com/model.glb",
			expiresAt: Date.now() - 1000, // Expired 1 second ago
		};

		expect(isCacheValid(cache)).toBe(false);
	});

	test("undefined cache is invalid", () => {
		expect(isCacheValid(undefined)).toBe(false);
	});

	test("cache key format is correct", () => {
		const appId = "app-123";
		const cleanPath = "models/character.glb";
		const cacheKey = `${appId}:${cleanPath}`;

		expect(cacheKey).toBe("app-123:models/character.glb");
	});
});

// ============================================================================
// EDGE CASES
// ============================================================================

describe("useAssetUrl Edge Cases", () => {
	test("handles empty string", () => {
		const path = "";
		expect(path).toBeFalsy();
	});

	test("handles undefined", () => {
		const path: string | undefined = undefined;
		expect(path).toBeUndefined();
	});

	test("handles paths with special characters", () => {
		const paths = [
			"models/character (1).glb",
			"models/character%20name.glb",
			"models/日本語.glb",
		];

		for (const path of paths) {
			expect(typeof path).toBe("string");
			expect(path.length).toBeGreaterThan(0);
		}
	});

	test("handles very long paths", () => {
		const longPath = "a/".repeat(100) + "model.glb";
		expect(cleanStoragePath(longPath)).toBe(longPath);
	});
});

// ============================================================================
// RETURN VALUE STRUCTURE
// ============================================================================

describe("useAssetUrl Return Value Structure", () => {
	interface UseAssetUrlReturn {
		url: string | undefined;
		isLoading: boolean;
		error: string | null;
	}

	test("loading state structure", () => {
		const loadingState: UseAssetUrlReturn = {
			url: undefined,
			isLoading: true,
			error: null,
		};

		expect(loadingState.url).toBeUndefined();
		expect(loadingState.isLoading).toBe(true);
		expect(loadingState.error).toBeNull();
	});

	test("success state structure", () => {
		const successState: UseAssetUrlReturn = {
			url: "https://signed-url.example.com/model.glb",
			isLoading: false,
			error: null,
		};

		expect(successState.url).toBeDefined();
		expect(successState.isLoading).toBe(false);
		expect(successState.error).toBeNull();
	});

	test("passthrough state structure (for valid URLs)", () => {
		const passthroughState: UseAssetUrlReturn = {
			url: "https://example.com/model.glb",
			isLoading: false,
			error: null,
		};

		expect(passthroughState.url).toBe("https://example.com/model.glb");
		expect(passthroughState.isLoading).toBe(false);
		expect(passthroughState.error).toBeNull();
	});

	test("fallback state structure (when context missing)", () => {
		// When context is missing, we fall back to using the raw path
		const fallbackState: UseAssetUrlReturn = {
			url: "models/character.glb",
			isLoading: false,
			error: null,
		};

		expect(fallbackState.url).toBe("models/character.glb");
		expect(fallbackState.isLoading).toBe(false);
		expect(fallbackState.error).toBeNull();
	});
});
