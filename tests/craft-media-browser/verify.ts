import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import puppeteer from "puppeteer";

interface CraftEvent {
	type: string;
	requestId?: string;
	bytes?: ArrayBuffer;
	revision?: number;
	readOnly?: boolean;
	error?: string;
	dirty?: boolean;
}
declare global {
	interface Window {
		events: CraftEvent[];
		audioProbe?: {
			contexts: AudioContext[];
			nonSilentBuffers: number;
		};
	}
}

const directory = await mkdtemp(join(tmpdir(), "flow-like-craft-media-"));
const assets = resolve(import.meta.dir, "../../apps/web/public/craft");
const runtimes = process.argv.slice(2);
const selected = runtimes.length
	? runtimes
	: ["soundcraft", "filmcraft", "cadcraft"];
const dxf = new TextEncoder().encode(
	"0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n30\n0\n11\n10\n21\n10\n31\n0\n0\nENDSEC\n0\nEOF\n",
);
if (selected.includes("soundcraft")) {
	execFileSync("ffmpeg", [
		"-v",
		"error",
		"-f",
		"lavfi",
		"-i",
		"sine=frequency=440:sample_rate=48000",
		"-t",
		"3",
		"-c:a",
		"libmp3lame",
		"-y",
		join(directory, "sample.mp3"),
	]);
}
if (selected.includes("filmcraft")) {
	execFileSync("ffmpeg", [
		"-v",
		"error",
		"-f",
		"lavfi",
		"-i",
		"color=c=blue:s=32x32:r=10",
		"-f",
		"lavfi",
		"-i",
		"sine=frequency=440:sample_rate=48000",
		"-t",
		"0.3",
		"-c:v",
		"libx264",
		"-pix_fmt",
		"yuv420p",
		"-c:a",
		"aac",
		"-y",
		join(directory, "sample.mp4"),
	]);
}
const server = Bun.serve({
	hostname: "127.0.0.1",
	port: 0,
	async fetch(request) {
		const path = new URL(request.url).pathname;
		if (path === "/")
			return new Response(
				`<!doctype html><html><link rel="icon" href="data:,"><style>html,body,iframe{width:100%;height:100%;margin:0;border:0}</style><script>window.events=[];addEventListener('message',e=>{if(e.origin===location.origin&&e.data.channel==='flow-like-craft')events.push(e.data)});</script><body></body></html>`,
				{ headers: { "Content-Type": "text/html" } },
			);
		const file = Bun.file(join(assets, path));
		return (await file.exists())
			? new Response(file)
			: new Response("Not found", { status: 404 });
	},
});
const browser = await puppeteer.launch({
	headless: true,
	args: [
		"--enable-unsafe-swiftshader",
		"--autoplay-policy=document-user-activation-required",
	],
	...(process.env.CHROME_PATH
		? { executablePath: process.env.CHROME_PATH }
		: {}),
});
let failed = false;
try {
	for (const runtime of selected) {
		const page = await browser.newPage();
		if (runtime === "soundcraft") {
			await page.evaluateOnNewDocument(() => {
				const probe = { contexts: [] as AudioContext[], nonSilentBuffers: 0 };
				window.audioProbe = probe;
				const OriginalContext = window.AudioContext;
				window.AudioContext = class extends OriginalContext {
					constructor(options?: AudioContextOptions) {
						super(options);
						probe.contexts.push(this);
					}
				};
				const originalStart = AudioBufferSourceNode.prototype.start;
				AudioBufferSourceNode.prototype.start = function (...args) {
					if (
						this.buffer
							?.getChannelData(0)
							.some((sample) => Math.abs(sample) > 0.001)
					)
						probe.nonSilentBuffers++;
					return originalStart.apply(this, args);
				};
			});
		}
		await page.setViewport({ width: 1280, height: 900 });
		const errors: string[] = [];
		page.on("pageerror", (error) => errors.push(String(error)));
		page.on("console", (message) => {
			if (message.type() === "error") console.error(runtime, message.text());
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
		await page.goto(server.url.origin);
		await page.evaluate((runtime) => {
			const iframe = document.createElement("iframe");
			iframe.src = `/${runtime}/index.html?session=smoke`;
			document.body.append(iframe);
		}, runtime);
		const waitEvent = async (type: string, requestId?: string) => {
			console.log(
				`${runtime}: waiting for ${type}${requestId ? ` (${requestId})` : ""}`,
			);
			await page.waitForFunction(
				(type, requestId) =>
					window.events.some(
						(e) =>
							(e.type === type && (!requestId || e.requestId === requestId)) ||
							(e.type === "error" &&
								type !== "error" &&
								(!e.requestId || e.requestId === requestId)),
					),
				{ timeout: 180_000 },
				type,
				requestId,
			);
			return page.evaluate(
				(type, requestId) => {
					const event = window.events.find(
						(e) =>
							(e.type === type && (!requestId || e.requestId === requestId)) ||
							(e.type === "error" &&
								type !== "error" &&
								(!e.requestId || e.requestId === requestId)),
					);
					if (event?.type === "error" && type !== "error")
						throw new Error(event.error);
					if (!event) throw new Error(`Missing ${type} event`);
					return {
						...event,
						bytes: event.bytes
							? Array.from(new Uint8Array(event.bytes))
							: undefined,
					};
				},
				type,
				requestId,
			);
		};
		const send = (data: Record<string, unknown>) =>
			page.evaluate(
				(data) =>
					document
						.querySelector("iframe")
						?.contentWindow?.postMessage(
							{ ...data, channel: "flow-like-craft", session: "smoke" },
							location.origin,
						),
				data,
			);
		const settle = () =>
			page.evaluate(
				() =>
					new Promise<void>((resolve) =>
						requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
					),
			);
		const closeAudio = async () => {
			if (runtime !== "soundcraft") return;
			const frame = await (await page.$("iframe"))?.contentFrame();
			await frame?.evaluate(async () => {
				await Promise.all(
					(window.audioProbe?.contexts ?? [])
						.filter((context) => context.state !== "closed")
						.map((context) => context.close()),
				);
			});
		};
		try {
			await waitEvent("ready");
			const bytes =
				runtime === "soundcraft"
					? new Uint8Array(await readFile(join(directory, "sample.mp3")))
					: runtime === "cadcraft"
						? dxf
						: new Uint8Array(await readFile(join(directory, "sample.mp4")));
			const format =
				runtime === "soundcraft"
					? "wav"
					: runtime === "cadcraft"
						? "dxf"
						: "mp4";
			await page.evaluate(
				({ bytes, format }) =>
					document.querySelector("iframe")?.contentWindow?.postMessage(
						{
							channel: "flow-like-craft",
							session: "smoke",
							type: "open",
							name: `sample.${format}`,
							bytes: Uint8Array.from(bytes).buffer,
							readOnly: true,
						},
						location.origin,
					),
				{
					bytes: [...bytes],
					format: runtime === "soundcraft" ? "mp3" : format,
				},
			);
			const opened = await waitEvent("opened");
			assert.equal(opened.readOnly, true);
			await send({ type: "export", requestId: "readonly", format });
			assert.match(
				(await waitEvent("error", "readonly")).error ?? "",
				/editing/i,
			);
			await send({ type: "set-read-only", readOnly: false });
			await settle();
			{
				const iframe = await page.$("iframe");
				assert.ok(iframe);
				const frame = await iframe.contentFrame();
				assert.ok(frame);
				await frame.evaluate(() => document.querySelector("canvas")?.focus());
				const modifier = await page.evaluate(() =>
					navigator.platform.includes("Mac") ? "Meta" : "Control",
				);
				if (runtime === "soundcraft") {
					assert.equal(
						await frame.evaluate(async () => {
							const context = window.audioProbe?.contexts[0];
							// DevTools evaluation can grant activation. Restore the state a
							// fresh browser gives audio created before a user's first click.
							await context?.suspend();
							return context?.state;
						}),
						"suspended",
						"the audio context starts suspended before clicking Play",
					);
					// The pinned canvas toolbar uses fixed-width transport buttons.
					await page.mouse.click(1189, 47);
					await frame.waitForFunction(
						() =>
							window.audioProbe?.contexts.some(
								(context) =>
									context.state === "running" && context.currentTime > 0.2,
							) && (window.audioProbe?.nonSilentBuffers ?? 0) > 2,
						{ timeout: 10_000 },
					);
					await page.keyboard.press("Space");
					await settle();
					console.log(
						"soundcraft: clicking Play resumes the audio clock and outputs decoded MP3 samples",
					);
				}
				if (runtime === "cadcraft") {
					for (const input of ["circle", "2,2", "1"]) {
						await page.keyboard.type(input);
						await page.keyboard.press("Enter");
						await settle();
					}
				} else {
					if (runtime === "soundcraft") {
						await page.keyboard.down(modifier);
						await page.keyboard.press("KeyA");
						await page.keyboard.up(modifier);
					} else await page.keyboard.press("ArrowRight");
					await settle();
					await page.keyboard.down(modifier);
					await page.keyboard.press(runtime === "soundcraft" ? "KeyB" : "KeyK");
					await page.keyboard.up(modifier);
				}
				await settle();
				assert.equal((await waitEvent("dirty")).dirty, true);
			}
			await send({ type: "export", requestId: "save", format });
			const exported = await waitEvent("exported", "save");
			assert.ok(exported.bytes);
			const output = Uint8Array.from(exported.bytes);
			assert.ok(output.length > 44);
			await send({ type: "saved", revision: (exported.revision ?? 0) + 1 });
			await settle();
			assert.equal(
				await page.evaluate(() =>
					window.events.some(
						(event) => event.type === "dirty" && event.dirty === false,
					),
				),
				false,
				"exporting and stale upload acknowledgements keep changes dirty",
			);
			await send({ type: "saved", revision: exported.revision });
			await page.waitForFunction(() =>
				window.events.some(
					(event) => event.type === "dirty" && event.dirty === false,
				),
			);
			if (runtime === "cadcraft") {
				assert.match(new TextDecoder().decode(output), /CIRCLE/);
				await send({ type: "export", requestId: "dwg", format: "dwg" });
				assert.match(
					(await waitEvent("error", "dwg")).error ?? "",
					/DWG|desktop|native/i,
				);
			} else
				assert.equal(
					new TextDecoder().decode(
						output.subarray(
							runtime === "soundcraft" ? 0 : 4,
							runtime === "soundcraft" ? 4 : 8,
						),
					),
					runtime === "soundcraft" ? "RIFF" : "ftyp",
				);
			if (runtime === "filmcraft") {
				const outputPath = join(directory, "exported.mp4");
				await writeFile(outputPath, output);
				const probe = JSON.parse(
					execFileSync(
						"ffprobe",
						["-v", "error", "-show_streams", "-of", "json", outputPath],
						{ encoding: "utf8" },
					),
				);
				assert.ok(
					probe.streams.some(
						(stream: { codec_name: string; width: number }) =>
							stream.codec_name === "h264" && stream.width === 32,
					),
				);
				assert.ok(
					probe.streams.some(
						(stream: { codec_name: string }) => stream.codec_name === "aac",
					),
				);
			}
			await closeAudio();
			await page.goto(server.url.origin);
			await page.evaluate((runtime) => {
				const iframe = document.createElement("iframe");
				iframe.src = `/${runtime}/index.html?session=smoke`;
				document.body.append(iframe);
			}, runtime);
			await waitEvent("ready");
			await page.evaluate(
				({ bytes, format }) =>
					document.querySelector("iframe")?.contentWindow?.postMessage(
						{
							channel: "flow-like-craft",
							session: "smoke",
							type: "open",
							name: `exported.${format}`,
							readOnly: true,
							bytes: Uint8Array.from(bytes).buffer,
						},
						location.origin,
					),
				{ bytes: [...output], format },
			);
			assert.equal((await waitEvent("opened")).readOnly, true);
			assert.deepEqual(errors, []);
			console.log(
				`${runtime}: ready, readonly open, readonly save rejected, edit enabled, ${output.length} byte ${format} export, saved acknowledgement, reopen`,
			);
		} catch (error) {
			failed = true;
			if (runtime === "soundcraft") {
				const frame = await (await page.$("iframe"))?.contentFrame();
				console.error(
					"Playback probe:",
					await frame?.evaluate(() => ({
						contexts: window.audioProbe?.contexts.map((context) => ({
							state: context.state,
							currentTime: context.currentTime,
						})),
						nonSilentBuffers: window.audioProbe?.nonSilentBuffers,
					})),
				);
			}
			console.error(
				runtime,
				await page.evaluate(() =>
					window.events.map(({ bytes, ...event }) => event),
				),
			);
			await page.screenshot({
				path: join(directory, `${runtime}-failure.png`),
			});
			console.error(`Browser failure artifacts: ${directory}`);
			throw error;
		} finally {
			await closeAudio();
			await page.close();
		}
	}
} finally {
	await browser.close();
	server.stop(true);
	if (!failed) await rm(directory, { recursive: true, force: true });
}
