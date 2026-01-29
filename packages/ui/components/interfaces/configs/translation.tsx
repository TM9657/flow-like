"use client";
import {
	Badge,
	type IBoard,
	type IEvent,
	type IEventMapping,
	type IEventPayload,
	type INode,
	Label,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@tm9657/flow-like-ui";
import { Cloud, Laptop, MonitorSmartphone } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

function SinkAvailabilityBadge({
	availability,
	description,
}: Readonly<{ availability: "local" | "remote" | "both"; description?: string }>) {
	const config = {
		local: {
			icon: Laptop,
			label: "Local",
			variant: "secondary" as const,
		},
		remote: {
			icon: Cloud,
			label: "Remote",
			variant: "default" as const,
		},
		both: {
			icon: MonitorSmartphone,
			label: "Both",
			variant: "outline" as const,
		},
	}[availability];

	const Icon = config.icon;

	const badge = (
		<Badge variant={config.variant} className="text-xs gap-1 ml-2">
			<Icon className="h-3 w-3" />
			{config.label}
		</Badge>
	);

	if (description) {
		return (
			<TooltipProvider>
				<Tooltip>
					<TooltipTrigger asChild>{badge}</TooltipTrigger>
					<TooltipContent>
						<p>{description}</p>
					</TooltipContent>
				</Tooltip>
			</TooltipProvider>
		);
	}

	return badge;
}

export function EventTypeConfiguration({
	eventConfig,
	node,
	event,
	disabled,
	onUpdate,
}: Readonly<{
	eventConfig: IEventMapping;
	node: INode;
	disabled: boolean;
	event: IEvent;
	onUpdate: (type: string, config: Partial<IEventPayload>) => void;
}>) {
	const foundConfig = eventConfig[node?.name];

	useEffect(() => {
		const eventTypes = eventConfig[node?.name];
		if (!eventTypes) {
			console.warn(`No event types configured for node: ${node?.name}`);
			return;
		}

		if (!eventTypes.eventTypes.includes(event.event_type)) {
			onUpdate(
				eventTypes.defaultEventType,
				eventTypes.configs[eventTypes.defaultEventType] ?? {},
			);
		}
	}, [node?.name, event.event_type]);

	if (foundConfig?.eventTypes.length <= 1) return null;

	const getSinkAvailability = (type: string) => {
		if (!foundConfig?.withSink?.includes(type)) return null;
		return foundConfig?.sinkAvailability?.[type] ?? { availability: "local" as const };
	};

	return (
		<div className="space-y-3">
			<Label htmlFor="event_type">Event Type</Label>
			<Select
				disabled={disabled}
				value={event.event_type}
				onValueChange={(value) => {
					onUpdate(value, foundConfig.configs[value] ?? {});
				}}
			>
				<SelectTrigger className="w-full">
					<SelectValue placeholder="Select event type" />
				</SelectTrigger>
				<SelectContent>
					{foundConfig?.eventTypes.map((type) => {
						const sinkConfig = getSinkAvailability(type);
						return (
							<SelectItem key={type} value={type}>
								<span className="flex items-center">
									{type.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}
									{sinkConfig && (
										<SinkAvailabilityBadge
											availability={sinkConfig.availability}
											description={sinkConfig.description}
										/>
									)}
								</span>
							</SelectItem>
						);
					})}
				</SelectContent>
			</Select>
		</div>
	);
}

export function EventTranslation({
	appId,
	eventConfig,
	eventType,
	editing,
	board,
	nodeId,
	config,
	onUpdate,
}: Readonly<{
	appId: string;
	eventConfig: IEventMapping;
	eventType: string;
	editing: boolean;
	config: Partial<IEventPayload>;
	board: IBoard;
	nodeId?: string;
	onUpdate: (payload: Partial<IEventPayload>) => void;
}>) {
	const [intermediateConfig, setIntermediateConfig] =
		useState<Partial<IEventPayload>>(config);
	const node: INode | undefined = board.nodes[nodeId ?? ""];

	const foundEventConfig = useMemo(() => {
		return eventConfig[node?.name];
	}, [node?.name]);

	const ConfigInterface = useMemo(() => {
		if (!foundEventConfig) return null;
		return foundEventConfig.configInterfaces[eventType] || null;
	}, [foundEventConfig, eventType]);

	const configProps = useMemo(
		() => ({
			isEditing: editing,
			appId,
			boardId: board.id,
			config: intermediateConfig,
			node: node,
			nodeId: nodeId ?? "",
			onConfigUpdate: (payload: Partial<IEventPayload>) => {
				setIntermediateConfig(payload);
				if (onUpdate) {
					onUpdate(payload);
				}
			},
		}),
		[
			editing,
			board.app_id,
			board.id,
			intermediateConfig,
			node,
			nodeId,
			onUpdate,
		],
	);

	if (!node) {
		return <p className="text-red-500">Node not found.</p>;
	}

	if (!foundEventConfig || !ConfigInterface) {
		return (
			<div className="w-full space-y-4">
				<p className="text-sm text-muted-foreground">
					No specific configuration available for this event type.
				</p>
			</div>
		);
	}

	return (
		<div className="w-full space-y-4">
			<ConfigInterface {...configProps} />
		</div>
	);
}
