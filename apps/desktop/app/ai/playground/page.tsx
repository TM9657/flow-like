"use client";
import { createId } from "@paralleldrive/cuid2";
import { Label } from "@radix-ui/react-context-menu";
import { invoke } from "@tauri-apps/api/core";
import { type Event, type UnlistenFn, listen } from "@tauri-apps/api/event";
import {
	type UseQueryResult,
	type Bit,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	type IDownloadProgress,
	type IPreferences,
	type IResponse,
	type IResponseChunk,
	IRole,
	type ISettingsProfile,
	Input,
	Response as LLMResponse,
	MarkdownComponent,
	Progress,
	Textarea,
	Tooltip,
	TooltipContent,
	TooltipTrigger,
	useBackend,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { CornerDownLeft, Mic } from "lucide-react";
import { useTheme } from "next-themes";
import { useEffect, useRef, useState } from "react";
import PuffLoader from "react-spinners/PuffLoader";

export default function Home() {
	const backend = useBackend();
	const { resolvedTheme } = useTheme();
	const fileInput = useRef<any>(null);
	const [prompt, setPrompt] = useState("");
	const [question, setQuestion] = useState("");
	const [model, setModel] = useState("");
	const profile: UseQueryResult<ISettingsProfile> = useInvoke(
		backend.getSettingsProfile,
		[],
	);
	const [response, setResponse] = useState("");
	const [progress, setProgress] = useState(0);
	const [loading, setLoading] = useState(false);
	const [files, setFiles] = useState<FileList | null>(null);
	const [dlProgress, setDlProgress] = useState<IDownloadProgress>({
		hash: "",
		max: 0,
		downloaded: 0,
		path: "",
	});

	async function fileToBase64(file: File): Promise<string> {
		return new Promise((resolve, reject) => {
			const reader = new FileReader();
			reader.readAsDataURL(file);
			reader.onload = () => resolve(reader.result as string);
			reader.onerror = (error) =>
				reject(new Error("Error converting file to base64"));
		});
	}

	function fixEncoding(corruptedText: string) {
		// Step 1: Encode the corrupted text as Latin-1 bytes
		const bytes = Buffer.from(corruptedText, "latin1");

		// Step 2: Decode those bytes back into UTF-8
		const fixedText = bytes.toString("utf8");

		return fixedText;
	}

	function base64FilesToString(files: string[]) {
		if (files.length === 0) return "";

		const images = files.map((img, index) => {
			return `![img${index + 1}](${img})`;
		});

		return `images: ${images.join(", ")}`;
	}

	async function send() {
		setLoading(true);
		const id = createId();

		const preferences: IPreferences = {
			multilinguality_weight: 0.3,
			reasoning_weight: 0.8,
			factuality_weight: 0.8,
			cost_weight: 0.1,
			creativity_weight: 0.3,
			speed_weight: 0.2,
			openness_weight: 0.2,
		};

		const bit = await invoke<Bit>("find_best_model", {
			preferences,
			multimodal: (files?.length ?? 0) > 0,
			remote: false,
		});

		const base64Files: string[] = files
			? await Promise.all(
					Array.from(files).map(async (file) => await fileToBase64(file)),
				)
			: [];

		setQuestion(prompt);
		setResponse("");
		setPrompt("");
		setModel(bit.meta.en.name);

		const intermediateResponse = LLMResponse.default();

		const streaming_subscription = listen(
			`streaming_out:${id}`,
			(event: Event<IResponseChunk[]>) => {
				for (const chunk of event.payload) {
					if (chunk.x_prefill_progress) {
						setProgress(chunk.x_prefill_progress ?? 0);
						continue;
					}

					setProgress(0);
					intermediateResponse.pushChunk(chunk);
				}
				const lastMessage = intermediateResponse.lastMessageOfRole(
					IRole.Assistant,
				);
				if (lastMessage) {
					setResponse(fixEncoding(lastMessage.content ?? ""));
				}
			},
		);

		const download_model_subscription = listen(
			`download:${bit.hash}`,
			(event: { payload: IDownloadProgress[] }) => {
				const lastElement = event.payload.pop();
				if (lastElement) setDlProgress({ ...lastElement });
			},
		);

		const response = LLMResponse.fromObject(
			await invoke<IResponse>("predict", {
				id,
				systemPrompt:
					"You are Flowy, a next gen AI assistant, always return your answers parsed in markdown. Be friendly and sprinkle some Smileys in your answers. Generally you are super open minded and talk about any topic the user likes to talk about.",
				bit,
				prompt: `${prompt}${base64FilesToString(base64Files)}`,
			}),
		);

		console.dir(response);
		setResponse(
			fixEncoding(response.lastMessageOfRole(IRole.Assistant)?.content ?? ""),
		);

		if (fileInput?.current) fileInput.current.value = null;
		setPrompt("");
		setLoading(false);

		(await streaming_subscription)();
		(await download_model_subscription)();
	}

	useEffect(() => {
		if (!profile.data) return;
		const subscriptions: (Promise<UnlistenFn> | undefined)[] = [];

		profile.data.hub_profile.bits.forEach(([category, model]) => {
			const unlistenFn = listen(
				`download:${model}`,
				(event: { payload: IDownloadProgress[] }) => {
					const lastElement = event.payload.pop();
					if (lastElement) setDlProgress({ ...lastElement });
				},
			);
			subscriptions.push(unlistenFn);
		});

		return () => {
			(async () => {
				for await (const subscription of subscriptions) {
					if (subscription) subscription();
				}
			})();
		};
	}, [profile.data]);

	async function keyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
		if (e.key !== "Enter" || e.shiftKey) return;
		e.preventDefault();
		send();
	}

	return (
		<main className="flex min-h-screen flex-col items-center w-full max-h-dvh overflow-auto dark:bg-dot-destructive/[0.2] bg-dot-destructive/[0.2] p-4">
			{dlProgress.downloaded < dlProgress.max && (
				<div className="flex flex-row items-center w-full relative">
					<Progress
						value={(100 * dlProgress.downloaded) / dlProgress.max}
						max={100}
					/>
					<h4 className="scroll-m-20 text-xl font-semibold tracking-tight ml-2 whitespace-nowrap">
						Downloading{" "}
						{((100 * dlProgress.downloaded) / dlProgress.max).toFixed(2)}%
					</h4>
					<br />
				</div>
			)}

			<div className="flex flex-row items-center w-full">
				<form className="relative overflow-hidden rounded-lg border bg-background focus-within:ring-0 focus-within:ring-ring w-full">
					<Label className="sr-only">Message</Label>
					<Textarea
						onKeyDown={async (e) => {
							keyDown(e);
						}}
						value={prompt}
						onChange={(e) => {
							setPrompt(e.target.value);
						}}
						id="message"
						placeholder="Type your message here..."
						disabled={loading || dlProgress.downloaded < dlProgress.max}
						className="min-h-12 resize-none border-0 p-3 shadow-none focus-visible:ring-0 focus-visible:ring-offset-0"
					/>
					<div className="flex items-center p-3 pt-0">
						<Tooltip>
							<TooltipTrigger asChild>
								<Button variant="ghost" size="icon">
									<Mic className="size-4" />
									<span className="sr-only">Use Microphone</span>
								</Button>
							</TooltipTrigger>
							<TooltipContent side="bottom">Use Microphone</TooltipContent>
						</Tooltip>
						<Tooltip>
							<TooltipTrigger asChild>
								<div className="grid w-full max-w-sm items-center gap-1.5">
									<Input
										ref={fileInput}
										id="picture"
										type="file"
										onChange={(e) => {
											const files = e.target.files;
											console.log("files selected");
											console.log(files);
											if (!files) {
												setFiles(null);
												return;
											}
											setFiles(files);
										}}
									/>
								</div>
							</TooltipTrigger>
							<TooltipContent side="bottom">Attach File</TooltipContent>
						</Tooltip>
						<Button
							disabled={loading}
							type="submit"
							size="sm"
							className="ml-auto gap-1.5"
							onClick={() => {
								send();
							}}
						>
							Send Message
							<CornerDownLeft className="size-3.5" />
						</Button>
					</div>
				</form>
			</div>
			<br />
			{question !== "" && (
				<Card className="w-full">
					<CardHeader>
						<CardTitle className="flex flex-row align-center">
							<div className="flex flex-row items-center">
								{loading && (
									<PuffLoader
										color={resolvedTheme === "dark" ? "white" : "black"}
										className="mr-2"
										size={30}
									/>
								)}{" "}
								{question}
							</div>
						</CardTitle>
						<CardDescription>
							{loading && (
								<p>
									Asking <b className="highlight">{model}</b>
								</p>
							)}
							{!loading && (
								<p>
									Answered by... <b>{model}</b>
								</p>
							)}
							{progress !== 0 && loading && (
								<Progress value={progress * 100} max={100} />
							)}
						</CardDescription>
					</CardHeader>
					<CardContent>
						<MarkdownComponent content={response} />
					</CardContent>
				</Card>
			)}
			<br />
			<br />
		</main>
	);
}
