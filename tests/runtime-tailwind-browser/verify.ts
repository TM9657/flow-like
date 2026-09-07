import assert from "node:assert/strict";
import path from "node:path";
import { compile } from "@tailwindcss/node";
import puppeteer from "puppeteer";
import { compile as compileTailwind } from "tailwindcss";
import { runtimeTailwindTheme } from "../../packages/ui/lib/runtime-tailwind";

declare global {
	interface Window {
		runtimeTailwind: {
			observeRuntimeTailwind(root: HTMLElement): () => void;
		};
		runtimeRootCleanups: Record<string, () => void>;
	}
}

const projectRoot = path.resolve(import.meta.dir, "../..");
const runtimeModule = path.join(
	projectRoot,
	"packages/ui/lib/runtime-tailwind.ts",
);
const bundle = await Bun.build({
	entrypoints: ["runtime-tailwind-browser-entry"],
	target: "browser",
	format: "iife",
	plugins: [
		{
			name: "runtime-tailwind-browser-entry",
			setup(build) {
				build.onResolve({ filter: /^runtime-tailwind-browser-entry$/ }, () => ({
					path: "entry",
					namespace: "runtime-test",
				}));
				build.onLoad({ filter: /.*/, namespace: "runtime-test" }, () => ({
					contents: `import { observeRuntimeTailwind } from ${JSON.stringify(runtimeModule)};
window.runtimeTailwind = { observeRuntimeTailwind };
window.runtimeRootCleanups = {};`,
					loader: "ts",
					resolveDir: projectRoot,
				}));
			},
		},
	],
});
assert.ok(bundle.success, bundle.logs.join("\n"));
assert.equal(bundle.outputs.length, 1);

const uiRoot = path.join(projectRoot, "packages/ui");
const hostCompiler = await compile(
	await Bun.file(path.join(uiRoot, "global.css")).text(),
	{ base: uiRoot, onDependency: () => {} },
);
const hostCss = hostCompiler.build(["hidden", "md:block", "md:flex"]);
const browser = await puppeteer.launch({ headless: true });

try {
	const page = await browser.newPage();
	const errors: string[] = [];
	page.on("pageerror", (error) => errors.push(String(error)));
	page.on("console", (message) => {
		if (message.type() === "error" || message.type() === "warn") {
			errors.push(message.text());
		}
	});
	await page.setViewport({ width: 1440, height: 900 });
	await page.setContent(`<!doctype html><html><head></head><body>
<aside id="host-sidebar" class="hidden md:block">Sidebar</aside>
<nav id="host-nav" class="hidden md:flex">Navigation</nav>
<main id="custom-page" class="w-[137px] md:w-[271px]">
  <div id="custom-child">Custom content</div>
</main>
<div id="unobserved" class="w-[137px]">Host content</div>
</body></html>`);
	await page.addStyleTag({ content: hostCss });
	await page.addScriptTag({ content: await bundle.outputs[0].text() });

	async function hostDisplays() {
		return page.evaluate(() =>
			["host-sidebar", "host-nav"].map((id) => {
				const element = document.getElementById(id);
				return element ? getComputedStyle(element).display : null;
			}),
		);
	}
	async function waitForWidth(id: string, width: string) {
		await page.waitForFunction(
			(elementId, expectedWidth) => {
				const element = document.getElementById(elementId);
				return element && getComputedStyle(element).width === expectedWidth;
			},
			{},
			id,
			width,
		);
	}

	assert.deepEqual(await hostDisplays(), ["block", "flex"]);
	const unscopedCompiler = await compileTailwind(
		runtimeTailwindTheme([
			["--spacing", ".25rem"],
			["--breakpoint-md", "48rem"],
		]),
	);
	const unscopedSheet = await page.addStyleTag({
		content: unscopedCompiler.build(["hidden"]),
	});
	assert.deepEqual(
		await hostDisplays(),
		["none", "none"],
		"The previous global runtime stylesheet reproduces the disappearing sidebar",
	);
	await unscopedSheet.evaluate((element) => element.remove());
	assert.deepEqual(await hostDisplays(), ["block", "flex"]);
	await page.evaluate(() => {
		const root = document.getElementById("custom-page");
		if (!root) throw new Error("Custom page root is missing");
		window.runtimeRootCleanups.page =
			window.runtimeTailwind.observeRuntimeTailwind(root);
	});
	await waitForWidth("custom-page", "271px");

	// A widget can introduce this class after the page has already rendered.
	await page.evaluate(() => {
		const child = document.getElementById("custom-child");
		if (!child) throw new Error("Custom page child is missing");
		child.className = "hidden";
	});
	await page.waitForFunction(() =>
		document
			.querySelector("[data-a2ui-runtime-tailwind]")
			?.textContent?.includes(".hidden"),
	);
	assert.deepEqual(
		await hostDisplays(),
		["block", "flex"],
		"Runtime .hidden must not override the host sidebar's responsive display",
	);
	assert.equal(
		await page.$eval(
			"#custom-child",
			(element) => getComputedStyle(element).display,
		),
		"none",
	);
	assert.notEqual(
		await page.$eval(
			"#unobserved",
			(element) => getComputedStyle(element).width,
		),
		"137px",
		"Runtime utilities must not style unrelated host elements",
	);

	await page.setViewport({ width: 600, height: 900 });
	await waitForWidth("custom-page", "137px");
	assert.deepEqual(await hostDisplays(), ["none", "none"]);
	await page.setViewport({ width: 1440, height: 900 });
	await waitForWidth("custom-page", "271px");
	assert.deepEqual(await hostDisplays(), ["block", "flex"]);

	// Portaled content has its own observed root outside the page's DOM subtree.
	await page.evaluate(() => {
		const portal = document.createElement("section");
		portal.id = "custom-portal";
		portal.className = "w-[219px]";
		portal.innerHTML = '<span class="hidden">Portal content</span>';
		document.body.appendChild(portal);
		window.runtimeRootCleanups.portal =
			window.runtimeTailwind.observeRuntimeTailwind(portal);
	});
	await waitForWidth("custom-portal", "219px");
	assert.equal(
		await page.$eval(
			"#custom-portal span",
			(element) => getComputedStyle(element).display,
		),
		"none",
	);
	assert.deepEqual(await hostDisplays(), ["block", "flex"]);

	await page.evaluate(() => window.runtimeRootCleanups.page());
	assert.equal(
		await page.$eval("#custom-page", (element) =>
			element.hasAttribute("data-a2ui-runtime-tailwind-root"),
		),
		false,
	);
	assert.notEqual(
		await page.$eval(
			"#custom-page",
			(element) => getComputedStyle(element).width,
		),
		"271px",
		"Cleaning up a root must release its runtime styles",
	);
	assert.equal(
		await page.$eval(
			"#custom-portal",
			(element) => getComputedStyle(element).width,
		),
		"219px",
		"The remaining portal must retain the shared stylesheet",
	);
	assert.equal(
		await page.$$eval(
			"[data-a2ui-runtime-tailwind]",
			(sheets) => sheets.length,
		),
		1,
	);

	await page.evaluate(() => window.runtimeRootCleanups.portal());
	await page.waitForFunction(
		() => !document.querySelector("[data-a2ui-runtime-tailwind]"),
	);
	assert.equal(
		await page.$eval("#custom-portal", (element) =>
			element.hasAttribute("data-a2ui-runtime-tailwind-root"),
		),
		false,
	);
	assert.deepEqual(await hostDisplays(), ["block", "flex"]);
	assert.deepEqual(errors, []);
	console.log(
		"Runtime Tailwind browser regression passed: unscoped CSS hides the sidebar; scoped CSS preserves it. Dynamic classes, responsive widths, portal roots, and cleanup passed.",
	);
} finally {
	await browser.close();
}
