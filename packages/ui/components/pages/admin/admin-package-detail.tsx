"use client";

import { useCallback } from "react";
import type {
	AdminPackageDetailResponse,
	ReviewRequest,
} from "../../../lib/schema/wasm";
import { useBackend } from "../../../state/backend-state";
import { useInvoke } from "../../../hooks/use-invoke";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AdminPackageDetailView } from "../../store/admin-package-detail-view";

export interface AdminPackageDetailProps {
	packageId: string;
	onBack: () => void;
	onSuccess?: () => void;
}

export function AdminPackageDetail({
	packageId,
	onBack,
	onSuccess,
}: AdminPackageDetailProps) {
	const backend = useBackend();
	const queryClient = useQueryClient();

	const profile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
	);

	const packageDetail = useQuery<AdminPackageDetailResponse>({
		queryKey: ["admin", "packages", packageId],
		queryFn: async () => {
			if (!profile.data) throw new Error("Profile not loaded");
			return backend.apiState.get<AdminPackageDetailResponse>(
				profile.data,
				`admin/packages/${packageId}`,
			);
		},
		enabled: !!profile.data && !!packageId,
	});

	const submitReview = useMutation({
		mutationFn: async (review: ReviewRequest) => {
			if (!profile.data) throw new Error("Profile not loaded");
			return backend.apiState.post(
				profile.data,
				`admin/packages/${packageId}/review`,
				review,
			);
		},
		onSuccess: () => {
			onSuccess?.();
			queryClient.invalidateQueries({
				queryKey: ["admin", "packages", packageId],
			});
		},
	});

	const updatePackage = useMutation({
		mutationFn: async (data: { status?: string; verified?: boolean }) => {
			if (!profile.data) throw new Error("Profile not loaded");
			return backend.apiState.patch(
				profile.data,
				`admin/packages/${packageId}`,
				data,
			);
		},
		onSuccess: () => {
			onSuccess?.();
			queryClient.invalidateQueries({ queryKey: ["admin", "packages"] });
		},
	});

	const handleSubmitReview = useCallback(
		(review: ReviewRequest) => submitReview.mutate(review),
		[submitReview],
	);

	const handleUpdatePackage = useCallback(
		(data: { status?: string; verified?: boolean }) => updatePackage.mutate(data),
		[updatePackage],
	);

	return (
		<AdminPackageDetailView
			packageDetail={packageDetail.data}
			isLoading={packageDetail.isLoading}
			onBack={onBack}
			onSubmitReview={handleSubmitReview}
			onUpdatePackage={handleUpdatePackage}
			isSubmittingReview={submitReview.isPending}
			isUpdatingPackage={updatePackage.isPending}
		/>
	);
}
