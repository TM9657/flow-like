"use client";

import {
	FloatingPortal,
	autoUpdate,
	flip,
	offset,
	shift,
	useFloating,
} from "@floating-ui/react";
import {
	type BarDatum,
	BarItem,
	type BarItemProps,
	type BarTooltipProps,
	ResponsiveBar,
	type ResponsiveBarSvgProps,
} from "@nivo/bar";
import { useValueFormatter } from "@nivo/core";
import { useMemo, useState } from "react";
import { resolveColorToRgb, useChartTokens } from "../../../lib/chart-theme";

const compactNumber = new Intl.NumberFormat("en-US", {
	notation: "compact",
	maximumFractionDigits: 1,
});
const smallNumber = new Intl.NumberFormat("en-US", {
	maximumSignificantDigits: 3,
});
const exactNumber = new Intl.NumberFormat("en-US", {
	maximumSignificantDigits: 21,
});

export function formatBarTick(value: number): string {
	return (Math.abs(value) >= 1000 ? compactNumber : smallNumber).format(value);
}

let textContext: CanvasRenderingContext2D | null | undefined;

function textWidth(text: string, fontSize: number, fontFamily: string): number {
	if (!textContext && typeof document !== "undefined") {
		textContext = document.createElement("canvas").getContext("2d");
	}
	if (!textContext) return text.length * fontSize * 0.65;
	textContext.font = `${fontSize}px ${fontFamily}`;
	return textContext.measureText(text).width;
}

function contrastingLabel({ color }: { color: string }): string {
	const channels = resolveColorToRgb(color, [251, 86, 45]).map((channel) => {
		const value = channel / 255;
		return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
	});
	const luminance =
		channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
	return luminance > 0.179 ? "#000000" : "#ffffff";
}

interface MarkdownBarChartProps extends ResponsiveBarSvgProps<BarDatum> {
	autoLeftMargin?: boolean;
	autoLeftLegendOffset?: boolean;
}

export default function MarkdownBarChart({
	autoLeftMargin,
	autoLeftLegendOffset,
	...props
}: MarkdownBarChartProps) {
	const [node, setNode] = useState<HTMLDivElement | null>(null);
	const [width, setWidth] = useState(props.defaultWidth ?? 640);
	const tokens = useChartTokens(node);
	const horizontal = props.layout === "horizontal";
	const fontFamily = node
		? getComputedStyle(node).fontFamily
		: "ui-sans-serif, system-ui, sans-serif";
	const tickStyle = {
		fontSize: 10,
		fontFamily,
		...props.theme?.text,
		...props.theme?.axis?.ticks?.text,
		...props.axisLeft?.style?.ticks?.text,
	};
	const labelStyle = {
		fontSize: 11,
		fontFamily,
		...props.theme?.text,
		...props.theme?.labels?.text,
	};
	const tickSize = Number.parseFloat(String(tickStyle.fontSize)) || 10;
	const labelSize = Number.parseFloat(String(labelStyle.fontSize)) || 11;
	const tickFont = tickStyle.fontFamily?.includes("var(")
		? fontFamily
		: tickStyle.fontFamily;
	const labelFont = labelStyle.fontFamily?.includes("var(")
		? fontFamily
		: labelStyle.fontFamily;
	const formatCategory = useValueFormatter<string | number>(
		props.axisLeft?.format,
	);
	const margin = { top: 30, right: 30, bottom: 50, left: 60, ...props.margin };
	let axisLeft = props.axisLeft;

	if (horizontal && axisLeft !== null) {
		const indexBy = props.indexBy ?? "id";
		const labels = props.data.map((item) =>
			String(
				formatCategory(
					typeof indexBy === "function" ? indexBy(item) : item[indexBy],
				),
			),
		);
		const padding = (axisLeft?.tickPadding ?? 5) + (axisLeft?.tickSize ?? 5);
		const legendSpace = axisLeft?.legend ? 34 : 12;
		const fitMargin = autoLeftMargin ?? props.margin?.left === undefined;
		const fitLegend =
			autoLeftLegendOffset ?? axisLeft?.legendOffset === undefined;
		const available = Math.max(
			24,
			(fitMargin ? width * 0.5 : margin.left) - padding - legendSpace,
		);
		const labelWidth = Math.min(
			available,
			Math.max(
				0,
				...labels.map((label) => textWidth(label, tickSize, tickFont)),
			),
		);
		if (fitMargin) {
			margin.left = Math.max(
				margin.left,
				Math.ceil(labelWidth + padding + legendSpace),
				axisLeft?.legend && !fitLegend
					? Math.abs(axisLeft.legendOffset ?? 0) + 14
					: 0,
			);
		}
		axisLeft = {
			...axisLeft,
			legendOffset: fitLegend
				? -(labelWidth + padding + 20)
				: axisLeft?.legendOffset,
			format:
				axisLeft?.format ??
				((value: string | number) => {
					const label = String(formatCategory(value));
					if (textWidth(label, tickSize, tickFont) <= available) return label;
					let end = label.length;
					while (
						end > 0 &&
						textWidth(`${label.slice(0, end)}…`, tickSize, tickFont) > available
					)
						end--;
					return `${label.slice(0, end)}…`;
				}),
		};
	}

	const barComponent = useMemo(
		() =>
			function FittedBar(item: BarItemProps<BarDatum>) {
				return (
					<BarItem
						{...item}
						shouldRenderLabel={
							item.shouldRenderLabel &&
							textWidth(item.label, labelSize, labelFont) + 12 <=
								item.bar.width &&
							labelSize + 6 <= item.bar.height
						}
					/>
				);
			},
		[labelSize, labelFont],
	);
	const tooltip = useMemo(
		() =>
			function BarTooltip(item: BarTooltipProps<BarDatum>) {
				const { refs, floatingStyles } = useFloating({
					placement: "top",
					strategy: "fixed",
					middleware: [offset(8), flip(), shift({ padding: 8 })],
					whileElementsMounted: (reference, floating, update) =>
						autoUpdate(reference, floating, update, { animationFrame: true }),
				});
				return (
					<>
						<span ref={refs.setReference} />
						<FloatingPortal>
							<div
								ref={refs.setFloating}
								role="tooltip"
								style={{
									...floatingStyles,
									zIndex: 50,
									maxWidth: "min(20rem, calc(100vw - 16px))",
									padding: "8px 12px",
									border: `1px solid ${tokens.tooltipBorder}`,
									borderRadius: 8,
									background: tokens.tooltipBackground,
									color: tokens.tooltipForeground,
									fontFamily,
									fontSize: 12,
									lineHeight: 1.5,
									overflowWrap: "anywhere",
									pointerEvents: "none",
								}}
							>
								<div>{props.tooltipLabel ? item.label : item.indexValue}</div>
								<div>
									{item.id}: <strong>{item.formattedValue}</strong>
								</div>
							</div>
						</FloatingPortal>
					</>
				);
			},
		[tokens, fontFamily, props.tooltipLabel],
	);

	return (
		<div ref={setNode} className="h-full w-full min-w-0">
			<ResponsiveBar
				valueFormat={(value) => exactNumber.format(value)}
				labelTextColor={contrastingLabel}
				barComponent={barComponent}
				tooltip={tooltip}
				isFocusable
				barAriaLabel={(item) =>
					`${item.indexValue}, ${item.id}: ${item.formattedValue}`
				}
				enableGridX={horizontal}
				enableGridY={!horizontal}
				{...props}
				margin={margin}
				axisLeft={axisLeft}
				axisBottom={
					horizontal && props.axisBottom !== null
						? {
								format: formatBarTick,
								tickValues: Math.max(
									2,
									Math.min(
										8,
										Math.floor((width - margin.left - margin.right) / 80),
									),
								),
								...props.axisBottom,
							}
						: props.axisBottom
				}
				onResize={(size) => {
					setWidth(size.width);
					props.onResize?.(size);
				}}
			/>
		</div>
	);
}
