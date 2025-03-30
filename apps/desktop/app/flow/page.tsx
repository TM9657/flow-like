"use client";
import { FlowWrapper } from "@tm9657/flow-like-ui/components/flow/flow-wrapper";
import "@xyflow/react/dist/style.css";
import { useSearchParams } from "next/navigation";
import { useMemo } from "react";

export default function FlowEditPage() {
	const searchParams = useSearchParams();
	const { boardId, appId } = useMemo(() => {
		const boardId = searchParams.get("id") ?? "";
		const appId = searchParams.get("app") ?? "";
		return { boardId, appId };
	}, [searchParams]);

	if (boardId === "") return <p>Board not found...</p>;
	return <FlowWrapper boardId={boardId} appId={appId} />;
}
