"use client";

import { invoke } from "@tauri-apps/api/core";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	IBoard,
	IVariable,
	IVault,
	Label,
	useInvoke,
	VariablesMenuEdit,
} from "@tm9657/flow-like-ui";
import { useSearchParams } from "next/navigation";
import { useMemo } from "react";

export default function Id() {
	const searchParams = useSearchParams();
	const id = searchParams.get("id");
	const vault = useInvoke<IVault | undefined>(
		"get_vault",
		{ vaultId: id },
		[id ?? ""],
		typeof id === "string",
	);
	const boards = useInvoke<IBoard[]>(
		"get_vault_boards",
		{ vaultId: id },
		[id ?? ""],
		typeof id === "string",
	);
	const variables = useMemo(() => {
		const vars = (boards.data ?? [])
			.map((board) => {
				return [
					board,
					Object.values(board.variables)
						.filter((variable) => variable.exposed && variable.editable)
						.sort((a, b) => a.name.localeCompare(b.name)),
				];
			})
			.filter(([boards, node]) => node.length > 0)
			.sort((a, b) =>
				(a[0] as IBoard).name.localeCompare((b[0] as IBoard).name),
			) as [IBoard, IVariable[]][];

		return vars;
	}, [boards.data]);

	async function upsertVariable(board: IBoard, variable: IVariable) {
		await invoke("get_vault_board", {
			vaultId: id,
			boardId: board.id,
			pushToRegistry: true,
		});
		await invoke("upsert_variable", { boardId: board.id, variable });
		await boards.refetch();
		await vault.refetch();
	}

	if (variables.length === 0) {
		return (
			<main className="justify-start flex flex-col items-start w-full flex-1 max-h-full overflow-y-auto flex-grow gap-4">
				<div className="border p-4 rounded-lg bg-card w-full">
					<h4>✅ No Configuration necessary!</h4>
					<p className="mt-1 text-muted-foreground">You are ready to go 🚀</p>
				</div>
			</main>
		);
	}

	return (
		<main className="justify-start flex flex-col items-start w-full flex-1 max-h-full overflow-y-auto flex-grow gap-4">
			<h2>Configuration</h2>
			<Accordion type="multiple" className="w-full gap-2 flex flex-col">
				{variables.map(([board, vars]) => (
					<AccordionItem
						className="rounded-md px-2 w-full bg-background/50 border"
						value={board.id}
						key={board.id}
					>
						<AccordionTrigger>
							<h4>{board.name}</h4>
						</AccordionTrigger>
						{vars.map((variable) => (
							<AccordionContent
								key={variable.id}
								className="px-2 grid w-full max-w-sm items-center gap-1.5"
							>
								<Label className="">{variable.name}</Label>
								<VariablesMenuEdit
									variable={variable}
									updateVariable={async (variable) => {
										await upsertVariable(board, variable);
									}}
								/>
							</AccordionContent>
						))}
					</AccordionItem>
				))}
			</Accordion>
		</main>
	);
}
