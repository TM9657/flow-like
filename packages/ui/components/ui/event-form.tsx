"use client";

import { useState } from "react";
import { useInvoke } from "../../hooks";
import type { IEvent } from "../../lib";
import { convertJsonToUint8Array } from "../../lib/uint8";
import { useBackend } from "../../state/backend-state";
import type { IEventMapping } from "../interfaces";
import { Button } from "./button";
import { EventTypeConfig } from "./event-type-config";
import { Input } from "./input";
import { Label } from "./label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./select";
import { Separator } from "./separator";
import { Textarea } from "./textarea";

interface EventFormProps {
	event?: IEvent;
	eventConfig: IEventMapping;
	appId: string;
	onSubmit: (event: Partial<IEvent>) => void;
	onCancel: () => void;
}

export function EventForm({
	eventConfig,
	appId,
	event,
	onSubmit,
	onCancel,
}: Readonly<EventFormProps>) {
	const backend = useBackend();
	const [formData, setFormData] = useState({
		name: event?.name ?? "",
		description: event?.description ?? "",
		board_version: undefined,
		node_id: event?.node_id ?? "",
		board_id: event?.board_id ?? "",
		event_type: undefined,
		config: [],
	});

	const boards = useInvoke(backend.boardState.getBoards, backend.boardState, [
		appId,
	]);
	const board = useInvoke(
		backend.boardState.getBoard,
		backend.boardState,
		[appId, formData.board_id, formData.board_version],
		(formData.board_id ?? "") !== "",
	);

	const versions = useInvoke(
		backend.boardState.getBoardVersions,
		backend.boardState,
		[appId, formData.board_id],
		(formData.board_id ?? "") !== "",
	);

	const [selectedNodeType, setSelectedNodeType] = useState<string>("");
	const [eventTypeConfig, setEventTypeConfig] = useState<any>({});

	const handleInputChange = (field: string, value: any) => {
		setFormData((prev) => ({ ...prev, [field]: value }));
	};

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();

		const eventData: Partial<IEvent> = {
			...formData,
			variables: event?.variables || {},
			...(selectedNodeType && { eventTypeConfig }),
		};

		onSubmit(eventData);
	};

	const isEditing = !!event;

	return (
		<form onSubmit={handleSubmit} className="space-y-6">
			{/* Basic Information */}
			<div className="space-y-4">
				<div className="space-y-2">
					<Label htmlFor="name">Event Name</Label>
					<Input
						id="name"
						value={formData.name}
						onChange={(e) => handleInputChange("name", e.target.value)}
						placeholder="Enter event name"
						required
					/>
				</div>

				<div className="space-y-2">
					<Label htmlFor="description">Description</Label>
					<Textarea
						id="description"
						value={formData.description}
						onChange={(e) => handleInputChange("description", e.target.value)}
						placeholder="Enter event description"
						rows={3}
					/>
				</div>
			</div>

			<Separator />

			{/* Board Selection */}
			<div className="space-y-4">
				<div className="space-y-2">
					<Label htmlFor="board">Flow</Label>
					<Select
						value={formData.board_id}
						onValueChange={(value) => {
							handleInputChange("board_id", value);
							handleInputChange("board_version", undefined);
							handleInputChange("node_id", undefined);
							setSelectedNodeType("");
							setEventTypeConfig({});
						}}
					>
						<SelectTrigger>
							<SelectValue placeholder="Select a board" />
						</SelectTrigger>
						<SelectContent>
							{boards.data?.map((board) => (
								<SelectItem key={board.id} value={board.id}>
									{board.name}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>
			</div>

			{/* Board Version Selection */}
			<div className="space-y-4">
				<div className="space-y-2">
					<Label htmlFor="board">Flow Version</Label>
					<Select
						value={formData.board_version ?? ""}
						onValueChange={(value) => {
							handleInputChange(
								"board_version",
								value === "" ? undefined : value.split(".").map(Number),
							);
							handleInputChange("node_id", undefined);
						}}
					>
						<SelectTrigger>
							<SelectValue placeholder="Latest" />
						</SelectTrigger>
						<SelectContent>
							{versions.data?.map((board) => (
								<SelectItem key={board.join(".")} value={board.join(".")}>
									v{board.join(".")}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>
			</div>

			{/* Node and Board Selection */}
			{board.data && (
				<div className="space-y-4">
					<div className="space-y-2">
						<Label htmlFor="node">Node</Label>
						<Select
							value={formData.node_id}
							onValueChange={(value) => {
								handleInputChange("node_id", value);
								const node = board.data.nodes[value];
								if (!node) return;
								if (node) {
									const eventType = eventConfig[node?.name];
									if (eventType) {
										handleInputChange("event_type", eventType.defaultEventType);
										handleInputChange(
											"config",
											convertJsonToUint8Array(
												eventType.configs[eventType.defaultEventType] ?? {},
											) ?? [],
										);
									}
								}
							}}
						>
							<SelectTrigger>
								<SelectValue placeholder="Select a node" />
							</SelectTrigger>
							<SelectContent>
								{Object.values(board.data.nodes)
									.filter((node) => node.start)
									.map((node) => (
										<SelectItem key={node.id} value={node.id}>
											{node?.friendly_name || node?.name}
										</SelectItem>
									))}
							</SelectContent>
						</Select>
					</div>

					{/* Event Type Selector - shown when node has multiple event types */}
					{formData.node_id && board.data.nodes[formData.node_id] && (() => {
						const node = board.data.nodes[formData.node_id];
						const nodeEventConfig = eventConfig[node?.name];

						if (!nodeEventConfig || nodeEventConfig.eventTypes.length <= 1) {
							return null;
						}

						return (
							<div className="space-y-2">
								<Label htmlFor="event_type">Event Type</Label>
								<Select
									value={formData.event_type || nodeEventConfig.defaultEventType}
									onValueChange={(value) => {
										handleInputChange("event_type", value);
										handleInputChange(
											"config",
											convertJsonToUint8Array(
												nodeEventConfig.configs[value] ?? {},
											) ?? [],
										);
									}}
								>
									<SelectTrigger>
										<SelectValue placeholder="Select event type" />
									</SelectTrigger>
									<SelectContent>
										{nodeEventConfig.eventTypes.map((type) => (
											<SelectItem key={type} value={type}>
												{type.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
						);
					})()}
				</div>
			)}

			{/* Type-specific Configuration */}
			{selectedNodeType && (
				<>
					<Separator />
					<EventTypeConfig
						type={selectedNodeType}
						config={eventTypeConfig}
						onChange={setEventTypeConfig}
					/>
				</>
			)}

			{/* Form Actions */}
			<div className="flex justify-end space-x-2 pt-4 border-t">
				<Button type="button" variant="outline" onClick={onCancel}>
					Cancel
				</Button>
				<Button
					type="submit"
					disabled={!formData.name || !formData.board_id || !formData.node_id}
				>
					{isEditing ? "Update Event" : "Create Event"}
				</Button>
			</div>
		</form>
	);
}
