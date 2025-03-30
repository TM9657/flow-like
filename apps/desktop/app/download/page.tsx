"use client"

import { Avatar, AvatarFallback, AvatarImage, BitHover, ChartConfig, ChartContainer, ChartTooltip, ChartTooltipContent, humanFileSize, IBit, useDownloadManager } from "@tm9657/flow-like-ui";
import { useRouter } from "next/navigation";
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

export default function Page() {
    const intervalRef = useRef<any>(null);
    const { manager } = useDownloadManager()
    const [bits, setBits] = useState<IBit[]>([]);
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

        useEffect(() => {
                intervalRef.current = setInterval(async () => {
                    const time = Date.now();
		            const timeString = new Date(time).toLocaleTimeString();
                    const measurement = await manager.getSpeed()
                    const bits = await manager.getParents()
                    setBits(bits)
                    setStats((prev) => {
                        const next = [...prev];
                        next.push({
                            time,
                            timeString,
                            speed: measurement.bytesPerSecond,
                            progress: measurement.progress,
                            total: measurement.total,
                            max: measurement.max,
                        });

                        if (next.length > 10) {
                            next.shift();
                        }

                        return next;
                    });
                }, 1000);

                return () => {
                    clearInterval(intervalRef.current);
                };
            }, [manager]);

    return (<main className="flex flex-col items-center justify-center w-full h-full flex-grow">
		<div className="p-4 max-w-screen-lg w-full bg-background border-card border-2 rounded-md">
			<div className="flex flex-row items-center">
				<div className="w-full">
					<h1>🚀 Download Overview!</h1>
					<h2>
						Downloaded your Bits{" "}
						<b className="highlight">
							{((stats[stats.length - 1]?.max ?? 0) === 0 ? 100 : stats[stats.length - 1]?.progress ?? 100).toFixed(2)}%
						</b>{" "}
						finished 🤩
					</h2>
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
                        <BitDownload key={bit.id} bit={bit} />
					))}
				</div>
				<div className="flex flex-row flex-wrap gap-2 justify-end items-center">
					<div className="border p-2 bg-card text-card-foreground">
						{humanFileSize(stats[stats.length - 1]?.max ?? 0)} Total
					</div>
					<div className="border p-2 bg-card text-card-foreground">
						{humanFileSize((stats[stats.length - 1]?.speed ?? 0))} / s
					</div>
				</div>
			</div>
		</div>
                    </main>
	);
}

function BitDownload({
	bit,
}: Readonly<{ bit: IBit }>) {
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
