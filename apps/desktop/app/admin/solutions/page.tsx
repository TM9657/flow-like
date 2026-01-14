"use client";

import {
	type ISolutionListResponse,
	type ISolutionLogPayload,
	type ISolutionRequest,
	type ISolutionUpdatePayload,
	type SolutionStatus,
	SolutionsPage,
	useBackend,
	useInvoke,
	useQuery,
	useQueryClient,
} from "@tm9657/flow-like-ui";
import { useDebounce } from "@uidotdev/usehooks";
import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";

export default function AdminSolutionsPage() {
	const backend = useBackend();
	const queryClient = useQueryClient();

	const profile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
	);

	const [page, setPage] = useState(1);
	const [limit, setLimit] = useState(25);
	const [statusFilter, setStatusFilter] = useState<SolutionStatus | undefined>(
		undefined,
	);
	const [searchQuery, setSearchQuery] = useState("");
	const debouncedSearch = useDebounce(searchQuery, 300);

	const queryParams = useMemo(() => {
		const params: Record<string, string | number> = {
			page,
			limit,
		};
		if (statusFilter) params.status = statusFilter;
		if (debouncedSearch) params.search = debouncedSearch;
		return params;
	}, [page, limit, statusFilter, debouncedSearch]);

	const solutions = useQuery<ISolutionListResponse, Error>({
		queryKey: ["admin", "solutions", queryParams],
		queryFn: async () => {
			if (!profile.data) throw new Error("Profile not loaded");
			const queryString = new URLSearchParams(
				Object.entries(queryParams).map(([k, v]) => [k, String(v)]),
			).toString();
			return backend.apiState.get<ISolutionListResponse>(
				profile.data,
				`admin/solutions?${queryString}`,
			);
		},
		enabled: !!profile.data,
	});

	const handleRefresh = useCallback(() => {
		queryClient.invalidateQueries({
			queryKey: ["admin", "solutions"],
		});
	}, [queryClient]);

	const handlePageChange = useCallback((newPage: number) => {
		setPage(newPage);
	}, []);

	const handleLimitChange = useCallback((newLimit: number) => {
		setLimit(newLimit);
		setPage(1);
	}, []);

	const handleStatusFilterChange = useCallback(
		(status: SolutionStatus | undefined) => {
			setStatusFilter(status);
			setPage(1);
		},
		[],
	);

	const handleSearchChange = useCallback((query: string) => {
		setSearchQuery(query);
		setPage(1);
	}, []);

	const handleUpdateSolution = useCallback(
		async (id: string, update: ISolutionUpdatePayload) => {
			if (!profile.data) throw new Error("Profile not loaded");

			try {
				await backend.apiState.patch(
					profile.data,
					`admin/solutions/${id}`,
					update,
				);
				toast.success("Solution updated successfully");
			} catch (error) {
				toast.error(
					`Failed to update solution: ${error instanceof Error ? error.message : "Unknown error"}`,
				);
				throw error;
			}
		},
		[profile.data, backend.apiState],
	);

	const handleFetchSolution = useCallback(
		async (id: string): Promise<ISolutionRequest | null> => {
			if (!profile.data) throw new Error("Profile not loaded");
			return backend.apiState.get<ISolutionRequest>(
				profile.data,
				`admin/solutions/${id}`,
			);
		},
		[profile.data, backend.apiState],
	);

	const handleAddLog = useCallback(
		async (id: string, log: ISolutionLogPayload) => {
			if (!profile.data) throw new Error("Profile not loaded");

			try {
				await backend.apiState.post(
					profile.data,
					`admin/solutions/${id}/logs`,
					log,
				);
				toast.success("Log added successfully");
			} catch (error) {
				toast.error(
					`Failed to add log: ${error instanceof Error ? error.message : "Unknown error"}`,
				);
				throw error;
			}
		},
		[profile.data, backend.apiState],
	);

	return (
		<main className="flex grow h-full bg-background max-h-full overflow-auto flex-col items-start w-full justify-start p-6">
			<SolutionsPage
				data={solutions.data}
				isLoading={solutions.isLoading}
				error={solutions.error}
				page={page}
				limit={limit}
				statusFilter={statusFilter}
				searchQuery={searchQuery}
				onPageChange={handlePageChange}
				onLimitChange={handleLimitChange}
				onStatusFilterChange={handleStatusFilterChange}
				onSearchChange={handleSearchChange}
				onRefresh={handleRefresh}
				onUpdateSolution={handleUpdateSolution}
				onFetchSolution={handleFetchSolution}
				onAddLog={handleAddLog}
				trackingBaseUrl="https://www.flow-like.com"
			/>
		</main>
	);
}
