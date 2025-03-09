"use client";
import { ISettingsProfile } from "@tm9657/flow-like-ui/types";
import { humanFileSize } from "@tm9657/flow-like-ui/lib/utils";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
} from "@tm9657/flow-like-ui/components/ui/avatar";
import { BitHover } from "@tm9657/flow-like-ui/components/ui/bit-hover";
import {
	ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "@tm9657/flow-like-ui/components/ui/chart";
import { useEvent } from "@tm9657/flow-like-ui/hooks/use-event";
import { useInvoke } from "@tm9657/flow-like-ui";
import { Bit, IDownloadProgress } from "@tm9657/flow-like-ui";
import { IBit } from "@tm9657/flow-like-ui/lib/schema/bit/bit";
import { useQueryClient, UseQueryResult } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { CartesianGrid, Line, LineChart, XAxis } from "recharts";

const chartConfig = {
	downloaded: {
		label: "Speed (Mbit/s)",
		color: "hsl(var(--chart-3))",
	},
	total: {
		label: "Total",
		color: "hsl(var(--chart-4))",
	},
} satisfies ChartConfig;

const downloadStatus = new Map<string, IDownloadProgress>();
export default function ProfileCreation() {
	const router = useRouter();
	const queryClient = useQueryClient();
	const intervalRef = useRef<any>(null);
	const searchParams = useSearchParams();
	const [totalSize, setTotalSize] = useState(0);
	const [stats, setStats] = useState<
		{
			time: number;
			timeString: string;
			speed: number;
			total: number;
			progress: number;
			max: number;
		}[]
	>([]);
	const [doneCounter, setDoneCounter] = useState(0);
	const [debugStats, setDebugStats] = useState({
		currentTotal: 0,
		maxTotal: 0,
	});
	const defaultProfiles: UseQueryResult<[ISettingsProfile, IBit[]][]> =
		useInvoke("get_default_profiles", {});

	const [bits, setBits] = useState<Bit[]>([]);

	async function calculateSize() {
		const sizes = await Promise.all(
			bits.map((bit) => Bit.fromObject(bit).fetchSize()),
		);
		setTotalSize(sizes.reduce((acc, size) => acc + size, 0));
	}

	useEffect(() => {
		calculateSize();
	}, [bits]);

	async function addProfiles(profiles: ISettingsProfile[]) {
		for await (const profile of profiles) {
			await invoke("upsert_profile", { profile });
		}
	}

	useEffect(() => {
		if (!defaultProfiles.data) return;
		const selected = new Set(
			searchParams
				.getAll("profiles")
				.map((profile) => profile.replaceAll("profiles=", "")),
		);
		if (selected.size === 0) router.push("/onboarding");

		const filteredProfiles = defaultProfiles.data.filter((profile) =>
			selected.has(profile[0].hub_profile.id ?? ""),
		);
		const foundBits = new Map<string, Bit>();
		filteredProfiles.forEach(([profile, bits]) => {
			bits.forEach((bit) => foundBits.set(bit.id, Bit.fromObject(bit)));
		});

		console.dir({
			filteredProfiles,
			selected,
		});

		setBits(Array.from(foundBits.values()));
		addProfiles(filteredProfiles.map(([profile]) => profile));
	}, [defaultProfiles.data, searchParams]);

	useEffect(() => {
		calculateStats();

		intervalRef.current = setInterval(() => {
			calculateStats();
		}, 1000);

		return () => {
			clearInterval(intervalRef.current);
		};
	}, []);

	useEffect(() => {
		async function finalize() {
			await queryClient.invalidateQueries({
				queryKey: ["get", "current", "profile"],
			});
			await queryClient.invalidateQueries({
				queryKey: ["get", "profiles"],
			});
			router.push("/onboarding/done");
		}
		if (doneCounter >= 5 && bits.length > 0) finalize();
	}, [doneCounter, bits, queryClient]);

	function onDownload(progress: IDownloadProgress) {
		downloadStatus.set(progress.hash, progress);
	}

	function calculateStats() {
		let lastTotal = 0;

		let currentTotal = 0;
		let max = 0;
		downloadStatus.forEach((value) => {
			currentTotal += value.downloaded;
			max += value.max;
		});

		if (max === 0) return;

		const time = Date.now();
		const timeString = new Date(time).toLocaleTimeString();
		setStats((prev) => {
			if (prev.length > 0) {
				lastTotal = prev[prev.length - 1].total;
			}
			const next = [...prev];
			next.push({
				time,
				timeString,
				speed: ((currentTotal - lastTotal) * 8) / 1024 / 1024,
				progress: (100 * currentTotal) / max,
				total: currentTotal,
				max,
			});

			if (next.length > 10) {
				next.shift();
			}

			return next;
		});

		if (currentTotal >= max) {
			setDoneCounter((prev) => prev + 1);
			console.log("Done Counter", doneCounter);
			return;
		}

		setDebugStats({
			currentTotal,
			maxTotal: max,
		});
		setDoneCounter(0);
	}

	return (
		<div className="p-4 max-w-screen-lg w-full">
			<div className="flex flex-row items-center">
				<div className="w-full">
					<h1>🚀 Great Selection!</h1>
					<h2>
						Let´s download your models{" "}
						<b className="highlight">
							{(stats[stats.length - 1]?.progress || 0).toFixed(2)}%
						</b>{" "}
						finished 🤩
					</h2>
					<small>{JSON.stringify(debugStats, null, 2)}</small>
				</div>
			</div>
			<div className="mt-4">
				<ChartContainer className="h-[300px] w-full z-20" config={chartConfig}>
					<LineChart
						accessibilityLayer
						data={stats}
						margin={{
							left: 12,
							right: 12,
						}}
					>
						<CartesianGrid vertical={false} />
						<XAxis
							dataKey="timeString"
							tickLine={false}
							axisLine={false}
							tickMargin={8}
						/>
						<ChartTooltip cursor={false} content={<ChartTooltipContent />} />
						<Line
							dataKey="speed"
							type="monotone"
							allowReorder="yes"
							animateNewValues={false}
							animationEasing="ease-in-out"
							animationDuration={100}
							animationId={1}
							stroke="var(--color-downloaded)"
							strokeWidth={2}
							dot={false}
						/>
					</LineChart>
				</ChartContainer>
			</div>
			<div className="flex flex-row justify-between items-start mt-4">
				<div className="flex flex-row items-center justify-end flex-nowrap gap-2">
					{bits.map((bit) => (
						<BitDownload key={bit.id} onDownload={onDownload} bit={bit} />
					))}
				</div>
				<div className="flex flex-row flex-wrap gap-2 justify-end items-center">
					<div className="border p-2 bg-card text-card-foreground">
						{humanFileSize(totalSize)} Total
					</div>
					<div className="border p-2 bg-card text-card-foreground">
						{stats[stats.length - 1]?.speed.toFixed(2) || 0} Mbit / s
					</div>
				</div>
			</div>
		</div>
	);
}

function BitDownload({
	bit,
	onDownload,
}: Readonly<{ bit: Bit; onDownload: (progress: IDownloadProgress) => void }>) {
	useEvent(
		`download:${bit.hash}`,
		(event: { payload: IDownloadProgress[] }) => {
			const lastElement = event.payload.pop();
			if (lastElement) onDownload(lastElement);
		},
		[],
	);

	useEffect(() => {
		const downloadBit = async () => {
			try {
				await bit.download();
			} catch (error) {
				console.error(error);
			}
		};

		downloadBit();
	}, [bit]);

	return (
		<BitHover bit={bit}>
			<Avatar className="border">
				<AvatarImage className="p-1" src={bit.icon} />
				<AvatarImage className="p-1" src="/app-logo.webp" />
				<AvatarFallback>NA</AvatarFallback>
			</Avatar>
		</BitHover>
	);
}
