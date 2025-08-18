"use client";

import type { IBit, UseQueryResult } from "@tm9657/flow-like-ui";
import {
    Bit,
    Button,
    IBitTypes,
    Input,
    useBackend,
    useInvoke,
    useMiniSearch,
} from "@tm9657/flow-like-ui";
import { Badge } from "@tm9657/flow-like-ui/components/ui/badge";
import {
    BentoGrid,
    BentoGridItem,
} from "@tm9657/flow-like-ui/components/ui/bento-grid";
import { BitCard } from "@tm9657/flow-like-ui/components/ui/bit-card";
import { Card, CardContent } from "@tm9657/flow-like-ui/components/ui/card";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@tm9657/flow-like-ui/components/ui/dropdown-menu";
import { Skeleton } from "@tm9657/flow-like-ui/components/ui/skeleton";
import type { ISettingsProfile } from "@tm9657/flow-like-ui/types";
import {
    Bot,
    Database,
    Eye,
    Filter,
    Image,
    ListFilter,
    Loader2,
    Search,
    Sparkles,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTauriInvoke } from "../../../components/useInvoke";

function bitTypeToFilter(bitType: IBitTypes) {
    switch (bitType) {
        case IBitTypes.Llm:
            return "LLM";
        case IBitTypes.Vlm:
            return "Vision LLM";
        case IBitTypes.Embedding:
            return "Embedding";
        case IBitTypes.ImageEmbedding:
            return "Image Embedding";
        default:
            return "Unknown";
    }
}

function getFilterIcon(filter: string) {
    switch (filter) {
        case "LLM":
            return <Bot className="h-3 w-3" />;
        case "Vision LLM":
            return <Eye className="h-3 w-3" />;
        case "Embedding":
            return <Database className="h-3 w-3" />;
        case "Image Embedding":
            return <Image className="h-3 w-3" />;
        case "In Profile":
            return <Sparkles className="h-3 w-3" />;
        case "Downloaded":
            return <Bot className="h-3 w-3" />;
        default:
            return <Filter className="h-3 w-3" />;
    }
}

// Layout helper: fill rows with capacity=3 (md). Prefer pattern [2,1,1] but
// never place a wide card when only 1 slot remains in the current row.
function computeWideFlags(count: number): boolean[] {
    const flags: boolean[] = [];
    let rem = 3;
    for (let i = 0; i < count; i += 1) {
        const preferWide = i % 3 === 0; // base pattern
        const wide = preferWide && rem >= 2;
        flags.push(wide);
        rem -= wide ? 2 : 1;
        if (rem === 0) rem = 3;
    }
    return flags;
}

export default function SettingsPage() {
    const backend = useBackend();
    const [searchTerm, setSearchTerm] = useState("");
    const [blacklist, setBlacklist] = useState(new Set<string>());
    const [compactHeader, setCompactHeader] = useState(false);

    const scrollContainerRef = useRef<HTMLDivElement>(null);

    const profile: UseQueryResult<ISettingsProfile> = useTauriInvoke(
        "get_current_profile",
        {},
    );

    const foundBits = useInvoke(
        backend.bitState.searchBits,
        backend.bitState,
        [
            {
                bit_types: [
                    IBitTypes.Llm,
                    IBitTypes.Vlm,
                    IBitTypes.Embedding,
                    IBitTypes.ImageEmbedding,
                ],
            },
        ],
        typeof profile.data !== "undefined",
        [profile.data?.hub_profile.id ?? ""],
    );

    const imageBlacklist = useCallback(async () => {
        if (!foundBits.data) return;

        const dependencies = await Promise.all(
            foundBits.data
                .filter((bit) => bit.type === IBitTypes.ImageEmbedding)
                .map((bit) =>
                    Bit.fromObject(bit).setBackend(backend).fetchDependencies(),
                ),
        );

        const bl = new Set<string>(
            dependencies.flatMap((dep) =>
                dep.bits
                    .filter((bit) => bit.type !== IBitTypes.ImageEmbedding)
                    .map((bit) => bit.id),
            ),
        );
        setBlacklist(bl);
    }, [backend, foundBits.data]);

    const [bits, setBits] = useState<IBit[]>([]);
    const [installedBits, setInstalledBits] = useState<Set<string>>(new Set());

    const [searchFilter, setSearchFilter] = useState<{
        appliedFilter: string[];
        filters: string[];
    }>({
        appliedFilter: ["All"],
        filters: [
            "LLM",
            "Vision LLM",
            "Embedding",
            "Image Embedding",
            "In Profile",
            "Downloaded",
        ],
    });

    const { search, searchResults, addAllAsync, removeAll } = useMiniSearch<any>(
        [],
        {
            fields: [
                "authors",
                "file_name",
                "hub",
                "id",
                "name",
                "long_description",
                "description",
                "type",
            ],
            storeFields: ["id"],
            searchOptions: {
                fuzzy: true,
                boost: {
                    name: 2,
                    type: 1.5,
                    description: 1,
                    long_description: 0.5,
                },
            },
        },
    );

    useEffect(() => {
        const el = scrollContainerRef.current;
        if (!el) return;
        const onScroll = () => setCompactHeader(el.scrollTop > 16);
        onScroll();
        el.addEventListener("scroll", onScroll);
        return () => el.removeEventListener("scroll", onScroll);
    }, []);

    useEffect(() => {
        if (!foundBits.data) return;
        imageBlacklist();
    }, [foundBits.data, imageBlacklist]);

    useEffect(() => {
        if (!foundBits.data || !profile.data) return;

        const checkInstalled = async () => {
            const installedSet = new Set<string>();
            for (const bit of foundBits.data) {
                const isInstalled = await backend.bitState.isBitInstalled(bit);
                if (isInstalled) installedSet.add(bit.id);
            }
            setInstalledBits(installedSet);
        };

        checkInstalled();
        imageBlacklist();
    }, [backend.bitState, foundBits.data, profile.data, imageBlacklist]);

    useEffect(() => {
        if (!foundBits.data || !profile.data) return;

        const profileBitIds = new Set(
            profile.data.hub_profile.bits.map((id) => id.split(":").pop()),
        );

        const allBits = foundBits.data
            ?.filter((bit) => {
                if (blacklist.has(bit.id)) return false;

                const hasProfileFilter =
                    searchFilter.appliedFilter.includes("In Profile");
                const hasDownloadedFilter =
                    searchFilter.appliedFilter.includes("Downloaded");
                const hasAllFilter = searchFilter.appliedFilter.includes("All");

                const typeFilters = searchFilter.appliedFilter.filter(
                    (filter) => !["All", "In Profile", "Downloaded"].includes(filter),
                );

                const typeMatch =
                    hasAllFilter ||
                    typeFilters.length === 0 ||
                    typeFilters.includes(bitTypeToFilter(bit.type));

                if (!hasProfileFilter && !hasDownloadedFilter) return typeMatch;

                const inProfile = profileBitIds.has(bit.id);
                const isDownloaded = installedBits.has(bit.id);

                if (hasProfileFilter && !hasDownloadedFilter) return typeMatch && inProfile;
                if (hasDownloadedFilter && !hasProfileFilter) return typeMatch && isDownloaded;
                if (hasProfileFilter && hasDownloadedFilter)
                    return typeMatch && (inProfile || isDownloaded);

                return typeMatch;
            })
            .sort((a, b) => Date.parse(b.updated) - Date.parse(a.updated));

        removeAll();
        setBits(allBits);
        addAllAsync(
            allBits.map((item) => ({
                ...item,
                name: item.meta?.en?.name,
                long_description: item.meta?.en?.long_description,
                description: item.meta?.en?.description,
            })),
        );
    }, [foundBits.data, blacklist, searchFilter, profile.data, installedBits, addAllAsync, removeAll]);

    const activeFilterCount = searchFilter.appliedFilter.filter(
        (f) => f !== "All",
    ).length;

    const visibleBits: IBit[] = useMemo(
        () => (searchTerm === "" ? bits : ((searchResults ?? []) as IBit[])),
        [bits, searchResults, searchTerm],
    );
    const wideFlags = useMemo(
        () => computeWideFlags(visibleBits.length),
        [visibleBits.length],
    );
    const skeletonWideFlags = useMemo(() => computeWideFlags(6), []);

    return (
        <main className="flex flex-grow h-full max-h-full overflow-hidden flex-col w-full">
            {/* Header Section */}
            <div
                className={`
                    sticky top-0 z-30 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60
                    transition-all duration-200 ease-in-out
                    ${compactHeader ? "shadow-sm" : ""}
                `}
            >
                <div
                    className={`max-w-screen-xl mx-auto ${compactHeader ? "px-4 py-2 sm:py-3" : "px-6 py-4 sm:py-6"} flex flex-col gap-3`}
                >
                    {/* Title and Description */}
                    <div className="flex flex-col gap-1.5">
                        <div className="flex items-center space-x-2">
                            <Sparkles className="h-7 w-7 text-primary" />
                            <h1
                                className={`scroll-m-20 font-extrabold tracking-tight bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text text-transparent
                                    ${compactHeader ? "text-2xl lg:text-3xl" : "text-4xl lg:text-5xl"}
                                `}
                            >
                                Model Catalog
                            </h1>
                        </div>
                        <p
                            className={`text-muted-foreground max-w-2xl
                                ${compactHeader ? "hidden h-0 opacity-0" : "text-lg"}
                            `}
                        >
                            Discover and configure AI models for your workflow. Browse through
                            our collection of language models, vision models, and embeddings.
                        </p>
                    </div>

                    {/* Search and Filter Controls */}
                    <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
                        <div className="flex flex-1 max-w-md">
                            <div className="relative flex flex-row items-center w-full">
                                <Search className="absolute left-3 top-1/2 h-4 w-4 text-muted-foreground -translate-y-1/2" />
                                <Input
                                    type="search"
                                    placeholder="Search models, types, or descriptions..."
                                    onChange={(e) => {
                                        search(e.target.value);
                                        setSearchTerm(e.target.value);
                                    }}
                                    value={searchTerm}
                                    className="w-full rounded-lg bg-background pl-9 pr-4 py-2 border-2 focus-visible:ring-2 focus-visible:ring-primary/20 focus-visible:border-primary transition-all duration-200"
                                />
                            </div>
                        </div>

                        <div className="flex items-center space-x-2">
                            {activeFilterCount > 0 && (
                                <div className="flex items-center space-x-1">
                                    {searchFilter.appliedFilter
                                        .filter((f) => f !== "All")
                                        .map((filter) => (
                                            <Badge
                                                key={filter}
                                                variant="secondary"
                                                className="flex items-center space-x-1"
                                            >
                                                {getFilterIcon(filter)}
                                                <span>{filter}</span>
                                            </Badge>
                                        ))}
                                </div>
                            )}

                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        className="h-9 gap-2 border-2 hover:bg-accent/50 transition-colors duration-200"
                                    >
                                        <ListFilter className="h-4 w-4" />
                                        <span className="hidden sm:inline-block">Filter</span>
                                        {activeFilterCount > 0 && (
                                            <Badge variant="secondary" className="ml-1 h-5 w-5 p-0">
                                                <small className="text-xs text-center ml-1">
                                                    {activeFilterCount}
                                                </small>
                                            </Badge>
                                        )}
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-56">
                                    <DropdownMenuLabel className="flex items-center space-x-2">
                                        <Filter className="h-4 w-4" />
                                        <span>Filter by Type</span>
                                    </DropdownMenuLabel>
                                    <DropdownMenuSeparator />
                                    <DropdownMenuCheckboxItem
                                        checked={searchFilter.appliedFilter.includes("All")}
                                        onCheckedChange={(checked) => {
                                            if (checked) {
                                                setSearchFilter((old) => ({
                                                    ...old,
                                                    appliedFilter: ["All"],
                                                }));
                                                return;
                                            }
                                            setSearchFilter((old) => ({
                                                ...old,
                                                appliedFilter: old.appliedFilter.filter(
                                                    (filter) => filter !== "All",
                                                ),
                                            }));
                                        }}
                                        className="flex items-center space-x-2"
                                    >
                                        <Sparkles className="h-3 w-3" />
                                        <span>All Types</span>
                                    </DropdownMenuCheckboxItem>
                                    <DropdownMenuSeparator />
                                    {searchFilter.filters.map((filter) => (
                                        <DropdownMenuCheckboxItem
                                            checked={searchFilter.appliedFilter.includes(filter)}
                                            key={filter}
                                            onCheckedChange={(checked) => {
                                                if (checked) {
                                                    setSearchFilter((old) => ({
                                                        ...old,
                                                        appliedFilter: [
                                                            ...old.appliedFilter.filter(
                                                                (filter) => filter !== "All",
                                                            ),
                                                            filter,
                                                        ],
                                                    }));
                                                    return;
                                                }
                                                setSearchFilter((old) => ({
                                                    ...old,
                                                    appliedFilter: old.appliedFilter.filter(
                                                        (f) => f !== filter,
                                                    ),
                                                }));
                                            }}
                                            className="flex items-center space-x-2"
                                        >
                                            {getFilterIcon(filter)}
                                            <span>{filter}</span>
                                        </DropdownMenuCheckboxItem>
                                    ))}
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </div>
                    </div>

                    {!foundBits.isLoading && (
                        <div className="flex items-center justify-between text-sm text-muted-foreground">
                            <div className="flex items-center space-x-2">
                                <span>
                                    {searchTerm === ""
                                        ? `${visibleBits.length} models available`
                                        : `${visibleBits.length} results for "${searchTerm}"`}
                                </span>
                                {searchFilter.appliedFilter.length > 0 &&
                                    !searchFilter.appliedFilter.includes("All") && (
                                        <Badge variant="outline" className="text-xs">
                                            Filtered
                                        </Badge>
                                    )}
                            </div>
                        </div>
                    )}
                </div>
            </div>

            {/* Content Section */}
            <div
                ref={scrollContainerRef}
                className="flex flex-grow h-full max-h-full overflow-auto w-full"
            >
                <div className="w-full max-w-screen-xl mx-auto p-6">
                    {foundBits.isLoading && (
                        <div className="space-y-6">
                            <div className="flex items-center justify-center py-8">
                                <Card className="p-6 w-full max-w-md">
                                    <CardContent className="flex flex-col items-center space-y-4 p-0">
                                        <Loader2 className="h-8 w-8 animate-spin text-primary" />
                                        <div className="text-center space-y-2">
                                            <h3 className="font-semibold">Loading Models</h3>
                                            <p className="text-sm text-muted-foreground">
                                                Fetching the latest AI models from the catalog...
                                            </p>
                                        </div>
                                    </CardContent>
                                </Card>
                            </div>
                            <BentoGrid className="mx-auto w-full grid-flow-row-dense">
                                {skeletonWideFlags.map((wide, i) => (
                                    <BentoGridItem
                                        className={`h-full w-full border-2 ${wide ? "md:col-span-2" : ""}`}
                                        key={`${i}__skeleton`}
                                        title={
                                            <div className="flex flex-row items-center space-x-2">
                                                <Skeleton className="h-4 w-[150px]" />
                                                <Skeleton className="h-4 w-[80px]" />
                                            </div>
                                        }
                                        description={
                                            <div className="space-y-2">
                                                <Skeleton className="h-20 w-full rounded-lg" />
                                                <div className="flex space-x-2">
                                                    <Skeleton className="h-6 w-16 rounded-full" />
                                                    <Skeleton className="h-6 w-20 rounded-full" />
                                                </div>
                                            </div>
                                        }
                                        header={
                                            <div className="space-y-3">
                                                <div className="flex flex-row items-center space-x-3">
                                                    <Skeleton className="h-12 w-12 rounded-full" />
                                                    <div className="space-y-1">
                                                        <Skeleton className="h-4 w-[100px]" />
                                                        <Skeleton className="h-3 w-[60px]" />
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                        icon={<Skeleton className="h-4 w-[120px]" />}
                                    />
                                ))}
                            </BentoGrid>
                        </div>
                    )}
                    {!foundBits.isLoading &&
                        (visibleBits.length === 0 ? (
                            <Card className="p-8 text-center max-w-md mx-auto mt-12">
                                <CardContent className="space-y-4 p-0">
                                    <div className="w-16 h-16 mx-auto bg-muted rounded-full flex items-center justify-center">
                                        <Search className="h-8 w-8 text-muted-foreground" />
                                    </div>
                                    <div className="space-y-2">
                                        <h3 className="font-semibold text-lg">No models found</h3>
                                        <p className="text-muted-foreground">
                                            {searchTerm
                                                ? `No models match "${searchTerm}". Try adjusting your search or filters.`
                                                : "No models available with the current filters."}
                                        </p>
                                    </div>
                                    {(searchTerm || activeFilterCount > 0) && (
                                        <Button
                                            variant="outline"
                                            onClick={() => {
                                                setSearchTerm("");
                                                search("");
                                                setSearchFilter((old) => ({
                                                    ...old,
                                                    appliedFilter: ["All"],
                                                }));
                                            }}
                                            className="mt-4"
                                        >
                                            Clear filters
                                        </Button>
                                    )}
                                </CardContent>
                            </Card>
                        ) : (
                            <BentoGrid className="mx-auto w-full pb-20 grid-flow-row-dense">
                                {visibleBits.map((bit: IBit, i: number) => {
                                    const wide = wideFlags[i];
                                    return <BitCard key={bit.id} bit={bit} wide={wide} />;
                                })}
                            </BentoGrid>
                        ))}
                </div>
            </div>
        </main>
    );
}