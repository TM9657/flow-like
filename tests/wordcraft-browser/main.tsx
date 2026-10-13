import { Suspense, lazy, useState } from "react";
import { createRoot } from "react-dom/client";
import { useStorageDocxHistory } from "../../packages/ui/components/storage/storage-docx-history";
import {
	Dialog,
	DialogContent,
	DialogTitle,
} from "../../packages/ui/components/ui/dialog";

const WordcraftDocument = lazy(() =>
	import("../../packages/ui/components/storage/wordcraft-document").then(
		(module) => ({ default: module.WordcraftDocument }),
	),
);

declare global {
	interface Window {
		savedDocx?: number[];
		documentDirty?: boolean;
		documentBusy?: boolean;
		discardPrompts?: number;
	}
}

function Harness() {
	const [url, setUrl] = useState("");
	const { openHistory, closeHistory } = useStorageDocxHistory(
		() => {
			if (window.documentBusy) return false;
			if (!window.documentDirty) return true;
			window.discardPrompts = (window.discardPrompts ?? 0) + 1;
			return window.confirm("Discard unsaved changes?");
		},
		() => setUrl(""),
	);
	return (
		<>
			<button
				type="button"
				id="open"
				onClick={() => {
					openHistory();
					setUrl("/sample.docx");
				}}
			>
				Open document
			</button>
			<button
				type="button"
				id="reopen"
				onClick={() => {
					openHistory();
					setUrl("/sample.docx");
				}}
			>
				Reopen saved document
			</button>
			<Dialog
				open={!!url}
				onOpenChange={(open) => {
					if (open || window.documentBusy) return;
					if (
						window.documentDirty &&
						!window.confirm("Discard unsaved changes?")
					)
						return;
					void closeHistory().then((closed) => {
						if (closed) setUrl("");
					});
				}}
			>
				{url && (
					<DialogContent
						className="flex h-[calc(100dvh-2rem)] max-w-[calc(100vw-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-[calc(100vw-2rem)] [&>[data-slot=dialog-close]]:top-5"
						aria-describedby={undefined}
					>
						<Suspense
							fallback={<DialogTitle>Loading Flows Word...</DialogTitle>}
						>
							<WordcraftDocument
								url={url}
								filename="sample.docx"
								title={
									<DialogTitle className="min-w-0 truncate text-base">
										sample.docx
									</DialogTitle>
								}
								editable
								onDirtyChange={(dirty) => {
									window.documentDirty = dirty;
								}}
								onBusyChange={(busy) => {
									window.documentBusy = busy;
								}}
								onSave={async (file) => {
									const response = await fetch("/sample.docx", {
										method: "PUT",
										body: file,
									});
									if (!response.ok) throw new Error("Saving failed");
									window.savedDocx = Array.from(
										new Uint8Array(await file.arrayBuffer()),
									);
								}}
							/>
						</Suspense>
					</DialogContent>
				)}
			</Dialog>
		</>
	);
}

const root = document.getElementById("app");
if (!root) throw new Error("Missing harness root");
createRoot(root).render(<Harness />);
