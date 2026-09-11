"use client";

import { useTranslation } from "@flow-like/locales";
import {
	AlertCircle,
	Braces,
	ChevronDown,
	Expand,
	MapPinIcon,
} from "lucide-react";
import { Suspense, lazy, useMemo, useState } from "react";
import {
	type GeometryDisplay,
	describeGeometry,
	formatGeometryCoordinate,
} from "../../lib/geometry-display";
import { cn } from "../../lib/utils";
import { Button } from "./button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "./dialog";

const GeometryPreview = lazy(() => import("./geometry-preview"));

type GeometryProps = Readonly<{
	value: unknown;
	metadata?: Record<string, unknown>;
}>;

export function GeometrySketch({
	display,
	className,
}: {
	display: GeometryDisplay;
	className?: string;
}) {
	if (display.error)
		return (
			<AlertCircle
				aria-hidden
				className={cn("size-5 text-destructive", className)}
			/>
		);
	if (!display.bounds)
		return (
			<MapPinIcon
				aria-hidden
				className={cn("size-5", className)}
				style={{ color: "var(--pin-geometry, #f97316)" }}
			/>
		);
	return (
		<svg
			viewBox="0 0 64 64"
			aria-hidden
			className={cn("shrink-0", className)}
			style={{ color: "var(--pin-geometry, #f97316)" }}
		>
			<title>{display.kind}</title>
			<rect
				width="64"
				height="64"
				rx="12"
				fill="currentColor"
				fillOpacity="0.07"
			/>
			<path
				d="M16 0V64 M32 0V64 M48 0V64 M0 16H64 M0 32H64 M0 48H64"
				fill="none"
				stroke="currentColor"
				strokeOpacity="0.12"
				strokeWidth="0.75"
			/>
			{display.paths.map(({ d, polygon }) => (
				<path
					key={`${polygon}:${d}`}
					d={d}
					fill={polygon ? "currentColor" : "none"}
					fillOpacity="0.2"
					fillRule="evenodd"
					stroke="currentColor"
					strokeWidth="2.5"
					strokeLinecap="round"
					strokeLinejoin="round"
				/>
			))}
			{display.point && (
				<circle
					cx="32"
					cy="32"
					r="12"
					fill="currentColor"
					fillOpacity="0.12"
					stroke="currentColor"
					strokeOpacity="0.3"
				/>
			)}
			{display.points.map(([x, y]) => (
				<circle
					key={`${x},${y}`}
					cx={x}
					cy={y}
					r={display.point ? 4.5 : 3}
					fill="currentColor"
				/>
			))}
		</svg>
	);
}

function useGeometrySummary(display: GeometryDisplay): string {
	const { t } = useTranslation("flow");
	if (display.error) return t("geometryInvalidValue", "Invalid geometry");
	if (!display.knownCrs) return t("geometryMapUnavailable", "Map unavailable");
	if (display.point)
		return display.point.map(formatGeometryCoordinate).join(", ");
	if (display.kind === "MultiPolygon")
		return t("geometryPolygonCount", "{{count}} polygons", {
			count: display.parts,
			defaultValue_one: "{{count}} polygon",
		});
	if (display.kind === "MultiLineString")
		return t("geometryLineCount", "{{count}} lines", {
			count: display.parts,
			defaultValue_one: "{{count}} line",
		});
	if (display.kind === "GeometryCollection")
		return t("geometryPartCount", "{{count}} parts", {
			count: display.parts,
			defaultValue_one: "{{count}} part",
		});
	if (display.kind === "MultiPoint")
		return t("geometryPointCount", "{{count}} points", {
			count: display.positions,
			defaultValue_one: "{{count}} point",
		});
	return t("geometryVertexCount", "{{count}} vertices", {
		count: display.positions,
		defaultValue_one: "{{count}} vertex",
	});
}

export function GeometryDetails({ value, metadata }: GeometryProps) {
	const { t } = useTranslation("flow");
	const display = useMemo(
		() => describeGeometry(value, metadata),
		[value, metadata],
	);
	const summary = useGeometrySummary(display);
	return (
		<div className="grid min-w-0 gap-3" data-geometry-details>
			<div className="flex min-w-0 items-center gap-3 rounded-xl border border-orange-500/20 bg-orange-500/5 p-3">
				<GeometrySketch display={display} className="size-14 shrink-0" />
				<div className="min-w-0 space-y-1">
					<div className="flex flex-wrap items-center gap-2">
						<span className="font-medium">
							{display.kind ?? t("geometry", "Geometry")}
						</span>
						{display.knownCrs && (
							<span className="rounded border border-orange-500/20 px-1.5 py-0.5 text-[10px] font-medium text-orange-700 dark:text-orange-400">
								WGS 84
							</span>
						)}
					</div>
					<p className="break-words text-xs tabular-nums text-muted-foreground">
						{summary}
					</p>
				</div>
			</div>
			{display.geometry && (
				<>
					<Suspense
						fallback={
							<div className="flex h-52 items-center justify-center rounded-xl border bg-muted/30 text-xs text-muted-foreground">
								{t("geometryLoadingMap", "Loading map…")}
							</div>
						}
					>
						<GeometryPreview value={display.geometry} />
					</Suspense>
					{display.point ? (
						<div className="grid grid-cols-2 gap-2">
							<Coordinate
								label={t("geometryLongitude", "Longitude")}
								value={display.point[0]}
							/>
							<Coordinate
								label={t("geometryLatitude", "Latitude")}
								value={display.point[1]}
							/>
						</div>
					) : (
						<div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
							<span>
								{t("geometryVertexCount", "{{count}} vertices", {
									count: display.positions,
									defaultValue_one: "{{count}} vertex",
								})}
							</span>
							{display.parts > 1 && (
								<span>
									{t("geometryPartCount", "{{count}} parts", {
										count: display.parts,
									})}
								</span>
							)}
							{display.holes > 0 && (
								<span>
									{t("geometryHoleCount", "{{count}} holes", {
										count: display.holes,
										defaultValue_one: "{{count}} hole",
									})}
								</span>
							)}
						</div>
					)}
				</>
			)}
			{!display.knownCrs && (
				<p className="text-xs text-muted-foreground">
					{t(
						"geometryUnknownCrs",
						"Map preview requires WGS 84 CRS metadata and planar edges.",
					)}
				</p>
			)}
			{display.error && (
				<p role="alert" className="break-words text-xs text-destructive">
					{display.error}
				</p>
			)}
			<details className="group/coordinates min-w-0 rounded-lg border">
				<summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-xs text-muted-foreground [&::-webkit-details-marker]:hidden">
					<Braces className="size-3.5" aria-hidden />
					{t("geometryGeoJsonCoordinates", "GeoJSON coordinates")}
					<ChevronDown
						className="ml-auto size-3.5 transition-transform group-open/coordinates:rotate-180"
						aria-hidden
					/>
				</summary>
				<pre className="max-h-64 overflow-auto whitespace-pre-wrap break-all border-t bg-muted/30 p-3 font-mono text-xs">
					{JSON.stringify(value, null, 2)}
				</pre>
			</details>
		</div>
	);
}

function Coordinate({ label, value }: { label: string; value: number }) {
	return (
		<div className="min-w-0 rounded-lg border bg-muted/20 px-3 py-2">
			<div className="text-[10px] text-muted-foreground">{label}</div>
			<div className="font-mono text-sm tabular-nums">
				{formatGeometryCoordinate(value)}°
			</div>
		</div>
	);
}

export function GeometryCell({
	value,
	metadata,
	onClick,
	variant = "compact",
}: GeometryProps &
	Readonly<{
		onClick?: () => void;
		variant?: "compact" | "card";
	}>) {
	const { t } = useTranslation("flow");
	const [open, setOpen] = useState(false);
	const display = useMemo(
		() => describeGeometry(value, metadata),
		[value, metadata],
	);
	const summary = useGeometrySummary(display);
	const label = display.kind ?? t("geometry", "Geometry");
	const card = variant === "card";
	const trigger = (
		<Button
			type="button"
			variant="ghost"
			size="sm"
			className={cn(
				"group/geometry max-w-full justify-start gap-2 overflow-hidden rounded-md border border-orange-500/15 bg-orange-500/5 text-foreground hover:border-orange-500/35 hover:bg-orange-500/10",
				card ? "h-auto min-h-20 w-full rounded-xl p-3" : "h-7 px-1.5 py-0",
			)}
			title={`${label} · ${summary}`}
			aria-label={t("geometryInspectValue", "Inspect {{type}}: {{summary}}", {
				type: label,
				summary,
			})}
			onClick={onClick}
		>
			<GeometrySketch
				display={display}
				className={card ? "size-12" : "size-6"}
			/>
			<span
				className={cn(
					"min-w-0 text-left",
					card
						? "flex-1 space-y-1"
						: "flex items-baseline gap-2 overflow-hidden",
				)}
			>
				<span
					className={cn(
						"block shrink-0 truncate font-medium text-orange-700 dark:text-orange-400",
						card ? "text-sm" : "text-xs",
					)}
				>
					{label}
				</span>
				<span
					className={cn(
						"block truncate font-normal tabular-nums text-muted-foreground",
						card ? "text-xs" : "text-[10px]",
					)}
				>
					{summary}
				</span>
			</span>
			{card && (
				<Expand
					className="size-3.5 shrink-0 text-orange-600/60 transition-colors group-hover/geometry:text-orange-600 dark:text-orange-400/60"
					aria-hidden
				/>
			)}
		</Button>
	);
	if (onClick) return trigger;
	return (
		<Dialog open={open} onOpenChange={setOpen}>
			<DialogTrigger asChild>{trigger}</DialogTrigger>
			<DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-xl">
				<DialogHeader>
					<DialogTitle>{t("geometry", "Geometry")}</DialogTitle>
					<DialogDescription>
						{display.knownCrs
							? t(
									"geometryStoredCoordinateOrder",
									"WGS 84: longitude, latitude in degrees.",
								)
							: t(
									"geometryInspectDescription",
									"Inspect the stored shape and its coordinates.",
								)}
					</DialogDescription>
				</DialogHeader>
				<GeometryDetails value={value} metadata={metadata} />
			</DialogContent>
		</Dialog>
	);
}
