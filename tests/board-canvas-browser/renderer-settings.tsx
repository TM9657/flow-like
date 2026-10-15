"use client";

import { useEffect, useState } from "react";
import { BoardRendererCard } from "../../packages/ui/components/settings/board-renderer-card";
import { setBoardRendererPreference } from "../../packages/ui/hooks/use-board-renderer";

/** Keep renderer query overrides in fixtures; production ignores them. */
export function initializeFixtureRenderer() {
	const renderer = new URLSearchParams(window.location.search).get(
		"boardRenderer",
	);
	if (renderer === "egui" || renderer === "react-flow")
		setBoardRendererPreference(renderer);
}

export function useFixtureRendererReady() {
	const [ready, setReady] = useState(false);
	useEffect(() => {
		initializeFixtureRenderer();
		setReady(true);
	}, []);
	return ready;
}

export function FixtureRendererSettings() {
	return (
		<details
			data-testid="fixture-renderer-settings"
			style={{ position: "relative" }}
		>
			<summary style={{ cursor: "pointer" }}>Fixture settings</summary>
			<div
				style={{
					position: "fixed",
					top: 64,
					right: 16,
					width: 380,
					zIndex: 1000,
					paddingTop: 8,
				}}
			>
				<BoardRendererCard />
			</div>
		</details>
	);
}
