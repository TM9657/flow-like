"use client";
import { AppStylesDocument } from "@flow-like/flow-like-ui";
import { useSearchParams } from "next/navigation";

export default function Page() {
	const searchParams = useSearchParams();
	const id = searchParams.get("id");

	if (!id) return null;

	return (
		<div className="flex h-full min-h-0 flex-col">
			<AppStylesDocument appId={id} className="flex-1" />
		</div>
	);
}
