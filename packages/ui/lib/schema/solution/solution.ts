"use client";

export enum SolutionStatus {
	PENDING_PAYMENT = "PENDING_PAYMENT",
	PENDING_REVIEW = "PENDING_REVIEW",
	IN_PROGRESS = "IN_PROGRESS",
	DELIVERED = "DELIVERED",
	CANCELLED = "CANCELLED",
	REFUNDED = "REFUNDED",
}

export enum SolutionPricingTier {
	STANDARD = "STANDARD",
	APPSTORE = "APPSTORE",
}

export interface ISolutionRequest {
	id: string;
	name: string;
	email: string;
	company: string;
	description: string;
	applicationType: string;
	dataSecurity: string;
	exampleInput: string;
	expectedOutput: string;
	userCount: string;
	userType: string;
	technicalLevel: string;
	timeline: string | null;
	additionalNotes: string | null;
	pricingTier: SolutionPricingTier;
	paidDeposit: boolean;
	files: Record<string, string>[] | null;
	storageKey: string | null;
	status: SolutionStatus;
	stripeCheckoutSessionId: string | null;
	stripePaymentIntentId: string | null;
	stripeSetupIntentId: string | null;
	totalCents: number;
	depositCents: number;
	remainderCents: number;
	priority: boolean;
	adminNotes: string | null;
	assignedTo: string | null;
	deliveredAt: string | null;
	createdAt: string;
	updatedAt: string;
}

export interface ISolutionListResponse {
	solutions: ISolutionRequest[];
	total: number;
	page: number;
	limit: number;
	hasMore: boolean;
}

export interface ISolutionUpdatePayload {
	status?: SolutionStatus;
	adminNotes?: string;
	assignedTo?: string;
	priority?: boolean;
}

export const SolutionStatusLabels: Record<SolutionStatus, string> = {
	[SolutionStatus.PENDING_PAYMENT]: "Pending Payment",
	[SolutionStatus.PENDING_REVIEW]: "Pending Review",
	[SolutionStatus.IN_PROGRESS]: "In Progress",
	[SolutionStatus.DELIVERED]: "Delivered",
	[SolutionStatus.CANCELLED]: "Cancelled",
	[SolutionStatus.REFUNDED]: "Refunded",
};

export const SolutionStatusColors: Record<SolutionStatus, string> = {
	[SolutionStatus.PENDING_PAYMENT]: "bg-yellow-500/10 text-yellow-500",
	[SolutionStatus.PENDING_REVIEW]: "bg-blue-500/10 text-blue-500",
	[SolutionStatus.IN_PROGRESS]: "bg-purple-500/10 text-purple-500",
	[SolutionStatus.DELIVERED]: "bg-green-500/10 text-green-500",
	[SolutionStatus.CANCELLED]: "bg-red-500/10 text-red-500",
	[SolutionStatus.REFUNDED]: "bg-orange-500/10 text-orange-500",
};
