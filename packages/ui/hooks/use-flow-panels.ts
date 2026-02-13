import type { RefObject } from "react";
import type { ImperativePanelHandle } from "react-resizable-panels";

interface UseFlowPanelsProps {
	varPanelRef: RefObject<ImperativePanelHandle | null>;
	runsPanelRef: RefObject<ImperativePanelHandle | null>;
	logPanelRef: RefObject<ImperativePanelHandle | null>;
	setVarsOpen: (value: boolean | ((v: boolean) => boolean)) => void;
	setRunsOpen: (value: boolean | ((v: boolean) => boolean)) => void;
	setLogsOpen: (value: boolean | ((v: boolean) => boolean)) => void;
}

export function useFlowPanels({
	varPanelRef,
	runsPanelRef,
	logPanelRef,
	setVarsOpen,
	setRunsOpen,
	setLogsOpen,
}: UseFlowPanelsProps) {
	const toggleVars = () => {
		if (
			typeof window !== "undefined" &&
			window.matchMedia("(max-width: 767px)").matches
		) {
			setVarsOpen((v) => !v);
			return;
		}
		if (!varPanelRef.current) return;
		const isCollapsed = varPanelRef.current.isCollapsed();
		isCollapsed ? varPanelRef.current.expand() : varPanelRef.current.collapse();

		if (!isCollapsed) return;

		const size = varPanelRef.current.getSize();
		if (size < 10) varPanelRef.current.resize(20);
	};

	const toggleRunHistory = () => {
		if (
			typeof window !== "undefined" &&
			window.matchMedia("(max-width: 767px)").matches
		) {
			setRunsOpen((v) => !v);
			return;
		}
		if (!runsPanelRef.current) return;
		const isCollapsed = runsPanelRef.current.isCollapsed();
		isCollapsed
			? runsPanelRef.current.expand()
			: runsPanelRef.current.collapse();

		if (!isCollapsed) return;

		const size = runsPanelRef.current.getSize();
		if (size < 10) runsPanelRef.current.resize(30);
	};

	const toggleLogs = () => {
		if (
			typeof window !== "undefined" &&
			window.matchMedia("(max-width: 767px)").matches
		) {
			setLogsOpen((v) => !v);
			return;
		}
		if (!logPanelRef.current) return;
		const isCollapsed = logPanelRef.current.isCollapsed();
		isCollapsed ? logPanelRef.current.expand() : logPanelRef.current.collapse();

		if (!isCollapsed) return;

		const size = logPanelRef.current.getSize();
		if (size < 10) logPanelRef.current.resize(20);
	};

	return {
		toggleVars,
		toggleRunHistory,
		toggleLogs,
	};
}
