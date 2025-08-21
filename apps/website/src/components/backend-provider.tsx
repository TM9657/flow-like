import {
	EmptyAIState,
	EmptyAppState,
	EmptyBitState,
	EmptyBoardState,
	EmptyEventState,
	EmptyHelperState,
	EmptyRoleState,
	EmptyStorageState,
	EmptyTeamState,
	EmptyTemplateState,
	EmptyUserState,
	type IAIState,
	type IAppState,
	type IBackendState,
	type IBitState,
	type IBoardState,
	type IEventState,
	type IHelperState,
	type IRoleState,
	type IStorageState,
	type ITeamState,
	type ITemplateState,
	type IUserState,
	LoadingScreen,
	ThemeProvider,
	useBackendStore,
} from "@tm9657/flow-like-ui";
import { Suspense, lazy, useEffect, useState } from "react";

const BoardWrapper = lazy(() => import("./board-wrapper"));
export class EmptyBackend implements IBackendState {
	aiState: IAIState = new EmptyAIState();
	appState: IAppState = new EmptyAppState();
	bitState: IBitState = new EmptyBitState();
	boardState: IBoardState = new EmptyBoardState();
	eventState: IEventState = new EmptyEventState();
	helperState: IHelperState = new EmptyHelperState();
	roleState: IRoleState = new EmptyRoleState();
	storageState: IStorageState = new EmptyStorageState();
	teamState: ITeamState = new EmptyTeamState();
	templateState: ITemplateState = new EmptyTemplateState();
	userState: IUserState = new EmptyUserState();
}

export function EmptyBackendProvider({ data }: Readonly<{ data: string }>) {
	const [nodes, setNodes] = useState<any[]>([]);
	const [edges, setEdges] = useState<any[]>([]);
	const [loaded, setLoaded] = useState(false);
	const { setBackend } = useBackendStore();

	useEffect(() => {
		(async () => {
			const response = await fetch(data);
			const json = await response.json();
			const { nodes, edges } = json;
			setNodes(nodes);
			setEdges(edges);
			const backend = new EmptyBackend();
			setBackend(backend);
			setLoaded(true);
		})();
	}, []);

	if (!loaded) {
		return (
			<ThemeProvider
				attribute="class"
				defaultTheme="dark"
				enableSystem
				disableTransitionOnChange
				>
				<LoadingScreen className="absolute top-0 left-0 right-0 bottom-0" />;
			</ThemeProvider>
		);
	}

	return (
		<ThemeProvider
			attribute="class"
			defaultTheme="dark"
			enableSystem
			disableTransitionOnChange
		>
			<Suspense fallback={<LoadingScreen className="absolute top-0 left-0 right-0 bottom-0" />}>
			<div className="max-h-36">
				<BoardWrapper nodes={nodes} edges={edges} />
			</div>
			</Suspense>
		</ThemeProvider>
	);
}
