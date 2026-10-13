import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer";

const directory = dirname(fileURLToPath(import.meta.url));

test(
	"browser codecs play and seek embedded audio/video, capture posters and report unsupported formats",
	{ timeout: 90000 },
	async (t) => {
		const temporary = mkdtempSync(join(tmpdir(), "deck-browser-codecs-"));
		t.after(() => rmSync(temporary, { recursive: true, force: true }));
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
			join(temporary, "audio.mp3"),
		]);
		execFileSync("ffmpeg", [
			"-v",
			"error",
			"-f",
			"lavfi",
			"-i",
			"color=c=blue:s=96x64:r=10",
			"-f",
			"lavfi",
			"-i",
			"sine=frequency=660:sample_rate=48000",
			"-t",
			"3",
			"-c:v",
			"libx264",
			"-pix_fmt",
			"yuv420p",
			"-c:a",
			"aac",
			"-y",
			join(temporary, "video.mp4"),
		]);
		const server = createServer((request, response) => {
			if (request.url === "/") {
				response.setHeader("Content-Type", "text/html");
				response.end(
					'<!doctype html><button id="unlock">Enable playback</button>',
				);
				return;
			}
			const path =
				request.url === "/browser-media.js"
					? join(directory, "browser-media.js")
					: join(
							temporary,
							request.url === "/audio.mp3" ? "audio.mp3" : "video.mp4",
						);
			response.setHeader(
				"Content-Type",
				request.url === "/browser-media.js"
					? "text/javascript"
					: request.url === "/audio.mp3"
						? "audio/mpeg"
						: "video/mp4",
			);
			response.end(readFileSync(path));
		});
		await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
		t.after(() => new Promise((resolve) => server.close(resolve)));
		const browser = await puppeteer.launch({
			headless: true,
			args: [
				"--no-sandbox",
				"--autoplay-policy=document-user-activation-required",
			],
		});
		t.after(() => browser.close());
		const page = await browser.newPage();
		const errors = [];
		page.on("pageerror", (error) => errors.push(error.message));
		await page.goto(`http://127.0.0.1:${server.address().port}`);
		await page.evaluate(async () => {
			window.analysers = [];
			window.gains = [];
			window.playAttempts = [];
			window.repaints = 0;
			const createGain = AudioContext.prototype.createGain;
			AudioContext.prototype.createGain = function () {
				const gain = createGain.call(this);
				window.gains.push(gain);
				return gain;
			};
			const play = HTMLMediaElement.prototype.play;
			HTMLMediaElement.prototype.play = function () {
				window.playAttempts.push({
					muted: this.muted,
					volume: this.volume,
					gain: window.gains.at(-1)?.gain.value,
				});
				return play.call(this);
			};
			const original = AudioContext.prototype.createMediaElementSource;
			AudioContext.prototype.createMediaElementSource = function (...args) {
				const node = original.apply(this, args);
				const analyser = this.createAnalyser();
				node.connect(analyser);
				window.analysers.push(analyser);
				return node;
			};
			window.media = await import("/browser-media.js");
			media.setRepaint(() => window.repaints++);
			window.options = (extra = {}) =>
				JSON.stringify({
					trimStart: 0,
					trimEnd: 0,
					fadeIn: 0,
					fadeOut: 0,
					volume: 1,
					looping: false,
					duration: 0,
					from: null,
					...extra,
				});
		});
		await page.click("#unlock");
		await page.evaluate(async () => {
			media.register(
				"audio",
				new Uint8Array(await (await fetch("/audio.mp3")).arrayBuffer()),
			);
			await media.prepare("audio");
			media.play("1", "audio", options({ volume: 0.4 }));
		});
		const firstPlay = await page.evaluate(() => window.playAttempts.at(-1));
		assert.equal(firstPlay.volume, 1);
		assert.ok(Math.abs(firstPlay.gain - 0.4) < 0.0001);
		await page.evaluate(() => media.tick("1"));
		assert.ok(
			Math.abs(
				(await page.evaluate(() => window.gains.at(-1).gain.value)) - 0.4,
			) < 0.0001,
		);
		await page.waitForFunction(
			() =>
				media.status("1")?.position > 0.2 &&
				media.status("1")?.state === "playing",
		);
		await page.waitForFunction(() =>
			window.analysers.some((analyser) => {
				const samples = new Uint8Array(analyser.fftSize);
				analyser.getByteTimeDomainData(samples);
				return samples.some((value) => Math.abs(value - 128) > 2);
			}),
		);
		await page.evaluate(() => {
			media.pause("1");
			media.seek("1", 1.4);
		});
		await page.waitForFunction(
			() =>
				media.status("1").state === "paused" &&
				Math.abs(media.status("1").position - 1.4) < 0.05,
		);
		assert.ok((await page.evaluate(() => media.probe("audio").duration)) > 2.9);
		await page.evaluate(() => {
			media.stop("1");
			media.play("silent", "audio", options({ volume: 0 }));
		});
		assert.deepEqual(await page.evaluate(() => window.playAttempts.at(-1)), {
			muted: true,
			volume: 1,
			gain: 0,
		});
		await page.evaluate(async () => {
			media.stop("silent");
			media.register(
				"video",
				new Uint8Array(await (await fetch("/video.mp4")).arrayBuffer()),
			);
			await media.prepare("video");
			media.attach("2", "video");
			media.play("2", "video", options());
		});
		await page.waitForFunction(() => {
			const frame = media.frame("2");
			if (frame) window.lastFrame = frame;
			return (
				media.status("2")?.position > 0.2 && window.lastFrame?.rgba[2] > 200
			);
		});
		assert.deepEqual(
			await page.evaluate(() => [lastFrame.width, lastFrame.height]),
			[96, 64],
		);
		await page.evaluate(() => {
			media.pause("2");
			media.seek("2", 1.25);
		});
		await page.waitForFunction(() => {
			const frame = media.frame("2");
			return frame && Math.abs(frame.time - 1.25) < 0.05;
		});
		await page.evaluate(() => media.requestPoster(42, "video", 1.25));
		await page.waitForFunction(() => !media.pendingPosters());
		assert.deepEqual(await page.evaluate(() => media.takePoster()), {
			shape: 42,
			ms: 1250,
		});
		assert.deepEqual(
			await page.evaluate(() =>
				Array.from(media.posterBytes("video", 1.25).slice(0, 8)),
			),
			[137, 80, 78, 71, 13, 10, 26, 10],
		);
		await page.evaluate(async () => {
			media.stop("2");
			media.register(
				"unsupported",
				new TextEncoder().encode("This is not a playable media container."),
			);
			await media.prepare("unsupported");
			media.play("3", "unsupported", options());
		});
		await page.waitForFunction(() => Boolean(media.status("3")?.error));
		assert.match(
			await page.evaluate(() => media.status("3").error),
			/cannot play this media format/,
		);
		await page.evaluate(() => media.stop("3"));
		assert.ok((await page.evaluate(() => window.repaints)) > 5);
		assert.deepEqual(errors, []);
	},
);
