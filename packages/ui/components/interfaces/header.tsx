"use client";
import { InfoIcon, SettingsIcon } from "lucide-react";
import Link from "next/link";
import { type ReactNode, memo, useImperativeHandle, useState } from "react";
import {
	Button,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui";
import type { IToolBarActions } from "./interfaces";

interface HeaderProps {
	usableEvents: Set<string>;
	currentEvent: any;
	sortedEvents: any[];
	metadata: any;
	appId: string;
	switchEvent: (eventId: string) => void;
}

const HeaderInner = ({
	ref,
	usableEvents,
	currentEvent,
	sortedEvents,
	metadata,
	appId,
	switchEvent,
}: HeaderProps & {
	ref: React.RefObject<IToolBarActions>;
}) => {
	const [toolbarElements, setToolbarElements] = useState<ReactNode[]>([]);

	useImperativeHandle(ref, () => ({
		pushToolbarElements: (elements: ReactNode[]) => {
			setToolbarElements(elements);
		},
	}));

	if (!currentEvent) return null;

	return (
		<div className="flex items-center justify-between p-4 bg-background backdrop-blur-xs">
			<div className="flex items-center gap-1">
				<Select value={currentEvent.id} onValueChange={switchEvent}>
					<SelectTrigger className="max-w-[200px] flex flex-row justify-between h-8 bg-muted/20 border-transparent">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{sortedEvents
							.filter((event) => usableEvents.has(event.event_type))
							.map((event) => (
								<SelectItem key={event.id} value={event.id}>
									{event.name ?? event.event_type}
								</SelectItem>
							))}
					</SelectContent>
				</Select>
				<div className="flex items-center gap-1">
					{toolbarElements.map((element, index) => (
						<div key={index}>{element}</div>
					))}
				</div>
			</div>
			<div className="flex items-center gap-2">
				<h1 className="text-lg font-semibold">{metadata?.name}</h1>
				<Link href={`/store?id=${appId}&eventId=${currentEvent.id}`}>
					<Button
						variant="ghost"
						size="icon"
						onClick={() => {
							// Handle chat history toggle
							console.log("Open chat history");
						}}
						className="h-8 w-8 p-0"
					>
						<InfoIcon className="h-4 w-4" />
					</Button>
				</Link>
			</div>
		</div>
	);
};

export const Header = memo(HeaderInner);
Header.displayName = "Header";
