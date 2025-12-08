"use client";

import {
	type ISolutionListResponse,
	type ISolutionUpdatePayload,
	SolutionStatus,
	SolutionsPage,
	useBackend,
	useInvoke,
	useQuery,
	useQueryClient,
} from "@tm9657/flow-like-ui";
import { useDebounce } from "@uidotdev/usehooks";
import { useCallback, useMemo, useState } from "react";
import { useAuth } from "react-oidc-context";
import { toast } from "sonner";
import { fetcher, patch } from "../../../lib/api";

export default function AdminSolutionsPage() {
	const auth = useAuth();
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
		queryKey: ["admin", "solutions", queryParams, auth?.user?.profile?.sub],
		queryFn: async () => {
			if (!profile.data) throw new Error("Profile not loaded");
			const queryString = new URLSearchParams(
				Object.entries(queryParams).map(([k, v]) => [k, String(v)]),
			).toString();
			return fetcher<ISolutionListResponse>(
				profile.data,
				`admin/solutions?${queryString}`,
				{ method: "GET" },
				auth,
			);
		},
		enabled: !!profile.data && auth?.isAuthenticated,
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
				await patch(profile.data, `admin/solutions/${id}`, update, auth);
				toast.success("Solution updated successfully");
			} catch (error) {
				toast.error(
					`Failed to update solution: ${error instanceof Error ? error.message : "Unknown error"}`,
				);
				throw error;
			}
		},
		[profile.data, auth],
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
			/>
		</main>
	);
}
