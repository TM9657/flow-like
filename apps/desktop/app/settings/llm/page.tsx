"use client"

import { ISettingsProfile } from "@tm9657/flow-like-ui/types";
import { BentoGrid, BentoGridItem } from "@tm9657/flow-like-ui/components/ui/bento-grid";
import { BitCard } from "@tm9657/flow-like-ui/components/ui/bit-card";
import { Button } from "@tm9657/flow-like-ui";
import { DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuContent, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@tm9657/flow-like-ui/components/ui/dropdown-menu";
import { Input } from "@tm9657/flow-like-ui";
import { Skeleton } from "@tm9657/flow-like-ui/components/ui/skeleton";
import { useInvoke } from "@tm9657/flow-like-ui";
import { Bit } from "@tm9657/flow-like-ui";
import { UseQueryResult } from "@tanstack/react-query";
import { ListFilter, Search } from "lucide-react";
import MiniSearch from 'minisearch';
import { useState } from "react";

let counter = 0;

export default function SettingsPage() {
    const profile : UseQueryResult<ISettingsProfile> = useInvoke("get_current_profile", {})
    const llms : UseQueryResult<Bit[]> = useInvoke("get_bits_by_category", {bitType: "Llm"}, ["Llm", profile.data?.hub_profile.id ?? ""], typeof profile.data !== "undefined")
    const vlms : UseQueryResult<Bit[]> = useInvoke("get_bits_by_category", {bitType: "Vlm"}, ["Vlm", profile.data?.hub_profile.id ?? ""], typeof profile.data !== "undefined")
    const [searchFilter, setSearchFilter] = useState<{ search: string, index: MiniSearch, results: any[], filter: string, filters: string[] }>({
        search: "", index: new MiniSearch({
            fields: ['categories', 'description', "file_name", "id", "name", "use"], // fields to index for full-text search
            storeFields: ['id']
        }), results: [], filter: "all", filters: []
    })

    return <main className="justify-start flex min-h-dvh flex-col items-center w-full pr-4">
        <div className="flex flex-row items-center justify-between w-full max-w-screen-2xl">
            <h1 className="scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl">Large Language Models</h1>
            <div className="flex flex-row items-center ml-2">
                <div className="relative flex flex-row items-center">
                    <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                    <Input
                        type="search"
                        placeholder="Search..."
                        onChange={(e) => {
                            setSearchFilter((old) => ({ ...old, search: e.target.value, results: old.index.search(e.target.value, { fuzzy: 0.2 }).map((res: any) => res.id) }))
                        }}
                        className="w-full rounded-lg bg-background pl-8 md:w-[200px] lg:w-[336px] focus-visible:ring-0 focus-visible:ring-offset-0"
                    />
                </div>
                <DropdownMenu>
                    <DropdownMenuTrigger asChild className="focus-visible:ring-0 focus-visible:ring-offset-0 mx-2">
                        <Button variant="outline" size="sm" className="h-8 gap-1">
                            <ListFilter className="h-3.5 w-3.5" />
                            <span className="sr-only sm:not-sr-only sm:whitespace-nowrap">
                                Filter
                            </span>
                        </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                        <DropdownMenuLabel>Filter by</DropdownMenuLabel>
                        <DropdownMenuSeparator />
                        <DropdownMenuCheckboxItem checked={searchFilter.filter === "all"} onClick={() => {
                            setSearchFilter((old) => ({ ...old, filter: "all" }))
                        }}>
                            all
                        </DropdownMenuCheckboxItem>
                        {searchFilter.filters.map(filter => (<DropdownMenuCheckboxItem checked={searchFilter.filter === filter} key={filter} onClick={() => {
                            setSearchFilter((old) => ({ ...old, filter }))
                        }}>{filter}</DropdownMenuCheckboxItem>))}
                    </DropdownMenuContent>
                </DropdownMenu>
            </div>

        </div>
        <br/>
        <div className={`max-h-[calc(100dvh-3rem)] overflow-auto invisible-scroll w-full`}>
            {(llms.isLoading || vlms.isLoading) &&
                <BentoGrid className="mx-auto cursor-pointer w-full">
                    {[...Array(10)].map((item, i) => {
                        if (i === 0) counter = 0;
                        let wide = counter === 3 || counter === 6;
                        if (counter === 6) counter = 0;
                        else counter += 1;
                        return <BentoGridItem
                            className={`h-full w-full ${wide ? "md:col-span-2" : ""}`}
                            key={i + "__skeleton"}
                            title={<div className="flex flex-row items-center"><Skeleton className="h-4 w-[200px]" /> <Skeleton className="h-4 ml-2 w-[100px]" /></div>}
                            description={<Skeleton className="h-[125px] w-full rounded-xl" />}
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
                    })}
                </BentoGrid>
            }
            {!(llms.isLoading || vlms.isLoading) &&
                <BentoGrid className="mx-auto cursor-pointer w-full pb-20">
                    {(llms.data && vlms.data) && [...llms.data, ...vlms.data].sort((a,b) => Date.parse(b.updated) - Date.parse(a.updated)).map((bit, i) => {
                        if (i === 0) counter = 0;
                        let wide = counter === 3 || counter === 6;
                        if (counter === 6) counter = 0;
                        else counter += 1;
                        return <BitCard key={bit.id} bit={bit} wide={wide} />
                    })}
                </BentoGrid>
            }
        </div>
    </main >
}