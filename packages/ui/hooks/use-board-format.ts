"use client";

import { useEffect, useState } from "react";
import {
	LEGACY_BOARD_FORMAT_VERSION,
	boardFormatVersion,
} from "../lib/board-format";
import { useBackend } from "../state/backend-state";

/** The board format supported by the current app's backend. */
export function useBoardFormat(appId?: string): number {
	const { boardState } = useBackend();
	const [supported, setSupported] = useState<{
		appId?: string;
		boardState?: typeof boardState;
		version: number;
	}>({ version: LEGACY_BOARD_FORMAT_VERSION });
	useEffect(() => {
		let active = true;
		setSupported({ appId, boardState, version: LEGACY_BOARD_FORMAT_VERSION });
		if (appId && boardState.getBoardFormat) {
			void boardState.getBoardFormat(appId).then(
				(result) => {
					if (active)
						setSupported({
							appId,
							boardState,
							version: boardFormatVersion(result?.board_format_version),
						});
				},
				() => {
					if (active)
						setSupported({
							appId,
							boardState,
							version: LEGACY_BOARD_FORMAT_VERSION,
						});
				},
			);
		}
		return () => {
			active = false;
		};
	}, [appId, boardState]);
	return supported.appId === appId && supported.boardState === boardState
		? supported.version
		: LEGACY_BOARD_FORMAT_VERSION;
}
