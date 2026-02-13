import { cn } from "../../lib/utils";
import type {
	Background,
	Border,
	Overflow,
	Position,
	ResponsiveOverrides,
	Shadow,
	Spacing,
	Style,
	Transform,
} from "./types";

export function resolveStyle(style: Style | undefined): string {
	if (!style) return "";

	const classes: string[] = [];

	// Primary class names (Tailwind-first approach)
	if (style.className) {
		classes.push(style.className);
	}

	// Overflow
	if (style.overflow) {
		classes.push(overflowToClass(style.overflow));
	}

	// Responsive overrides
	if (style.responsiveOverrides) {
		classes.push(...resolveResponsiveOverrides(style.responsiveOverrides));
	}

	return cn(...classes);
}

function overflowToClass(overflow: Overflow): string {
	switch (overflow) {
		case "visible":
			return "overflow-visible";
		case "hidden":
			return "overflow-hidden";
		case "scroll":
			return "overflow-scroll";
		case "auto":
			return "overflow-auto";
		default:
			return "";
	}
}

function resolveResponsiveOverrides(overrides: ResponsiveOverrides): string[] {
	const classes: string[] = [];

	if (overrides.sm?.className) {
		classes.push(...overrides.sm.className.split(" ").map((c) => `sm:${c}`));
	}
	if (overrides.sm?.hidden) classes.push("sm:hidden");

	if (overrides.md?.className) {
		classes.push(...overrides.md.className.split(" ").map((c) => `md:${c}`));
	}
	if (overrides.md?.hidden) classes.push("md:hidden");

	if (overrides.lg?.className) {
		classes.push(...overrides.lg.className.split(" ").map((c) => `lg:${c}`));
	}
	if (overrides.lg?.hidden) classes.push("lg:hidden");

	if (overrides.xl?.className) {
		classes.push(...overrides.xl.className.split(" ").map((c) => `xl:${c}`));
	}
	if (overrides.xl?.hidden) classes.push("xl:hidden");

	if (overrides.xxl?.className) {
		classes.push(...overrides.xxl.className.split(" ").map((c) => `2xl:${c}`));
	}
	if (overrides.xxl?.hidden) classes.push("2xl:hidden");

	return classes;
}

export function resolveInlineStyle(
	style: Style | undefined,
): React.CSSProperties {
	if (!style) return {};

	const inlineStyle: React.CSSProperties = {};

	// Background
	if (style.background) {
		Object.assign(inlineStyle, backgroundToCss(style.background));
	}

	// Border
	if (style.border) {
		Object.assign(inlineStyle, borderToCss(style.border));
	}

	// Shadow
	if (style.shadow) {
		inlineStyle.boxShadow = shadowToCss(style.shadow);
	}

	// Position
	if (style.position) {
		Object.assign(inlineStyle, positionToCss(style.position));
	}

	// Transform
	if (style.transform) {
		Object.assign(inlineStyle, transformToCss(style.transform));
	}

	// Spacing (margin & padding)
	if (style.margin) {
		Object.assign(inlineStyle, spacingToCss(style.margin, "margin"));
	}
	if (style.padding) {
		Object.assign(inlineStyle, spacingToCss(style.padding, "padding"));
	}
	if (style.gap) inlineStyle.gap = style.gap;

	// Sizing
	if (style.width) inlineStyle.width = style.width;
	if (style.height) inlineStyle.height = style.height;
	if (style.minWidth) inlineStyle.minWidth = style.minWidth;
	if (style.minHeight) inlineStyle.minHeight = style.minHeight;
	if (style.maxWidth) inlineStyle.maxWidth = style.maxWidth;
	if (style.maxHeight) inlineStyle.maxHeight = style.maxHeight;

	// Flex item properties
	if (style.flex) inlineStyle.flex = style.flex;
	if (style.flexGrow !== undefined) inlineStyle.flexGrow = style.flexGrow;
	if (style.flexShrink !== undefined) inlineStyle.flexShrink = style.flexShrink;
	if (style.flexBasis) inlineStyle.flexBasis = style.flexBasis;
	if (style.alignSelf) inlineStyle.alignSelf = style.alignSelf;

	// Grid item properties
	if (style.gridColumn) inlineStyle.gridColumn = style.gridColumn;
	if (style.gridRow) inlineStyle.gridRow = style.gridRow;
	if (style.gridArea) inlineStyle.gridArea = style.gridArea;
	if (style.justifySelf) inlineStyle.justifySelf = style.justifySelf;

	// Typography
	if (style.color) inlineStyle.color = style.color;
	if (style.fontSize) inlineStyle.fontSize = style.fontSize;
	if (style.fontWeight) inlineStyle.fontWeight = style.fontWeight;
	if (style.fontFamily) inlineStyle.fontFamily = style.fontFamily;
	if (style.lineHeight) inlineStyle.lineHeight = style.lineHeight;
	if (style.letterSpacing) inlineStyle.letterSpacing = style.letterSpacing;
	if (style.textAlign) inlineStyle.textAlign = style.textAlign;
	if (style.textDecoration) inlineStyle.textDecoration = style.textDecoration;
	if (style.textTransform) inlineStyle.textTransform = style.textTransform;
	if (style.whiteSpace) inlineStyle.whiteSpace = style.whiteSpace;
	if (style.wordBreak) inlineStyle.wordBreak = style.wordBreak;

	// Visibility & interaction
	if (style.opacity !== undefined) inlineStyle.opacity = style.opacity;
	if (style.visibility) inlineStyle.visibility = style.visibility;
	if (style.cursor) inlineStyle.cursor = style.cursor;
	if (style.userSelect) inlineStyle.userSelect = style.userSelect;
	if (style.pointerEvents) inlineStyle.pointerEvents = style.pointerEvents;

	// Stacking
	if (style.zIndex !== undefined) inlineStyle.zIndex = style.zIndex;

	// Transitions & animations
	if (style.transition) inlineStyle.transition = style.transition;
	if (style.animation) inlineStyle.animation = style.animation;

	// Display
	if (style.display) inlineStyle.display = style.display;

	// Outline
	if (style.outline) inlineStyle.outline = style.outline;
	if (style.outlineOffset) inlineStyle.outlineOffset = style.outlineOffset;

	// Filters
	if (style.filter) inlineStyle.filter = style.filter;
	if (style.backdropFilter) inlineStyle.backdropFilter = style.backdropFilter;

	// Aspect ratio
	if (style.aspectRatio) inlineStyle.aspectRatio = style.aspectRatio;

	return inlineStyle;
}

function backgroundToCss(bg: Background): React.CSSProperties {
	if ("color" in bg) {
		return { backgroundColor: bg.color };
	}
	if ("gradient" in bg) {
		const { gradient } = bg;
		const stops = gradient.stops
			.map(
				(s) => `${s.color}${s.position !== undefined ? ` ${s.position}%` : ""}`,
			)
			.join(", ");

		switch (gradient.type) {
			case "linear":
				return {
					background: `linear-gradient(${gradient.angle ?? 180}deg, ${stops})`,
				};
			case "radial":
				return { background: `radial-gradient(${stops})` };
			case "conic":
				return {
					background: `conic-gradient(from ${gradient.angle ?? 0}deg, ${stops})`,
				};
		}
	}
	if ("image" in bg) {
		const { image } = bg;
		const url = "literalString" in image.url ? image.url.literalString : "";
		return {
			backgroundImage: `url(${url})`,
			backgroundSize: image.size ?? "cover",
			backgroundPosition: image.position ?? "center",
			backgroundRepeat: image.repeat ?? "no-repeat",
		};
	}
	if ("blur" in bg) {
		return { backdropFilter: `blur(${bg.blur})` };
	}
	return {};
}

function borderToCss(border: Border): React.CSSProperties {
	const style: React.CSSProperties = {};
	if (border.width) style.borderWidth = border.width;
	if (border.style) style.borderStyle = border.style;
	if (border.color) style.borderColor = border.color;
	if (border.radius) style.borderRadius = border.radius;
	return style;
}

function shadowToCss(shadow: Shadow): string {
	const parts = [
		shadow.inset ? "inset" : "",
		shadow.x ?? "0",
		shadow.y ?? "0",
		shadow.blur ?? "0",
		shadow.spread ?? "0",
		shadow.color ?? "rgba(0,0,0,0.25)",
	].filter(Boolean);
	return parts.join(" ");
}

function positionToCss(pos: Position): React.CSSProperties {
	const style: React.CSSProperties = { position: pos.type };
	if (pos.top) style.top = pos.top;
	if (pos.right) style.right = pos.right;
	if (pos.bottom) style.bottom = pos.bottom;
	if (pos.left) style.left = pos.left;
	return style;
}

function transformToCss(transform: Transform): React.CSSProperties {
	const transforms: string[] = [];
	if (transform.translate) transforms.push(`translate(${transform.translate})`);
	if (transform.rotate !== undefined)
		transforms.push(`rotate(${transform.rotate}deg)`);
	if (transform.scale) transforms.push(`scale(${transform.scale})`);

	const style: React.CSSProperties = {};
	if (transforms.length > 0) style.transform = transforms.join(" ");
	if (transform.transformOrigin)
		style.transformOrigin = transform.transformOrigin;
	return style;
}

function spacingToCss(
	spacing: Spacing,
	type: "margin" | "padding",
): React.CSSProperties {
	const style: React.CSSProperties = {};
	if (spacing.top) style[`${type}Top`] = spacing.top;
	if (spacing.right) style[`${type}Right`] = spacing.right;
	if (spacing.bottom) style[`${type}Bottom`] = spacing.bottom;
	if (spacing.left) style[`${type}Left`] = spacing.left;
	return style;
}

// Utility to merge multiple styles
export function mergeStyles(...styles: (Style | undefined)[]): {
	className: string;
	style: React.CSSProperties;
} {
	const classNames: string[] = [];
	const inlineStyles: React.CSSProperties[] = [];

	for (const s of styles) {
		if (s) {
			classNames.push(resolveStyle(s));
			inlineStyles.push(resolveInlineStyle(s));
		}
	}

	return {
		className: cn(...classNames),
		style: Object.assign({}, ...inlineStyles),
	};
}
