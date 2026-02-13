"use client";

import { Monitor, RotateCcw, Smartphone, Tablet } from "lucide-react";
import { useCallback, useState } from "react";
import { cn } from "../../lib";
import { Button } from "../ui/button";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";

interface DevicePreset {
	name: string;
	width: number;
	height: number;
	icon: typeof Monitor;
}

const DEVICE_PRESETS: DevicePreset[] = [
	{ name: "Desktop", width: 1440, height: 900, icon: Monitor },
	{ name: "Laptop", width: 1280, height: 800, icon: Monitor },
	{ name: "Tablet", width: 768, height: 1024, icon: Tablet },
	{ name: "Mobile", width: 375, height: 812, icon: Smartphone },
	{ name: "Mobile Small", width: 320, height: 568, icon: Smartphone },
];

const BREAKPOINTS = {
	sm: 640,
	md: 768,
	lg: 1024,
	xl: 1280,
	"2xl": 1536,
};

export interface ResponsivePreviewProps {
	className?: string;
	children: React.ReactNode;
}

export function ResponsivePreview({
	className,
	children,
}: ResponsivePreviewProps) {
	const [selectedDevice, setSelectedDevice] = useState(DEVICE_PRESETS[0]);
	const [customWidth, setCustomWidth] = useState(1440);
	const [customHeight, setCustomHeight] = useState(900);
	const [orientation, setOrientation] = useState<"portrait" | "landscape">(
		"landscape",
	);
	const [mode, setMode] = useState<"preset" | "custom">("preset");

	const activeBreakpoint = (() => {
		const width = mode === "preset" ? selectedDevice.width : customWidth;
		if (width >= BREAKPOINTS["2xl"]) return "2xl";
		if (width >= BREAKPOINTS.xl) return "xl";
		if (width >= BREAKPOINTS.lg) return "lg";
		if (width >= BREAKPOINTS.md) return "md";
		if (width >= BREAKPOINTS.sm) return "sm";
		return "xs";
	})();

	const displayWidth =
		mode === "preset"
			? orientation === "portrait"
				? selectedDevice.height
				: selectedDevice.width
			: customWidth;
	const displayHeight =
		mode === "preset"
			? orientation === "portrait"
				? selectedDevice.width
				: selectedDevice.height
			: customHeight;

	const handleDeviceSelect = useCallback((deviceName: string) => {
		const device = DEVICE_PRESETS.find((d) => d.name === deviceName);
		if (device) {
			setSelectedDevice(device);
		}
	}, []);

	const toggleOrientation = useCallback(() => {
		setOrientation((prev) => (prev === "portrait" ? "landscape" : "portrait"));
	}, []);

	return (
		<div className={cn("flex flex-col h-full", className)}>
			{/* Toolbar */}
			<div className="flex items-center justify-between gap-4 p-2 border-b bg-background">
				{/* Device selector */}
				<div className="flex items-center gap-2">
					<Select
						value={selectedDevice.name}
						onValueChange={handleDeviceSelect}
					>
						<SelectTrigger className="w-36 h-8">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{DEVICE_PRESETS.map((device) => {
								const Icon = device.icon;
								return (
									<SelectItem key={device.name} value={device.name}>
										<div className="flex items-center gap-2">
											<Icon className="h-4 w-4" />
											{device.name}
										</div>
									</SelectItem>
								);
							})}
						</SelectContent>
					</Select>

					<Button
						variant="ghost"
						size="sm"
						onClick={toggleOrientation}
						title="Toggle orientation"
					>
						<RotateCcw className="h-4 w-4" />
					</Button>
				</div>

				{/* Size display */}
				<div className="flex items-center gap-2 text-sm text-muted-foreground">
					<span>
						{displayWidth} × {displayHeight}
					</span>
					<span className="px-1.5 py-0.5 rounded bg-muted text-xs font-medium">
						{activeBreakpoint}
					</span>
				</div>

				{/* Quick device buttons */}
				<div className="flex items-center gap-1">
					{DEVICE_PRESETS.slice(0, 4).map((device) => {
						const Icon = device.icon;
						return (
							<Button
								key={device.name}
								variant={
									selectedDevice.name === device.name ? "secondary" : "ghost"
								}
								size="sm"
								onClick={() => setSelectedDevice(device)}
								title={device.name}
							>
								<Icon className="h-4 w-4" />
							</Button>
						);
					})}
				</div>
			</div>

			{/* Preview area */}
			<div className="flex-1 flex items-center justify-center bg-muted/30 overflow-hidden p-4 min-w-0 min-h-0">
				<div
					className="bg-background shadow-lg rounded-lg overflow-hidden transition-all duration-200"
					style={{
						width: displayWidth,
						height: displayHeight,
						maxWidth: "100%",
						maxHeight: "100%",
						flexShrink: 1,
					}}
				>
					<div className="h-full w-full overflow-auto">{children}</div>
				</div>
			</div>

			{/* Breakpoint indicator bar */}
			<div className="flex items-center h-8 px-4 border-t bg-background text-xs">
				<div className="flex items-center gap-2 flex-1">
					{Object.entries(BREAKPOINTS).map(([name, width]) => (
						<div
							key={name}
							className={cn(
								"px-2 py-0.5 rounded transition-colors",
								displayWidth >= width
									? "bg-primary/10 text-primary font-medium"
									: "text-muted-foreground",
							)}
						>
							{name} ({width}px)
						</div>
					))}
				</div>
			</div>
		</div>
	);
}

export interface SideBySidePreviewProps {
	className?: string;
	children: React.ReactNode;
}

export function SideBySidePreview({
	className,
	children,
}: SideBySidePreviewProps) {
	const devices = [
		DEVICE_PRESETS[0], // Desktop
		DEVICE_PRESETS[3], // Mobile
	];

	return (
		<div
			className={cn(
				"flex h-full gap-4 p-4 bg-muted/30 overflow-auto",
				className,
			)}
		>
			{devices.map((device) => {
				const Icon = device.icon;
				return (
					<div key={device.name} className="flex flex-col gap-2">
						<div className="flex items-center gap-2 text-sm text-muted-foreground">
							<Icon className="h-4 w-4" />
							<span>{device.name}</span>
							<span className="text-xs">
								({device.width}×{device.height})
							</span>
						</div>
						<div
							className="bg-background shadow-lg rounded-lg overflow-auto"
							style={{
								width: device.width * 0.5,
								height: device.height * 0.5,
							}}
						>
							<div
								className="overflow-auto"
								style={{
									transform: "scale(0.5)",
									transformOrigin: "top left",
									width: device.width,
									height: device.height,
								}}
							>
								{children}
							</div>
						</div>
					</div>
				);
			})}
		</div>
	);
}
