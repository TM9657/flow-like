"use client";
import type { UseQueryResult } from "@tanstack/react-query";
import { useInvoke } from "@tm9657/flow-like-ui";
import type { Bit } from "@tm9657/flow-like-ui";
import {
	BentoGrid,
	BentoGridItem,
} from "@tm9657/flow-like-ui/components/ui/bento-grid";
import { BitCard } from "@tm9657/flow-like-ui/components/ui/bit-card";
import { Skeleton } from "@tm9657/flow-like-ui/components/ui/skeleton";
import type { ISettingsProfile } from "@tm9657/flow-like-ui/types";
import { useEffect, useState } from "react";

let counter = 0;

export default function Page() {
	const profile: UseQueryResult<ISettingsProfile> = useInvoke(
		"get_current_profile",
		{},
	);
	const embeddingModels: UseQueryResult<Bit[]> = useInvoke(
		"get_bits_by_category",
		{ bitType: "Embedding" },
		["Embedding", profile.data?.hub_profile.id ?? ""],
		typeof profile.data !== "undefined",
	);
	const imageEmbeddingModels: UseQueryResult<Bit[]> = useInvoke(
		"get_bits_by_category",
		{ bitType: "ImageEmbedding" },
		["ImageEmbedding", profile.data?.hub_profile.id ?? ""],
		typeof profile.data !== "undefined",
	);

	const [blacklist, setBlacklist] = useState(new Set<string>());

	async function imageBlacklist() {
		if (!(embeddingModels.data && imageEmbeddingModels.data)) return;

		const dependencies = await Promise.all(
			imageEmbeddingModels.data.map((bit) => bit.fetchDependencies()),
		);
		const blacklist = new Set<string>(
			dependencies.flatMap((dep) =>
				dep.bits
					.filter((bit) => bit.type !== "ImageEmbedding")
					.map((bit) => bit.id),
			),
		);
		setBlacklist(blacklist);
	}

	useEffect(() => {
		if (!(embeddingModels.data && imageEmbeddingModels.data)) return;
		imageBlacklist();
	}, [embeddingModels.data, imageEmbeddingModels.data]);

	return (
		<main className="justify-start flex flex-row items-start w-full pr-4">
			<div className="mr-6 invisible-scroll">
				<h1 className="scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl">
					Embedding Models
				</h1>
				<br />
				<div
					className={`max-h-[calc(100dvh-3rem)] overflow-auto invisible-scroll w-full`}
				>
					{!(embeddingModels.data && imageEmbeddingModels.data) && (
						<BentoGrid className="mx-auto cursor-pointer w-full">
							{[...Array(10)].map((item, i) => {
								if (i === 0) counter = 0;
								const wide = counter === 3 || counter === 6;
								if (counter === 6) counter = 0;
								else counter += 1;
								return (
									<BentoGridItem
										className={`h-full w-full ${wide ? "md:col-span-2" : ""}`}
										key={`${i}__skeleton`}
										title={
											<div className="flex flex-row items-center">
												<Skeleton className="h-4 w-[200px]" />{" "}
												<Skeleton className="h-4 ml-2 w-[100px]" />
											</div>
										}
										description={
											<Skeleton className="h-[125px] w-full rounded-xl" />
										}
										header={
											<div>
												<div className="flex flex-row items-center">
													<Skeleton className="h-14 w-14 rounded-full" />
													<Skeleton className="h-4 w-[40px] ml-2" />
													<Skeleton className="h-4 w-[40px] ml-2" />
												</div>
											</div>
										}
										icon={<Skeleton className="h-4 w-[200px]" />}
									/>
								);
							})}
						</BentoGrid>
					)}
					{embeddingModels.data && imageEmbeddingModels.data && (
						<BentoGrid className="mx-auto cursor-pointer w-full pb-20">
							{embeddingModels.data &&
								imageEmbeddingModels.data &&
								[...embeddingModels.data, ...imageEmbeddingModels.data]
									.sort((a, b) => Date.parse(b.updated) - Date.parse(a.updated))
									.filter((bit) => !blacklist.has(bit.id))
									.map((bit, i) => {
										if (i === 0) counter = 0;
										const wide = counter === 3 || counter === 6;
										if (counter === 6) counter = 0;
										else counter += 1;
										return <BitCard key={bit.id} bit={bit} wide={wide} />;
									})}
						</BentoGrid>
					)}
				</div>
				<br />
			</div>
		</main>
	);
}
