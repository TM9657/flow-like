import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import tailwindcss from "@tailwindcss/postcss";
import { strFromU8, strToU8, unzipSync, zipSync } from "fflate";
import postcss from "postcss";
import puppeteer from "puppeteer";
import sharp from "sharp";

const directory = await mkdtemp(join(tmpdir(), "flow-like-wordcraft-browser-"));
const assets = resolve(import.meta.dir, "../../apps/web/public/wordcraft");
const sample = zipSync({
	"[Content_Types].xml": strToU8(
		'<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
	),
	"_rels/.rels": strToU8(
		'<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
	),
	"word/document.xml": strToU8(
		'<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Original document</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>',
	),
});
const build = await Bun.build({
	entrypoints: [join(import.meta.dir, "main.tsx")],
	outdir: directory,
	target: "browser",
	format: "esm",
	splitting: true,
	define: { "process.env.NODE_ENV": JSON.stringify("production") },
});
assert.ok(build.success, build.logs.join("\n"));
const stylePath = resolve(import.meta.dir, "../../packages/ui/global.css");
const styles = await postcss([tailwindcss()]).process(
	await readFile(stylePath, "utf8"),
	{ from: stylePath },
);
let storedDocument: Uint8Array = sample;
const requests: string[] = [];
const server = Bun.serve({
	hostname: "127.0.0.1",
	port: 0,
	async fetch(request) {
		const path = new URL(request.url).pathname;
		requests.push(path);
		if (path === "/library/config/storage")
			return new Response(
				`<!doctype html><html class="dark"><link rel="icon" href="data:,"><link rel="stylesheet" href="/style.css"><div id="app"></div><script type="module" src="/main.js"></script></html>`,
				{ headers: { "Content-Type": "text/html" } },
			);
		if (path === "/style.css")
			return new Response(styles.css, {
				headers: { "Content-Type": "text/css" },
			});
		if (path === "/sample.docx" && request.method === "PUT") {
			storedDocument = new Uint8Array(await request.arrayBuffer());
			return new Response(null, { status: 204 });
		}
		if (path === "/sample.docx")
			return new Response(storedDocument, {
				headers: {
					"Content-Type":
						"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
					"Cache-Control": "max-age=3600",
				},
			});
		const file = Bun.file(
			path.startsWith("/wordcraft/")
				? join(assets, path.slice("/wordcraft/".length))
				: join(directory, path),
		);
		if (!(await file.exists()))
			return new Response("Not found", { status: 404 });
		return new Response(file, {
			headers: path.endsWith(".mjs")
				? { "Content-Type": "text/javascript" }
				: {},
		});
	},
});
const browser = await puppeteer.launch({
	headless: true,
	args: ["--enable-unsafe-swiftshader"],
	...(process.env.CHROME_PATH
		? { executablePath: process.env.CHROME_PATH }
		: {}),
});
try {
	const page = await browser.newPage();
	await page.evaluateOnNewDocument(() => {
		localStorage.setItem("flow-like:language", "en");
	});
	const clickButton = async (label: string) => {
		for (const button of await page.$$("button")) {
			if (
				await button.evaluate(
					(element, text) => element.textContent?.includes(text),
					label,
				)
			) {
				await button.click();
				return;
			}
		}
		throw new Error(`Button not found: ${label}`);
	};
	await page.setViewport({ width: 1280, height: 900 });
	const errors: string[] = [];
	page.on("response", (response) => {
		if (response.status() >= 400)
			errors.push(`HTTP ${response.status()}: ${response.url()}`);
	});
	page.on("pageerror", (error) => {
		errors.push(String(error));
		console.error(error);
	});
	page.on("console", (message) => {
		if (message.type() === "error") console.error(message.text());
	});
	await page.setRequestInterception(true);
	page.on("request", (request) => {
		if (
			request.url().startsWith(server.url.origin) ||
			/^(blob|data):/.test(request.url())
		)
			void request.continue();
		else {
			errors.push(`External request: ${request.url()}`);
			void request.abort();
		}
	});
	await page.goto(`${server.url.origin}/library/config/storage`);
	assert.equal(
		requests.some((path) => path.startsWith("/wordcraft/")),
		false,
		"runtime stays unloaded before DOCX selection",
	);
	await page.click("#open");
	await page.waitForFunction(
		() =>
			[...document.querySelectorAll("button")].some(
				(button) =>
					button.textContent?.includes("Edit document") && !button.disabled,
			),
		{ timeout: 90_000 },
	);
	assert.ok(
		requests.some((path) => /wordcraft\.wasm\.gz\.\d+$/.test(path)),
		"local WASM loaded on demand",
	);
	const header = await page.evaluate(() => {
		const title = document
			.querySelector('[data-slot="dialog-title"]')
			?.getBoundingClientRect();
		const edit = document
			.querySelector('[aria-label="Edit document"]')
			?.getBoundingClientRect();
		const close = document
			.querySelector('[data-slot="dialog-close"]')
			?.getBoundingClientRect();
		return {
			title: title && title.y + title.height / 2,
			edit: edit && edit.y + edit.height / 2,
			close: close && close.y + close.height / 2,
		};
	});
	assert.ok(header.title && header.edit && header.close);
	assert.ok(
		Math.abs(header.title - header.edit) < 3 &&
			Math.abs(header.edit - header.close) < 3,
		"title, edit action and close share one header row",
	);
	const previewFrame = await (
		await page.waitForSelector("iframe")
	)?.contentFrame();
	assert.ok(previewFrame);
	await previewFrame.evaluate(
		() =>
			new Promise<void>((resolve) =>
				requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
			),
	);
	const canvas = await previewFrame.waitForSelector("canvas");
	assert.ok(canvas);
	const { data: pixels, info } = await sharp(await canvas.screenshot())
		.removeAlpha()
		.raw()
		.toBuffer({ resolveWithObject: true });
	const paper: number[] = [];
	for (let x = 0; x < info.width; x++) {
		const offset = (32 * info.width + x) * info.channels;
		if (
			pixels[offset] > 250 &&
			pixels[offset + 1] > 250 &&
			pixels[offset + 2] > 250
		)
			paper.push(x);
	}
	assert.ok(paper.length > info.width / 2, "preview paper rendered");
	assert.ok(
		Math.abs(paper[0] - (info.width - 1 - paper[paper.length - 1])) < 8,
		"reading page is horizontally centered",
	);
	await page.screenshot({ path: join(directory, "preview.png") });
	await clickButton("Edit document");
	const frameElement = await page.waitForSelector("iframe");
	const frame = await frameElement?.contentFrame();
	assert.ok(frame, "document iframe exists");
	await frame.waitForSelector("canvas");
	const renderEditor = () =>
		frame.evaluate(
			() =>
				new Promise<void>((resolve) =>
					requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
				),
		);
	// The WASM event loop applies the mode and focus on animation frames.
	await renderEditor();
	await frame.focus("canvas");
	await renderEditor();
	await page.keyboard.press("End");
	await page.keyboard.type(" Edited in FlowLike", { delay: 30 });
	await page.waitForFunction(() => window.documentDirty === true);
	let discardPrompted = false;
	page.once("dialog", (dialog) => {
		discardPrompted = true;
		void dialog.dismiss();
	});
	await page.evaluate(() => window.history.back());
	await page.waitForFunction(() => window.discardPrompts === 1);
	await page.waitForFunction(
		() => window.history.state?.__flowLikeDocx?.entry === "document",
	);
	assert.ok(discardPrompted, "browser Back asks before discarding edits");
	assert.equal(
		frame.isDetached(),
		false,
		"canceling Back keeps the editor mounted",
	);
	await clickButton("Save");
	await page.waitForFunction(
		() => !!window.savedDocx && window.documentDirty === false,
		{ timeout: 30_000 },
	);
	const saved = new Uint8Array(
		await page.evaluate(() => window.savedDocx ?? []),
	);
	assert.deepEqual(
		saved,
		storedDocument,
		"Save replaces bytes at the original URL",
	);
	const document = strFromU8(unzipSync(saved)["word/document.xml"]);
	assert.ok(
		document.includes("Edited in FlowLike"),
		"typed text survives DOCX export",
	);
	assert.ok(
		document.includes("Original document"),
		"original text remains in saved document",
	);
	await page.evaluate(() => window.history.back());
	await page.waitForSelector("iframe", { hidden: true });
	assert.ok(
		page.url().endsWith("/library/config/storage"),
		"Back closes the saved document within Storage",
	);
	await page.click("#reopen");
	await page.waitForFunction(
		() =>
			[...document.querySelectorAll("button")].some(
				(button) =>
					button.textContent?.includes("Edit document") && !button.disabled,
			),
		{ timeout: 90_000 },
	);
	const downloaded = await page.evaluate(async () => {
		const link = document.querySelector("a[download]") as HTMLAnchorElement;
		return Array.from(
			new Uint8Array(await (await fetch(link.href)).arrayBuffer()),
		);
	});
	assert.deepEqual(
		new Uint8Array(downloaded),
		saved,
		"reopening the same URL fetches saved bytes despite prior cache headers",
	);
	await page.screenshot({ path: join(directory, "reopened.png") });
	await page.setViewport({ width: 390, height: 844 });
	await page.waitForFunction(() => {
		const actions = document
			.querySelector('[aria-label="Edit document"]')
			?.getBoundingClientRect();
		const close = document
			.querySelector('[data-slot="dialog-close"]')
			?.getBoundingClientRect();
		return (
			actions &&
			close &&
			actions.right < close.left &&
			close.right < window.innerWidth
		);
	});
	await page.screenshot({ path: join(directory, "mobile.png") });
	assert.deepEqual(
		errors,
		[],
		"runtime has no browser exceptions or remote asset requests",
	);
	console.log(
		`DOCX lazy load, viewing, editing, binary save and reopening passed. Screenshots: ${directory}`,
	);
} catch (error) {
	const page = (await browser.pages()).at(-1);
	await page?.screenshot({ path: join(directory, "failure.png") });
	console.error(await page?.evaluate(() => document.body.innerText));
	console.error(`Browser artifacts: ${directory}`);
	throw error;
} finally {
	await browser.close();
	await server.stop(true);
	if (!process.env.KEEP_WORDCRAFT_TEST_OUTPUT)
		await rm(directory, { recursive: true, force: true });
}
