"use client";

import { CheckIcon, CopyIcon } from "lucide-react";
import {
	NodeApi,
	SlateElement,
	type SlateElementProps,
	SlateLeaf,
	type SlateLeafProps,
	type TCodeBlockElement,
} from "platejs";
import * as React from "react";
import { Button } from "../../..";

export function CodeBlockElementStatic(
	props: SlateElementProps<TCodeBlockElement>,
) {
	return (
		<SlateElement className="codeblock py-1" {...props}>
			<div className="relative rounded-md bg-muted/50">
				<pre className="overflow-x-auto p-8 pr-4 font-mono text-sm leading-[normal] [tab-size:2] print:break-inside-avoid">
					<code>{props.children}</code>
				</pre>

				<div
					className="absolute top-1 right-1 z-10 flex gap-0.5 select-none"
					contentEditable={false}
				>
					<CopyButton
						size="icon"
						variant="ghost"
						className="size-6 gap-1 text-xs text-muted-foreground"
						value={() => getCodeBlockText(props.element)}
					/>
				</div>
			</div>
		</SlateElement>
	);
}

export function CodeLineElementStatic(props: SlateElementProps) {
	return <SlateElement {...props} />;
}

export function CodeSyntaxLeafStatic(props: SlateLeafProps) {
	const tokenClassName = props.leaf.className as string;

	return <SlateLeaf className={tokenClassName} {...props} />;
}

function getCodeBlockText(element: TCodeBlockElement): string {
	const children = (element?.children ?? []) as any[];
	if (!children.length) return "";
	return children.map((line) => NodeApi.string(line)).join("\n");
}

function CopyButton({
	value,
	...props
}: { value: (() => string) | string } & Omit<
	React.ComponentProps<typeof Button>,
	"value"
>) {
	const [hasCopied, setHasCopied] = React.useState(false);

	React.useEffect(() => {
		const t = setTimeout(() => {
			setHasCopied(false);
		}, 2000);
		return () => clearTimeout(t);
	}, [hasCopied]);

	return (
		<Button
			onClick={() => {
				void navigator.clipboard.writeText(
					typeof value === "function" ? value() : value,
				);
				setHasCopied(true);
			}}
			{...props}
		>
			<span className="sr-only">Copy</span>
			{hasCopied ? (
				<CheckIcon className="size-3!" />
			) : (
				<CopyIcon className="size-3!" />
			)}
		</Button>
	);
}
