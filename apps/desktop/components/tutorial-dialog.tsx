import {
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle
} from "@tm9657/flow-like-ui";
import { useEffect, useState } from "react";

export function TutorialDialog() {
	const [showTutorial, setShowTutorial] = useState(true);

	function checkTutorial() {
		const hasFinishedTutorial = localStorage.getItem("tutorial-done");
		setShowTutorial(hasFinishedTutorial !== "true");
	}

	useEffect(() => {
		checkTutorial();
	}, []);

	return (
		<Dialog open={showTutorial} onOpenChange={(open) => setShowTutorial(open)}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						Welcome to <span className="text-primary">Flow</span> Like
					</DialogTitle>
					<DialogDescription>
						Welcome, should we guide you through some onboarding?
					</DialogDescription>
				</DialogHeader>
				<div className="w-full flex flex-col items-center justify-center gap-4 border rounded-lg bg-card shadow-sm">
					<img src="/app-logo.webp" alt="logo" className="w-28 h-28 mt-4" />
					<div className="flex flex-col items-center justify-center mb-4">
						<h3>
							<span className="text-primary">Flow</span> Like
						</h3>
						<p className="text-center leading-5">
							Your solution for scalable Software.
						</p>
					</div>
				</div>
				<DialogFooter>
					<Button
						variant={"outline"}
						onClick={() => {
							localStorage.setItem("tutorial-done", "true");
							setShowTutorial(false);
						}}
					>
						Skip
					</Button>
					<Button>Next</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
