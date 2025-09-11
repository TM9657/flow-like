"use client";

import { Button, Card, CardContent, CardHeader } from "@tm9657/flow-like-ui";
import { PartyPopper } from "lucide-react";
import { useRouter } from "next/navigation";
import Crossfire from "react-canvas-confetti/dist/presets/crossfire";

export default function DonePage() {
	const router = useRouter();

	return (
		<main>
			<Crossfire autorun={{ speed: 1 }} />
			<Card>
				<CardContent className="w-full max-w-screen-md min-w-lg">
					<CardHeader>
						<h1>🎉 Congratulations!</h1>
						<p>You have successfully completed the onboarding process.</p>
					</CardHeader>
					<Button
						className="gap-2 w-full mt-6"
						onClick={() => {
							localStorage.setItem("onboarding-done", "true");
							router.push("/");
						}}
					>
						<PartyPopper />
						Finish Setup
					</Button>
				</CardContent>
			</Card>
		</main>
	);
}
