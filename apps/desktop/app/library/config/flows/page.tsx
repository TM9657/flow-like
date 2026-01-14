"use client";
import { createId } from "@paralleldrive/cuid2";
import { invoke } from "@tauri-apps/api/core";
import {
	Badge,
	BubbleActions,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
	type IApp,
	type IBoard,
	IExecutionStage,
	ILogLevel,
	Input,
	Label,
	Skeleton,
	Textarea,
	type UseQueryResult,
	formatRelativeTime,
	useBackend,
	useFlowBoardParentState,
	useInvoke,
} from "@tm9657/flow-like-ui";
import {
	Calendar,
	ExternalLink,
	FileText,
	PlusCircleIcon,
	SquareMousePointerIcon,
	Trash2,
	VariableIcon,
	WorkflowIcon,
} from "lucide-react";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

export default function Page() {
	const backend = useBackend();
	const parentRegister = useFlowBoardParentState();
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const app = useInvoke(
		backend.appState.getApp,
		backend.appState,
		[id ?? ""],
		typeof id === "string",
	);
	const boards = useInvoke(
		backend.boardState.getBoards,
		backend.boardState,
		[id ?? ""],
		typeof id === "string",
	);

	useEffect(() => {
		if (!app.data) return;
		if (!boards.data) return;
		boards.data?.forEach((board) => {
			parentRegister?.addBoardParent(
				board.id,
				`/library/config/flows?id=${id}`,
			);
		});
	}, [boards.data, id]);

	const [boardCreation, setBoardCreation] = useState({
		open: false,
		name: "",
		description: "",
	});

	const handleCreateBoard = async () => {
		if (!id) return;
		await backend.boardState.upsertBoard(
			id,
			createId(),
			boardCreation.name,
			boardCreation.description,
			ILogLevel.Debug,
			IExecutionStage.Dev,
		);
		await Promise.allSettled([await boards.refetch(), await app.refetch()]);
		setBoardCreation({
			name: "",
			description: "",
			open: false,
		});
	};

	return (
		<main className="h-full flex flex-col max-h-full overflow-auto md:overflow-visible min-h-0">
			<div className="container mx-auto px-6 pb-4 flex flex-col h-full gap-4">
				<PageHeader
					boardCreation={boardCreation}
					setBoardCreation={setBoardCreation}
					onCreateBoard={handleCreateBoard}
				/>

				<BoardsSection
					boards={boards}
					app={app.data}
					boardCreation={boardCreation}
					setBoardCreation={setBoardCreation}
				/>
			</div>
		</main>
	);
}

function PageHeader({
	boardCreation,
	setBoardCreation,
	onCreateBoard,
}: Readonly<{
	boardCreation: { open: boolean; name: string; description: string };
	setBoardCreation: (value: any) => void;
	onCreateBoard: () => Promise<void>;
}>) {
	return (
		<div className="flex flex-col space-y-6 flex-shrink-0">
			<div className="flex items-center justify-between">
				<div className="space-y-2">
					<h1 className="text-4xl font-bold tracking-tight bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text text-transparent">
						Project Flows
					</h1>
					<p className="text-muted-foreground text-lg">
						Manage and organize your application workflows
					</p>
				</div>
				<CreateFlowDialog
					boardCreation={boardCreation}
					setBoardCreation={setBoardCreation}
					onCreateBoard={onCreateBoard}
				/>
			</div>
		</div>
	);
}

function CreateFlowDialog({
	boardCreation,
	setBoardCreation,
	onCreateBoard,
}: Readonly<{
	boardCreation: { open: boolean; name: string; description: string };
	setBoardCreation: (value: any) => void;
	onCreateBoard: () => Promise<void>;
}>) {
	return (
		<Dialog
			open={boardCreation.open}
			onOpenChange={(open) => setBoardCreation({ ...boardCreation, open })}
		>
			<DialogTrigger asChild>
				<Button
					size="lg"
					className="gap-2 shadow-lg hover:shadow-xl transition-all duration-200 bg-gradient-to-r from-primary to-primary/80"
				>
					<PlusCircleIcon className="h-5 w-5" />
					Create New Flow
				</Button>
			</DialogTrigger>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle className="text-2xl">Create New Flow</DialogTitle>
					<DialogDescription className="text-base">
						Design a new flow for your application
					</DialogDescription>
				</DialogHeader>
				<div className="space-y-4 py-4">
					<div className="space-y-2">
						<Label htmlFor="name" className="text-sm font-medium">
							Flow Name
						</Label>
						<Input
							value={boardCreation.name}
							id="name"
							placeholder="Enter flow name..."
							className="h-11"
							onChange={(e) => {
								setBoardCreation((old: any) => ({
									...old,
									name: e.target.value,
								}));
							}}
						/>
					</div>
					<div className="space-y-2">
						<Label htmlFor="description" className="text-sm font-medium">
							Description
						</Label>
						<Textarea
							value={boardCreation.description}
							id="description"
							placeholder="Describe the purpose of this flow..."
							className="min-h-[100px] resize-none"
							onChange={(e) => {
								setBoardCreation((old: any) => ({
									...old,
									description: e.target.value,
								}));
							}}
						/>
					</div>
				</div>
				<DialogFooter className="gap-3">
					<Button
						variant="outline"
						onClick={() => setBoardCreation({ ...boardCreation, open: false })}
					>
						Cancel
					</Button>
					<Button
						onClick={onCreateBoard}
						className="bg-gradient-to-r from-primary to-primary/80"
					>
						Create Board
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function BoardsSection({
	boards,
	app,
	boardCreation,
	setBoardCreation,
}: Readonly<{
	boards: UseQueryResult<IBoard[]>;
	app?: IApp;
	boardCreation: { open: boolean; name: string; description: string };
	setBoardCreation: (value: any) => void;
}>) {
	if (boards.isLoading) {
		return (
			<div className="space-y-6 flex flex-1 flex-col flex-grow max-h-full h-full overflow-y-auto md:overflow-visible overflow-x-hidden">
				<BoardsSkeleton />
			</div>
		);
	}

	if (boards.data?.length === 0) {
		return (
			<div className="space-y-6 flex flex-1 flex-col flex-grow max-h-full h-full overflow-y-auto md:overflow-visible overflow-x-hidden">
				<EmptyBoards
					setBoardCreation={setBoardCreation}
					boardCreation={boardCreation}
				/>
			</div>
		);
	}

	const uniqueBoards = Array.from(
		new Map((boards.data ?? []).map((board) => [board.id, board])).values(),
	);

	return (
		<div className="space-y-6 flex flex-1 flex-col flex-grow max-h-full h-full overflow-y-auto md:overflow-visible overflow-x-hidden">
			<div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6 pb-6">
				{app &&
					uniqueBoards.map((board) => (
						<BoardCard
							key={board.id}
							board={board}
							app={app}
							boardsQuery={boards}
						/>
					))}
			</div>
		</div>
	);
}

function BoardsSkeleton() {
	return (
		<div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6 pb-6">
			{[...Array(6)].map((_, i) => (
				<Card key={`${i}-skeleton`} className="border-0 shadow-md">
					<CardHeader>
						<Skeleton className="h-6 w-3/4" />
						<Skeleton className="h-4 w-1/2" />
					</CardHeader>
					<CardContent>
						<Skeleton className="h-20 w-full" />
					</CardContent>
				</Card>
			))}
		</div>
	);
}

function EmptyBoards({
	setBoardCreation,
	boardCreation,
}: Readonly<{
	setBoardCreation: (value: any) => void;
	boardCreation: { open: boolean; name: string; description: string };
}>) {
	return (
		<Card className="border-0 shadow-md bg-gradient-to-br from-muted/50 to-muted/20">
			<CardContent className="flex flex-col items-center justify-center py-16">
				<WorkflowIcon className="h-16 w-16 text-muted-foreground/50 mb-4" />
				<h3 className="text-xl font-semibold mb-2">No boards yet</h3>
				<p className="text-muted-foreground text-center mb-6 max-w-md">
					Create your first flow to start building amazing automations
				</p>
				<Button
					onClick={() => setBoardCreation({ ...boardCreation, open: true })}
					className="gap-2"
				>
					<PlusCircleIcon className="h-4 w-4" />
					Create Your First Board
				</Button>
			</CardContent>
		</Card>
	);
}

function BoardCard({
	board,
	app,
	boardsQuery,
}: Readonly<{
	board: IBoard;
	app: IApp;
	boardsQuery: UseQueryResult<IBoard[]>;
}>) {
	const backend = useBackend();
	const router = useRouter();
	const pages = useInvoke(backend.pageState.getPages, backend.pageState, [
		app.id,
		board.id,
	]);
	const handleBoardClick = async () => {
		await invoke("get_app_board", {
			appId: app.id,
			boardId: board.id,
			pushToRegistry: true,
		});
		router.push(`/flow?id=${board.id}&app=${app.id}`);
	};

	const handleDeleteBoard = async () => {
		await backend.boardState.deleteBoard(app.id, board.id);
		await boardsQuery.refetch();
	};

	return (
		<BubbleActions
			actions={[
				{
					id: "open",
					label: "Open Board",
					icon: <ExternalLink className="h-4 w-4 text-foreground" />,
					onClick: () => {
						handleBoardClick();
					},
				},
				{
					id: "delete",
					label: "Delete Board",
					icon: <Trash2 className="h-4 w-4 text-foreground" />,
					variant: "destructive",
					onClick: () => {
						handleDeleteBoard();
					},
				},
			]}
			side="top"
			align="end"
		>
			<Card
				title={board.id}
				className="relative group border-0 shadow-md hover:shadow-xl transition-all duration-300 cursor-pointer bg-gradient-to-br from-card to-card/80 hover:from-card/90 hover:to-card/70"
			>
				<CardHeader className="pb-3">
					<div className="flex items-start justify-between">
						<div className="flex items-center space-x-3 flex-1">
							<div className="p-2 rounded-lg bg-primary/10 group-hover:bg-primary/20 transition-colors">
								<WorkflowIcon className="h-5 w-5 text-primary" />
							</div>
							<div className="flex-1">
								<CardTitle className="text-lg group-hover:text-primary transition-colors">
									{board.name}
								</CardTitle>
								<div className="flex items-center gap-2 mt-1">
									<Badge variant="secondary" className="text-xs">
										{board.stage}
									</Badge>
									<Badge variant="outline" className="text-xs">
										{board.log_level}
									</Badge>
								</div>
							</div>
						</div>
					</div>
				</CardHeader>
				<CardContent className="space-y-4">
					<CardDescription className="text-sm leading-relaxed min-h-[2.5rem]">
						{board.description === "" ? (
							<span className="italic text-muted-foreground/70">
								No description provided
							</span>
						) : (
							board.description
						)}
					</CardDescription>

					<div className="flex items-center justify-between pt-2 border-t">
						<div className="flex items-center space-x-4 text-sm text-muted-foreground">
							<div className="flex items-center space-x-1">
								<SquareMousePointerIcon className="h-3 w-3" />
								<span>{Object.keys(board.nodes).length}</span>
							</div>
							<div className="flex items-center space-x-1">
								<VariableIcon className="h-3 w-3" />
								<span>{Object.keys(board.variables).length}</span>
							</div>
							<div className="flex items-center space-x-1">
								<FileText className="h-3 w-3" />
								<span>{pages.data?.length ?? 0}</span>
							</div>
						</div>
						<div className="flex items-center space-x-1 text-xs text-muted-foreground">
							<Calendar className="h-3 w-3" />
							<span>{formatRelativeTime(board.updated_at)}</span>
						</div>
					</div>
				</CardContent>
				<button
					className="absolute inset-0 rounded-lg"
					onClick={handleBoardClick}
				/>
			</Card>
		</BubbleActions>
	);
}
