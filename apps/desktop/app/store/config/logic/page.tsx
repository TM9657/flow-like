"use client";
import { invoke } from "@tauri-apps/api/core";
import {
	type UseQueryResult,
	Badge,
	Button,
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	type IApp,
	type IBoard,
	Input,
	Label,
	Separator,
	Textarea,
	formatRelativeTime,
	useBackend,
	useFlowBoardParentState,
	useInvoke,
} from "@tm9657/flow-like-ui";
import { PlusCircleIcon, WorkflowIcon } from "lucide-react";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

export default function Page() {
	const backend = useBackend();
	const parentRegister = useFlowBoardParentState();
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const app = useInvoke(backend.getApp, [id ?? ""], typeof id === "string");
	const boards = useInvoke(
		backend.getBoards,
		[id ?? ""],
		typeof id === "string",
	);

	useEffect(() => {
		if (!app.data) return;
		if (!boards.data) return;
		boards.data?.forEach((board) => {
			parentRegister?.addBoardParent(board.id, `/store/config/logic?id=${id}`);
		});
	}, [boards.data, id]);

	const [boardCreation, setBoardCreation] = useState({
		open: false,
		name: "",
		description: "",
	});

	return (
		<main>
			<div className="flex py-2 flex-row items-center gap-2 sticky top-0 bg-background">
				<h3>Boards</h3>
				<Dialog
					open={boardCreation.open}
					onOpenChange={(open) => setBoardCreation({ ...boardCreation, open })}
				>
					<DialogTrigger>
						<Button size={"sm"} className="gap-2">
							<PlusCircleIcon />
							Add Board
						</Button>
					</DialogTrigger>
					<DialogContent>
						<DialogHeader>
							<DialogTitle>Add Board</DialogTitle>
							<DialogDescription>
								Create a new board for the app
							</DialogDescription>
						</DialogHeader>
						<div className="grid w-full max-w-sm items-center gap-1.5">
							<Label htmlFor="name">Name</Label>
							<Input
								value={boardCreation.name}
								id="name"
								placeholder="Name"
								onChange={(e) => {
									setBoardCreation((old) => ({ ...old, name: e.target.value }));
								}}
							/>
						</div>
						<div className="grid w-full max-w-sm items-center gap-1.5">
							<Label htmlFor="description">Description</Label>
							<Textarea
								value={boardCreation.description}
								id="description"
								placeholder="Description"
								onChange={(e) => {
									setBoardCreation((old) => ({
										...old,
										description: e.target.value,
									}));
								}}
							/>
						</div>
						<DialogFooter className="gap-2">
							<Button
								variant="outline"
								onClick={() =>
									setBoardCreation({ ...boardCreation, open: false })
								}
							>
								Cancel
							</Button>
							<Button
								onClick={async () => {
									await invoke("create_app_board", {
										appId: app.data?.id,
										name: boardCreation.name,
										description: boardCreation.description,
									});
									await Promise.allSettled([
										await boards.refetch(),
										await app.refetch(),
									]);
									setBoardCreation({ name: "", description: "", open: false });
								}}
							>
								Create
							</Button>
						</DialogFooter>
					</DialogContent>
				</Dialog>
			</div>
			<Separator className="mt-2 mb-4" />
			<div className="flex flex-col gap-2 mt-4 pr-2">
				{app.data &&
					boards.data?.map((board) => (
						<Board
							key={board.id}
							board={board}
							app={app.data}
							boardsQuery={boards}
						/>
					))}
			</div>
		</main>
	);
}

function Board({
	board,
	app,
	boardsQuery,
}: Readonly<{
	board: IBoard;
	app: IApp;
	boardsQuery: UseQueryResult<IBoard[]>;
}>) {
	const router = useRouter();

	return (
		<ContextMenu>
			<ContextMenuTrigger asChild className="w-full">
				<button
					type="button"
					className="flex w-full flex-row items-stretch gap-2 border p-3 bg-card/80 rounded-md cursor-pointer hover:bg-card"
					onClick={async () => {
						await invoke("get_app_board", {
							appId: app.id,
							boardId: board.id,
							pushToRegistry: true,
						});
						router.push(`/flow?id=${board.id}`);
					}}
				>
					<div className="flex flex-row items-center gap-2 w-full justify-between">
						<div className="w-full flex flex-row items-center gap-2">
							<WorkflowIcon className="min-w-6 min-h-6 h-6 w-6" />
							<div className="flex flex-col items-start w-full">
								<div className="flex flex-row items-center gap-2">
									{board.name}
									<Badge variant="outline">{board.stage}</Badge>
								</div>
								<small className="text-muted-foreground justify-start text-start line-clamp-1">
									{board.description === ""
										? "[Description]"
										: board.description}
								</small>
							</div>
						</div>
						<div className="flex flex-row items-center justify-between min-w-fit">
							<div className="flex flex-col items-end gap-2">
								<small>{formatRelativeTime(board.updated_at)}</small>
								<div className="flex flex-row items-center gap-2">
									<Badge>{board.log_level}</Badge>
									<Badge className="min-w-fit">
										{Object.keys(board.variables).length} Variables
									</Badge>
									<Badge className="min-w-fit">
										{Object.keys(board.nodes).length} Nodes
									</Badge>
								</div>
							</div>
						</div>
					</div>
				</button>
			</ContextMenuTrigger>
			<ContextMenuContent>
				<ContextMenuItem
					disabled={(boardsQuery.data?.length ?? 2) <= 1}
					onClick={async () => {
						await invoke("delete_app_board", {
							appId: app.id,
							boardId: board.id,
						});
						await boardsQuery.refetch();
					}}
				>
					Delete
				</ContextMenuItem>
			</ContextMenuContent>
		</ContextMenu>
	);
}
