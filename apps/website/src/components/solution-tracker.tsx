import React, { useState, useEffect } from "react";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Input,
	Label,
} from "@tm9657/flow-like-ui";
import {
	LuCheck,
	LuClock,
	LuCreditCard,
	LuLoader,
	LuPackage,
	LuSearch,
	LuCircleAlert,
	LuX,
	LuRefreshCw,
	LuArrowRight,
	LuBuilding2,
	LuCalendar,
	LuZap,
	LuSparkles,
} from "react-icons/lu";

interface PublicSolutionLog {
	action: string;
	createdAt: string;
}

interface PublicSolutionStatus {
	id: string;
	company: string;
	status: string;
	statusLabel: string;
	statusDescription: string;
	paidDeposit: boolean;
	priority: boolean;
	pricingTier: string;
	totalCents: number;
	depositCents: number;
	remainderCents: number;
	deliveredAt: string | null;
	createdAt: string;
	updatedAt: string;
	logs: PublicSolutionLog[];
}

const STATUS_STEPS = [
	{ key: "PendingPayment", label: "Payment", icon: LuCreditCard },
	{ key: "PendingReview", label: "Review", icon: LuClock },
	{ key: "InProgress", label: "In Progress", icon: LuLoader },
	{ key: "Delivered", label: "Delivered", icon: LuPackage },
];

const STATUS_ORDER: Record<string, number> = {
	PendingPayment: 0,
	PendingReview: 1,
	InProgress: 2,
	Delivered: 3,
	Cancelled: -1,
	Refunded: -1,
};

function formatCurrency(cents: number): string {
	return new Intl.NumberFormat("en-US", {
		style: "currency",
		currency: "EUR",
	}).format(cents / 100);
}

function formatDate(dateStr: string): string {
	return new Date(dateStr).toLocaleDateString("en-US", {
		year: "numeric",
		month: "long",
		day: "numeric",
		hour: "2-digit",
		minute: "2-digit",
	});
}

function getStatusColor(status: string): string {
	switch (status) {
		case "PendingPayment":
			return "text-yellow-500";
		case "PendingReview":
			return "text-blue-500";
		case "InProgress":
			return "text-purple-500";
		case "Delivered":
			return "text-green-500";
		case "Cancelled":
			return "text-red-500";
		case "Refunded":
			return "text-orange-500";
		default:
			return "text-muted-foreground";
	}
}

interface SolutionTrackerProps {
	initialToken?: string;
}

export function SolutionTracker({ initialToken }: SolutionTrackerProps) {
	const [token, setToken] = useState(initialToken || "");
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [solution, setSolution] = useState<PublicSolutionStatus | null>(null);
	const [searched, setSearched] = useState(false);

	useEffect(() => {
		// Check URL params for token
		const urlParams = new URLSearchParams(window.location.search);
		const urlToken = urlParams.get("token");
		const tokenToUse = initialToken || urlToken;

		if (tokenToUse) {
			setToken(tokenToUse);
			fetchStatus(tokenToUse);
		}
	}, [initialToken]);

	async function fetchStatus(trackingToken: string) {
		if (!trackingToken.trim()) {
			setError("Please enter a tracking token");
			return;
		}

		setLoading(true);
		setError(null);
		setSearched(true);

		try {
			const apiUrl = import.meta.env.PUBLIC_API_URL || "https://api.flow-like.com";
			const response = await fetch(`${apiUrl}/solution/track/${encodeURIComponent(trackingToken.trim())}`);

			if (response.status === 404) {
				setError("No solution found with this tracking token. Please check and try again.");
				setSolution(null);
				return;
			}

			if (!response.ok) {
				throw new Error("Failed to fetch solution status");
			}

			const data = await response.json();
			setSolution(data);
		} catch {
			setError("Unable to fetch solution status. Please try again later.");
			setSolution(null);
		} finally {
			setLoading(false);
		}
	}

	function handleSubmit(e: React.FormEvent) {
		e.preventDefault();
		fetchStatus(token);
		// Update URL with token
		const url = new URL(window.location.href);
		url.searchParams.set("token", token);
		window.history.replaceState({}, "", url.toString());
	}

	const currentStep = solution ? STATUS_ORDER[solution.status] ?? -1 : -1;
	const isCancelledOrRefunded = solution && (solution.status === "Cancelled" || solution.status === "Refunded");

	return (
		<div className="w-full max-w-4xl mx-auto space-y-8">
			{/* Search Form */}
			<Card className="border-primary/20 bg-card/60 backdrop-blur-sm">
				<CardHeader className="text-center">
					<CardTitle className="text-2xl">Track Your Solution</CardTitle>
					<CardDescription>
						Enter your tracking token to view the status of your 24-hour solution request
					</CardDescription>
				</CardHeader>
				<CardContent>
					<form onSubmit={handleSubmit} className="flex flex-col sm:flex-row gap-3">
						<div className="flex-1">
							<Label htmlFor="token" className="sr-only">Tracking Token</Label>
							<Input
								id="token"
								type="text"
								placeholder="Enter your tracking token..."
								value={token}
								onChange={(e) => setToken(e.target.value)}
								className="h-12 text-base"
							/>
						</div>
						<Button type="submit" disabled={loading} className="h-12 px-6">
							{loading ? (
								<LuLoader className="h-4 w-4 animate-spin mr-2" />
							) : (
								<LuSearch className="h-4 w-4 mr-2" />
							)}
							{loading ? "Searching..." : "Track"}
						</Button>
					</form>
				</CardContent>
			</Card>

			{/* Error State */}
			{error && searched && (
				<Card className="border-destructive/50 bg-destructive/5">
					<CardContent className="py-8 text-center">
						<LuCircleAlert className="h-12 w-12 text-destructive mx-auto mb-4" />
						<p className="text-destructive font-medium">{error}</p>
						<p className="text-sm text-muted-foreground mt-2">
							If you just submitted a request, your tracking token was sent to your email.
						</p>
					</CardContent>
				</Card>
			)}

			{/* Solution Status */}
			{solution && !error && (
				<div className="space-y-6">
					{/* Status Header */}
					<Card className="border-primary/20 bg-card/60 backdrop-blur-sm overflow-hidden">
						<div className="p-6 sm:p-8">
							{/* Company & Status */}
							<div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4 mb-8">
								<div>
									<div className="flex items-center gap-2 text-muted-foreground mb-1">
										<LuBuilding2 className="h-4 w-4" />
										<span className="text-sm">Company</span>
									</div>
									<h2 className="text-2xl font-bold">{solution.company}</h2>
								</div>
								<div className="flex items-center gap-3">
									{solution.priority && (
										<span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-amber-500/10 text-amber-500 text-sm font-medium">
											<LuZap className="h-3.5 w-3.5" />
											Priority
										</span>
									)}
									<span className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium ${getStatusColor(solution.status)} bg-current/10`}>
										{isCancelledOrRefunded ? (
											solution.status === "Cancelled" ? <LuX className="h-3.5 w-3.5" /> : <LuRefreshCw className="h-3.5 w-3.5" />
										) : (
											<LuSparkles className="h-3.5 w-3.5" />
										)}
										{solution.statusLabel}
									</span>
								</div>
							</div>

							{/* Progress Timeline */}
							{!isCancelledOrRefunded && (
								<div className="mb-8">
									<div className="relative flex justify-between">
										{/* Progress Line */}
										<div className="absolute top-5 left-0 right-0 h-0.5 bg-muted">
											<div
												className="h-full bg-primary transition-all duration-500"
												style={{ width: `${Math.max(0, (currentStep / (STATUS_STEPS.length - 1)) * 100)}%` }}
											/>
										</div>

										{/* Steps */}
										{STATUS_STEPS.map((step, index) => {
											const isCompleted = index < currentStep;
											const isCurrent = index === currentStep;
											const Icon = step.icon;

											return (
												<div key={step.key} className="relative flex flex-col items-center z-10">
													<div className={`
														w-10 h-10 rounded-full flex items-center justify-center transition-all duration-300
														${isCompleted ? "bg-primary text-primary-foreground" : ""}
														${isCurrent ? "bg-primary text-primary-foreground ring-4 ring-primary/20" : ""}
														${!isCompleted && !isCurrent ? "bg-muted text-muted-foreground" : ""}
													`}>
														{isCompleted ? (
															<LuCheck className="h-5 w-5" />
														) : isCurrent && step.key === "InProgress" ? (
															<LuLoader className="h-5 w-5 animate-spin" />
														) : (
															<Icon className="h-5 w-5" />
														)}
													</div>
													<span className={`mt-2 text-xs font-medium text-center max-w-20 ${isCurrent ? "text-primary" : "text-muted-foreground"}`}>
														{step.label}
													</span>
												</div>
											);
										})}
									</div>
								</div>
							)}

							{/* Status Description */}
							<div className="p-4 rounded-xl bg-muted/50 border">
								<p className="text-center text-muted-foreground">
									{solution.statusDescription}
								</p>
							</div>
						</div>
					</Card>

					{/* Details Grid */}
					<div className="grid sm:grid-cols-2 gap-6">
						{/* Payment Info */}
						<Card className="border-primary/10">
							<CardHeader>
								<CardTitle className="text-lg flex items-center gap-2">
									<LuCreditCard className="h-5 w-5 text-primary" />
									Payment Details
								</CardTitle>
							</CardHeader>
							<CardContent className="space-y-4">
								<div className="flex justify-between items-center">
									<span className="text-muted-foreground">Total</span>
									<span className="font-semibold text-lg">{formatCurrency(solution.totalCents)}</span>
								</div>
								<div className="flex justify-between items-center">
									<span className="text-muted-foreground">Deposit</span>
									<div className="flex items-center gap-2">
										<span className="font-medium">{formatCurrency(solution.depositCents)}</span>
										{solution.paidDeposit ? (
											<span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-green-500/10 text-green-500 text-xs font-medium">
												<LuCheck className="h-3 w-3" />
												Paid
											</span>
										) : (
											<span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-yellow-500/10 text-yellow-500 text-xs font-medium">
												<LuClock className="h-3 w-3" />
												Pending
											</span>
										)}
									</div>
								</div>
								<div className="flex justify-between items-center pt-3 border-t">
									<span className="text-muted-foreground">Remainder</span>
									<span className="font-medium">{formatCurrency(solution.remainderCents)}</span>
								</div>
								<div className="flex justify-between items-center text-sm">
									<span className="text-muted-foreground">Tier</span>
									<span className="capitalize">{solution.pricingTier}</span>
								</div>
							</CardContent>
						</Card>

						{/* Timeline Info */}
						<Card className="border-primary/10">
							<CardHeader>
								<CardTitle className="text-lg flex items-center gap-2">
									<LuCalendar className="h-5 w-5 text-primary" />
									Timeline
								</CardTitle>
							</CardHeader>
							<CardContent className="space-y-4">
								<div className="flex justify-between items-center">
									<span className="text-muted-foreground">Submitted</span>
									<span className="text-sm font-medium">{formatDate(solution.createdAt)}</span>
								</div>
								<div className="flex justify-between items-center">
									<span className="text-muted-foreground">Last Updated</span>
									<span className="text-sm font-medium">{formatDate(solution.updatedAt)}</span>
								</div>
								{solution.deliveredAt && (
									<div className="flex justify-between items-center pt-3 border-t">
										<span className="text-muted-foreground">Delivered</span>
										<span className="text-sm font-medium text-green-500">{formatDate(solution.deliveredAt)}</span>
									</div>
								)}
							</CardContent>
						</Card>
					</div>

					{/* Activity Log */}
					{solution.logs.length > 0 && (
						<Card className="border-primary/10">
							<CardHeader>
								<CardTitle className="text-lg flex items-center gap-2">
									<LuClock className="h-5 w-5 text-primary" />
									Activity Log
								</CardTitle>
								<CardDescription>
									Recent updates on your solution request
								</CardDescription>
							</CardHeader>
							<CardContent>
								<div className="space-y-4">
									{solution.logs.map((log, index) => (
										<div key={index} className="flex gap-4 items-start">
											<div className="shrink-0 w-2 h-2 mt-2 rounded-full bg-primary" />
											<div className="flex-1 min-w-0">
												<p className="font-medium">{log.action}</p>
												<p className="text-sm text-muted-foreground">{formatDate(log.createdAt)}</p>
											</div>
										</div>
									))}
								</div>
							</CardContent>
						</Card>
					)}

					{/* Help Section */}
					<Card className="border-muted bg-muted/30">
						<CardContent className="py-6">
							<div className="flex flex-col sm:flex-row items-center justify-between gap-4">
								<div className="text-center sm:text-left">
									<h3 className="font-semibold">Need Help?</h3>
									<p className="text-sm text-muted-foreground">
										Contact us if you have any questions about your solution.
									</p>
								</div>
								<Button variant="outline" asChild>
									<a href="mailto:support@flow-like.com" className="flex items-center gap-2">
										Contact Support
										<LuArrowRight className="h-4 w-4" />
									</a>
								</Button>
							</div>
						</CardContent>
					</Card>
				</div>
			)}

			{/* Initial State */}
			{!searched && !solution && (
				<Card className="border-muted bg-muted/30">
					<CardContent className="py-12 text-center">
						<LuPackage className="h-16 w-16 text-muted-foreground/50 mx-auto mb-4" />
						<h3 className="font-semibold text-lg mb-2">Track Your Solution</h3>
						<p className="text-muted-foreground max-w-md mx-auto">
							Enter your tracking token above to view the current status of your 24-hour solution request.
							You received this token in your confirmation email.
						</p>
					</CardContent>
				</Card>
			)}
		</div>
	);
}
