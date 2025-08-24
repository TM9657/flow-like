import {
	LoadingScreen,
	QueryClient,
	createIDBPersister,
} from "@tm9657/flow-like-ui";
import { Suspense, lazy } from "react";

const PersistQueryClientProvider = lazy(() =>
	import("@tm9657/flow-like-ui").then((module) => ({
		default: module.PersistQueryClientProvider,
	})),
);

const Board = lazy(() => import("./board"));
const persister = createIDBPersister();
const queryClient = new QueryClient();
export default function BoardWrapper({
	nodes,
	edges,
}: Readonly<{ nodes: any[]; edges: any[] }>) {
	return (
		<Suspense fallback={<LoadingScreen />}>
			<PersistQueryClientProvider
				client={queryClient}
				persistOptions={{
					persister,
				}}
			>
				<div className="w-full h-full">
					<Board nodes={nodes} edges={edges} />
				</div>
			</PersistQueryClientProvider>
		</Suspense>
	);
}
