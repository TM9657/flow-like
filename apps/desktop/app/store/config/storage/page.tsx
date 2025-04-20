"use client";
import { invoke } from "@tauri-apps/api/core";
import {
	Breadcrumb,
	BreadcrumbItem,
	BreadcrumbLink,
	BreadcrumbList,
	BreadcrumbPage,
	BreadcrumbSeparator,
	Button
} from "@tm9657/flow-like-ui";
import { useSearchParams } from "next/navigation";
import { useTauriInvoke } from "../../../../components/useInvoke";

export default function Page() {
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const prefix = searchParams.get("prefix") ?? "";
	const files = useTauriInvoke<any>("storage_list", { appId: id, prefix }, [id ?? "", prefix], typeof id === "string");

	return <main>
		<div className="flex flex-row items-end justify-between">
			<Breadcrumbs />
			<div className="flex flex-row items-center gap-2">
				<Button variant={"outline"} onClick={() => {
					invoke("storage_add", {
						appId: id,
						prefix: prefix,
						folder: false
					})
				}}>
					Upload Files
				</Button>
				<Button variant={"outline"} onClick={() => {
					invoke("storage_add", {
						appId: id,
						prefix: prefix,
						folder: true
					})
				}}>
					Upload Folder
				</Button>
			</div>
		</div>
		{files.data && <p>{JSON.stringify(files.data)}</p>}
	</main>
}

function Breadcrumbs() {
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const prefix = searchParams.get("prefix") ?? "";

	return <Breadcrumb>
		<BreadcrumbList>
			<BreadcrumbItem>
				<BreadcrumbLink href={`/store/config/storage?id=${id}`}>Uploads</BreadcrumbLink>
			</BreadcrumbItem>
			{decodeURIComponent(prefix).split("/").slice(0, -1).map((part, index) => (
				<>
					<BreadcrumbSeparator />
					<BreadcrumbItem>
						<BreadcrumbLink href={`/store/config/storage?id=${id}&prefix=${encodeURIComponent(prefix.split("/").slice(0, index).join("/"))}`}>{part}</BreadcrumbLink>
					</BreadcrumbItem>
				</>
			))}
			{decodeURIComponent(prefix).split("/").pop() && <><BreadcrumbSeparator />
				<BreadcrumbItem>
					<BreadcrumbPage>{decodeURIComponent(prefix).split("/").pop()}</BreadcrumbPage>
				</BreadcrumbItem></>}
		</BreadcrumbList>
	</Breadcrumb>
}