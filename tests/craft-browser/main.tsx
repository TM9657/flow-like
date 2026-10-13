import { Suspense, lazy, useState } from "react";
import { createRoot } from "react-dom/client";
import { craftViewer } from "../../packages/ui/components/storage/craft-viewers";
import {
	Dialog,
	DialogContent,
	DialogTitle,
} from "../../packages/ui/components/ui/dialog";
const CraftDocument = lazy(() =>
	import("../../packages/ui/components/storage/craft-document").then(
		(module) => ({ default: module.CraftDocument }),
	),
);
function Harness() {
	const [name, setName] = useState("");
	const viewer = craftViewer(name);
	return (
		<>
			<button type="button" id="pdf" onClick={() => setName("sample.pdf")}>
				Open PDF
			</button>
			<button type="button" id="audio" onClick={() => setName("sample.wav")}>
				Open audio
			</button>
			<Dialog
				open={!!name}
				onOpenChange={(open) => {
					if (!open) setName("");
				}}
			>
				{viewer && (
					<DialogContent
						className="flex h-[calc(100dvh-2rem)] max-w-[calc(100vw-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-[calc(100vw-2rem)] [&>[data-slot=dialog-close]]:top-5"
						aria-describedby={undefined}
					>
						<Suspense fallback={<DialogTitle>Loading...</DialogTitle>}>
							<CraftDocument
								key={name}
								viewer={viewer}
								url={`/${name}`}
								filename={name}
								title={<DialogTitle>{name}</DialogTitle>}
								editable
								onSave={async (file) => {
									const response = await fetch(`/${name}`, {
										method: "PUT",
										body: file,
									});
									if (!response.ok) throw new Error("Saving failed");
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
if (!root) throw new Error("Missing root");
createRoot(root).render(<Harness />);
