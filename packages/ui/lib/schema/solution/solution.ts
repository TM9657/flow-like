"use client";

export enum SolutionStatus {
	AWAITING_DEPOSIT = "AWAITING_DEPOSIT",
	PENDING_REVIEW = "PENDING_REVIEW",
	IN_QUEUE = "IN_QUEUE",
	ONBOARDING_DONE = "ONBOARDING_DONE",
	IN_PROGRESS = "IN_PROGRESS",
	DELIVERED = "DELIVERED",
	AWAITING_PAYMENT = "AWAITING_PAYMENT",
	PAID = "PAID",
	CANCELLED = "CANCELLED",
	REFUNDED = "REFUNDED",
}

export enum SolutionPricingTier {
	STANDARD = "STANDARD",
	APPSTORE = "APPSTORE",
}

export interface ISolutionFile {
	name: string;
	key: string;
	downloadUrl: string;
	size: number;
}

export interface ISolutionLog {
	id: string;
	solutionId: string;
	action: string;
	details: string | null;
	actor: string | null;
	createdAt: string;
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
	files: ISolutionFile[] | null;
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
	trackingToken: string;
	logs?: ISolutionLog[];
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

export interface ISolutionLogPayload {
	action: string;
	details?: string;
}

export const SolutionStatusLabels: Record<SolutionStatus, string> = {
	[SolutionStatus.AWAITING_DEPOSIT]: "Awaiting Deposit",
	[SolutionStatus.PENDING_REVIEW]: "Pending Review",
	[SolutionStatus.IN_QUEUE]: "In Queue",
	[SolutionStatus.ONBOARDING_DONE]: "Onboarding Done",
	[SolutionStatus.IN_PROGRESS]: "In Progress",
	[SolutionStatus.DELIVERED]: "Delivered",
	[SolutionStatus.AWAITING_PAYMENT]: "Awaiting Payment",
	[SolutionStatus.PAID]: "Paid",
	[SolutionStatus.CANCELLED]: "Cancelled",
	[SolutionStatus.REFUNDED]: "Refunded",
};

export const SolutionStatusColors: Record<SolutionStatus, string> = {
	[SolutionStatus.AWAITING_DEPOSIT]: "bg-yellow-500/10 text-yellow-500",
	[SolutionStatus.PENDING_REVIEW]: "bg-blue-500/10 text-blue-500",
	[SolutionStatus.IN_QUEUE]: "bg-cyan-500/10 text-cyan-500",
	[SolutionStatus.ONBOARDING_DONE]: "bg-indigo-500/10 text-indigo-500",
	[SolutionStatus.IN_PROGRESS]: "bg-purple-500/10 text-purple-500",
	[SolutionStatus.DELIVERED]: "bg-emerald-500/10 text-emerald-500",
	[SolutionStatus.AWAITING_PAYMENT]: "bg-amber-500/10 text-amber-500",
	[SolutionStatus.PAID]: "bg-green-500/10 text-green-500",
	[SolutionStatus.CANCELLED]: "bg-red-500/10 text-red-500",
	[SolutionStatus.REFUNDED]: "bg-orange-500/10 text-orange-500",
};

export const SolutionStatusOrder: SolutionStatus[] = [
	SolutionStatus.AWAITING_DEPOSIT,
	SolutionStatus.PENDING_REVIEW,
	SolutionStatus.IN_QUEUE,
	SolutionStatus.ONBOARDING_DONE,
	SolutionStatus.IN_PROGRESS,
	SolutionStatus.DELIVERED,
	SolutionStatus.AWAITING_PAYMENT,
	SolutionStatus.PAID,
];

export const SolutionStatusDescriptions: Record<SolutionStatus, string> = {
	[SolutionStatus.AWAITING_DEPOSIT]: "Awaiting priority deposit payment",
	[SolutionStatus.PENDING_REVIEW]: "Request submitted, pending review",
	[SolutionStatus.IN_QUEUE]: "Approved and waiting in queue",
	[SolutionStatus.ONBOARDING_DONE]: "Onboarding completed",
	[SolutionStatus.IN_PROGRESS]: "Actively being worked on",
	[SolutionStatus.DELIVERED]: "Solution has been delivered",
	[SolutionStatus.AWAITING_PAYMENT]: "Awaiting final payment",
	[SolutionStatus.PAID]: "Payment completed",
	[SolutionStatus.CANCELLED]: "Request was cancelled",
	[SolutionStatus.REFUNDED]: "Payment has been refunded",
};
