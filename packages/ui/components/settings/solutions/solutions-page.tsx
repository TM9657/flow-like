"use client";

import {
	AlertCircle,
	ArrowUpDown,
	Building2,
	Calendar,
	ChevronLeft,
	ChevronRight,
	ChevronsLeft,
	ChevronsRight,
	Clock,
	DollarSign,
	Edit,
	ExternalLink,
	FileText,
	Filter,
	Loader2,
	Mail,
	RefreshCw,
	Search,
	Star,
	User,
	X,
} from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import {
	type ISolutionListResponse,
	type ISolutionRequest,
	type ISolutionUpdatePayload,
	SolutionStatus,
	SolutionStatusColors,
	SolutionStatusLabels,
} from "../../../lib/schema/solution/solution";
import { Badge } from "../../ui/badge";
import { Button } from "../../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../ui/dialog";
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "../../ui/dropdown-menu";
import { Input } from "../../ui/input";
import { Label } from "../../ui/label";
import { ScrollArea } from "../../ui/scroll-area";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../ui/select";
import { Separator } from "../../ui/separator";
import { Skeleton } from "../../ui/skeleton";
import { Switch } from "../../ui/switch";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "../../ui/table";
import { Textarea } from "../../ui/textarea";

export interface SolutionsPageProps {
	data: ISolutionListResponse | undefined;
	isLoading: boolean;
	error: Error | null;
	page: number;
	limit: number;
	statusFilter: SolutionStatus | undefined;
	searchQuery: string;
	onPageChange: (page: number) => void;
	onLimitChange: (limit: number) => void;
	onStatusFilterChange: (status: SolutionStatus | undefined) => void;
	onSearchChange: (query: string) => void;
	onRefresh: () => void;
	onUpdateSolution: (
		id: string,
		update: ISolutionUpdatePayload,
	) => Promise<void>;
}

export function SolutionsPage({
	data,
	isLoading,
	error,
	page,
	limit,
	statusFilter,
	searchQuery,
	onPageChange,
	onLimitChange,
	onStatusFilterChange,
	onSearchChange,
	onRefresh,
	onUpdateSolution,
}: Readonly<SolutionsPageProps>) {
	const [selectedSolution, setSelectedSolution] =
		useState<ISolutionRequest | null>(null);
	const [editingSolution, setEditingSolution] =
		useState<ISolutionRequest | null>(null);
	const [isUpdating, setIsUpdating] = useState(false);

	const handleUpdateSolution = useCallback(
		async (update: ISolutionUpdatePayload) => {
			if (!editingSolution) return;
			setIsUpdating(true);
			try {
				await onUpdateSolution(editingSolution.id, update);
				setEditingSolution(null);
				onRefresh();
			} finally {
				setIsUpdating(false);
			}
		},
		[editingSolution, onUpdateSolution, onRefresh],
	);

	const totalPages = useMemo(() => {
		if (!data) return 1;
		return Math.ceil(data.total / limit);
	}, [data, limit]);

	if (error) {
		return (
			<Card className="w-full">
				<CardContent className="flex flex-col items-center justify-center py-12 gap-4">
					<AlertCircle className="h-12 w-12 text-destructive" />
					<p className="text-lg font-medium">Failed to load solutions</p>
					<p className="text-sm text-muted-foreground">{error.message}</p>
					<Button onClick={onRefresh} variant="outline">
						<RefreshCw className="h-4 w-4 mr-2" />
						Retry
					</Button>
				</CardContent>
			</Card>
		);
	}

	return (
		<div className="space-y-4">
			<div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
				<div>
					<h2 className="text-2xl font-bold tracking-tight">
						Solution Requests
					</h2>
					<p className="text-muted-foreground">
						Manage and track custom solution requests
					</p>
				</div>
				<Button onClick={onRefresh} variant="outline" size="sm">
					<RefreshCw className="h-4 w-4 mr-2" />
					Refresh
				</Button>
			</div>

			<SolutionFilters
				searchQuery={searchQuery}
				statusFilter={statusFilter}
				onSearchChange={onSearchChange}
				onStatusFilterChange={onStatusFilterChange}
			/>

			{isLoading ? (
				<SolutionsTableSkeleton />
			) : data?.solutions.length === 0 ? (
				<Card>
					<CardContent className="flex flex-col items-center justify-center py-12 gap-4">
						<FileText className="h-12 w-12 text-muted-foreground" />
						<p className="text-lg font-medium">No solutions found</p>
						<p className="text-sm text-muted-foreground">
							{statusFilter || searchQuery
								? "Try adjusting your filters"
								: "No solution requests have been submitted yet"}
						</p>
					</CardContent>
				</Card>
			) : (
				<>
					<SolutionsTable
						solutions={data?.solutions ?? []}
						onViewDetails={setSelectedSolution}
						onEdit={setEditingSolution}
					/>

					<SolutionsPagination
						page={page}
						limit={limit}
						totalPages={totalPages}
						total={data?.total ?? 0}
						hasMore={data?.hasMore ?? false}
						onPageChange={onPageChange}
						onLimitChange={onLimitChange}
					/>
				</>
			)}

			<SolutionDetailsDialog
				solution={selectedSolution}
				onClose={() => setSelectedSolution(null)}
				onEdit={(solution) => {
					setSelectedSolution(null);
					setEditingSolution(solution);
				}}
			/>

			<SolutionEditDialog
				solution={editingSolution}
				onClose={() => setEditingSolution(null)}
				onSave={handleUpdateSolution}
				isUpdating={isUpdating}
			/>
		</div>
	);
}

function SolutionFilters({
	searchQuery,
	statusFilter,
	onSearchChange,
	onStatusFilterChange,
}: Readonly<{
	searchQuery: string;
	statusFilter: SolutionStatus | undefined;
	onSearchChange: (query: string) => void;
	onStatusFilterChange: (status: SolutionStatus | undefined) => void;
}>) {
	const statusOptions = Object.values(SolutionStatus);

	return (
		<Card>
			<CardContent className="p-4">
				<div className="flex flex-col sm:flex-row gap-4">
					<div className="relative flex-1">
						<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
						<Input
							placeholder="Search by name, email, or company..."
							value={searchQuery}
							onChange={(e) => onSearchChange(e.target.value)}
							className="pl-9"
						/>
					</div>

					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button variant="outline" className="min-w-[140px]">
								<Filter className="h-4 w-4 mr-2" />
								{statusFilter ? SolutionStatusLabels[statusFilter] : "All Status"}
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align="end" className="w-48">
							<DropdownMenuLabel>Filter by Status</DropdownMenuLabel>
							<DropdownMenuSeparator />
							<DropdownMenuCheckboxItem
								checked={!statusFilter}
								onCheckedChange={() => onStatusFilterChange(undefined)}
							>
								All Status
							</DropdownMenuCheckboxItem>
							{statusOptions.map((status) => (
								<DropdownMenuCheckboxItem
									key={status}
									checked={statusFilter === status}
									onCheckedChange={() => onStatusFilterChange(status)}
								>
									{SolutionStatusLabels[status]}
								</DropdownMenuCheckboxItem>
							))}
						</DropdownMenuContent>
					</DropdownMenu>

					{(statusFilter || searchQuery) && (
						<Button
							variant="ghost"
							size="sm"
							onClick={() => {
								onSearchChange("");
								onStatusFilterChange(undefined);
							}}
						>
							<X className="h-4 w-4 mr-2" />
							Clear
						</Button>
					)}
				</div>
			</CardContent>
		</Card>
	);
}

function SolutionsTable({
	solutions,
	onViewDetails,
	onEdit,
}: Readonly<{
	solutions: ISolutionRequest[];
	onViewDetails: (solution: ISolutionRequest) => void;
	onEdit: (solution: ISolutionRequest) => void;
}>) {
	const formatCurrency = (cents: number) => {
		return new Intl.NumberFormat("en-US", {
			style: "currency",
			currency: "USD",
		}).format(cents / 100);
	};

	const formatDate = (dateString: string) => {
		return new Date(dateString).toLocaleDateString("en-US", {
			year: "numeric",
			month: "short",
			day: "numeric",
		});
	};

	return (
		<Card>
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead className="w-[50px]" />
						<TableHead>Requester</TableHead>
						<TableHead>Company</TableHead>
						<TableHead>Status</TableHead>
						<TableHead>Tier</TableHead>
						<TableHead className="text-right">Total</TableHead>
						<TableHead>Created</TableHead>
						<TableHead className="w-[100px]">Actions</TableHead>
					</TableRow>
				</TableHeader>
				<TableBody>
					{solutions.map((solution) => (
						<TableRow
							key={solution.id}
							className="cursor-pointer hover:bg-muted/50"
							onClick={() => onViewDetails(solution)}
						>
							<TableCell>
								{solution.priority && (
									<Star className="h-4 w-4 text-yellow-500 fill-yellow-500" />
								)}
							</TableCell>
							<TableCell>
								<div className="flex flex-col">
									<span className="font-medium">{solution.name}</span>
									<span className="text-sm text-muted-foreground">
										{solution.email}
									</span>
								</div>
							</TableCell>
							<TableCell>{solution.company}</TableCell>
							<TableCell>
								<Badge className={SolutionStatusColors[solution.status]}>
									{SolutionStatusLabels[solution.status]}
								</Badge>
							</TableCell>
							<TableCell>
								<Badge variant="outline">{solution.pricingTier}</Badge>
							</TableCell>
							<TableCell className="text-right font-medium">
								{formatCurrency(solution.totalCents)}
							</TableCell>
							<TableCell className="text-muted-foreground">
								{formatDate(solution.createdAt)}
							</TableCell>
							<TableCell>
								<Button
									variant="ghost"
									size="sm"
									onClick={(e) => {
										e.stopPropagation();
										onEdit(solution);
									}}
								>
									<Edit className="h-4 w-4" />
								</Button>
							</TableCell>
						</TableRow>
					))}
				</TableBody>
			</Table>
		</Card>
	);
}

function SolutionsPagination({
	page,
	limit,
	totalPages,
	total,
	hasMore,
	onPageChange,
	onLimitChange,
}: Readonly<{
	page: number;
	limit: number;
	totalPages: number;
	total: number;
	hasMore: boolean;
	onPageChange: (page: number) => void;
	onLimitChange: (limit: number) => void;
}>) {
	return (
		<Card>
			<CardContent className="p-4">
				<div className="flex flex-col sm:flex-row items-center justify-between gap-4">
					<div className="text-sm text-muted-foreground">
						Showing {(page - 1) * limit + 1} to{" "}
						{Math.min(page * limit, total)} of {total} results
					</div>

					<div className="flex items-center gap-4">
						<div className="flex items-center gap-2">
							<Label className="text-sm">Per page:</Label>
							<Select
								value={limit.toString()}
								onValueChange={(value) => onLimitChange(Number(value))}
							>
								<SelectTrigger className="w-20">
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="10">10</SelectItem>
									<SelectItem value="25">25</SelectItem>
									<SelectItem value="50">50</SelectItem>
									<SelectItem value="100">100</SelectItem>
								</SelectContent>
							</Select>
						</div>

						<div className="flex items-center gap-1">
							<Button
								variant="outline"
								size="icon"
								onClick={() => onPageChange(1)}
								disabled={page === 1}
							>
								<ChevronsLeft className="h-4 w-4" />
							</Button>
							<Button
								variant="outline"
								size="icon"
								onClick={() => onPageChange(page - 1)}
								disabled={page === 1}
							>
								<ChevronLeft className="h-4 w-4" />
							</Button>
							<span className="px-4 text-sm">
								Page {page} of {totalPages}
							</span>
							<Button
								variant="outline"
								size="icon"
								onClick={() => onPageChange(page + 1)}
								disabled={!hasMore}
							>
								<ChevronRight className="h-4 w-4" />
							</Button>
							<Button
								variant="outline"
								size="icon"
								onClick={() => onPageChange(totalPages)}
								disabled={!hasMore}
							>
								<ChevronsRight className="h-4 w-4" />
							</Button>
						</div>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}

function SolutionsTableSkeleton() {
	return (
		<Card>
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead className="w-[50px]" />
						<TableHead>Requester</TableHead>
						<TableHead>Company</TableHead>
						<TableHead>Status</TableHead>
						<TableHead>Tier</TableHead>
						<TableHead className="text-right">Total</TableHead>
						<TableHead>Created</TableHead>
						<TableHead className="w-[100px]">Actions</TableHead>
					</TableRow>
				</TableHeader>
				<TableBody>
					{Array.from({ length: 5 }).map((_, i) => (
						<TableRow key={`skeleton-${i + 1}`}>
							<TableCell>
								<Skeleton className="h-4 w-4" />
							</TableCell>
							<TableCell>
								<div className="space-y-2">
									<Skeleton className="h-4 w-32" />
									<Skeleton className="h-3 w-48" />
								</div>
							</TableCell>
							<TableCell>
								<Skeleton className="h-4 w-24" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-6 w-20" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-6 w-16" />
							</TableCell>
							<TableCell className="text-right">
								<Skeleton className="h-4 w-16 ml-auto" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-4 w-20" />
							</TableCell>
							<TableCell>
								<Skeleton className="h-8 w-8" />
							</TableCell>
						</TableRow>
					))}
				</TableBody>
			</Table>
		</Card>
	);
}

function SolutionDetailsDialog({
	solution,
	onClose,
	onEdit,
}: Readonly<{
	solution: ISolutionRequest | null;
	onClose: () => void;
	onEdit: (solution: ISolutionRequest) => void;
}>) {
	if (!solution) return null;

	const formatCurrency = (cents: number) => {
		return new Intl.NumberFormat("en-US", {
			style: "currency",
			currency: "USD",
		}).format(cents / 100);
	};

	const formatDate = (dateString: string) => {
		return new Date(dateString).toLocaleString("en-US", {
			year: "numeric",
			month: "long",
			day: "numeric",
			hour: "2-digit",
			minute: "2-digit",
		});
	};

	return (
		<Dialog open={!!solution} onOpenChange={onClose}>
			<DialogContent className="max-w-3xl max-h-[90vh]">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						Solution Request Details
						{solution.priority && (
							<Star className="h-5 w-5 text-yellow-500 fill-yellow-500" />
						)}
					</DialogTitle>
					<DialogDescription>
						Request ID: {solution.id}
					</DialogDescription>
				</DialogHeader>

				<ScrollArea className="max-h-[60vh] pr-4">
					<div className="space-y-6">
						<div className="flex items-center justify-between">
							<Badge className={SolutionStatusColors[solution.status]}>
								{SolutionStatusLabels[solution.status]}
							</Badge>
							<Badge variant="outline">{solution.pricingTier}</Badge>
						</div>

						<Separator />

						<div className="grid grid-cols-2 gap-4">
							<InfoItem icon={User} label="Name" value={solution.name} />
							<InfoItem icon={Mail} label="Email" value={solution.email} />
							<InfoItem
								icon={Building2}
								label="Company"
								value={solution.company}
							/>
							<InfoItem
								icon={Clock}
								label="Timeline"
								value={solution.timeline ?? "Not specified"}
							/>
						</div>

						<Separator />

						<div className="space-y-4">
							<h4 className="font-semibold">Project Details</h4>
							<div className="grid grid-cols-2 gap-4">
								<InfoItem
									label="Application Type"
									value={solution.applicationType}
								/>
								<InfoItem
									label="Data Security"
									value={solution.dataSecurity}
								/>
								<InfoItem label="User Type" value={solution.userType} />
								<InfoItem label="User Count" value={solution.userCount} />
								<InfoItem
									label="Technical Level"
									value={solution.technicalLevel}
								/>
							</div>
						</div>

						<Separator />

						<div className="space-y-2">
							<Label className="font-semibold">Description</Label>
							<p className="text-sm text-muted-foreground whitespace-pre-wrap">
								{solution.description}
							</p>
						</div>

						<div className="space-y-2">
							<Label className="font-semibold">Example Input</Label>
							<p className="text-sm text-muted-foreground whitespace-pre-wrap">
								{solution.exampleInput}
							</p>
						</div>

						<div className="space-y-2">
							<Label className="font-semibold">Expected Output</Label>
							<p className="text-sm text-muted-foreground whitespace-pre-wrap">
								{solution.expectedOutput}
							</p>
						</div>

						{solution.additionalNotes && (
							<div className="space-y-2">
								<Label className="font-semibold">Additional Notes</Label>
								<p className="text-sm text-muted-foreground whitespace-pre-wrap">
									{solution.additionalNotes}
								</p>
							</div>
						)}

						<Separator />

						<div className="space-y-4">
							<h4 className="font-semibold">Pricing</h4>
							<div className="grid grid-cols-3 gap-4">
								<InfoItem
									icon={DollarSign}
									label="Total"
									value={formatCurrency(solution.totalCents)}
								/>
								<InfoItem
									icon={DollarSign}
									label="Deposit"
									value={`${formatCurrency(solution.depositCents)} ${solution.paidDeposit ? "(Paid)" : "(Unpaid)"}`}
								/>
								<InfoItem
									icon={DollarSign}
									label="Remainder"
									value={formatCurrency(solution.remainderCents)}
								/>
							</div>
						</div>

						<Separator />

						<div className="space-y-4">
							<h4 className="font-semibold">Admin Info</h4>
							<div className="grid grid-cols-2 gap-4">
								<InfoItem
									label="Assigned To"
									value={solution.assignedTo ?? "Unassigned"}
								/>
								<InfoItem
									label="Delivered At"
									value={
										solution.deliveredAt
											? formatDate(solution.deliveredAt)
											: "Not delivered"
									}
								/>
							</div>
							{solution.adminNotes && (
								<div className="space-y-2">
									<Label className="font-semibold">Admin Notes</Label>
									<p className="text-sm text-muted-foreground whitespace-pre-wrap">
										{solution.adminNotes}
									</p>
								</div>
							)}
						</div>

						<Separator />

						<div className="grid grid-cols-2 gap-4 text-sm text-muted-foreground">
							<InfoItem
								icon={Calendar}
								label="Created"
								value={formatDate(solution.createdAt)}
							/>
							<InfoItem
								icon={Calendar}
								label="Updated"
								value={formatDate(solution.updatedAt)}
							/>
						</div>
					</div>
				</ScrollArea>

				<DialogFooter>
					<Button variant="outline" onClick={onClose}>
						Close
					</Button>
					<Button onClick={() => onEdit(solution)}>
						<Edit className="h-4 w-4 mr-2" />
						Edit
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function InfoItem({
	icon: Icon,
	label,
	value,
}: Readonly<{
	icon?: React.ComponentType<{ className?: string }>;
	label: string;
	value: string;
}>) {
	return (
		<div className="space-y-1">
			<Label className="text-xs text-muted-foreground flex items-center gap-1">
				{Icon && <Icon className="h-3 w-3" />}
				{label}
			</Label>
			<p className="text-sm font-medium">{value}</p>
		</div>
	);
}

function SolutionEditDialog({
	solution,
	onClose,
	onSave,
	isUpdating,
}: Readonly<{
	solution: ISolutionRequest | null;
	onClose: () => void;
	onSave: (update: ISolutionUpdatePayload) => Promise<void>;
	isUpdating: boolean;
}>) {
	const [status, setStatus] = useState<SolutionStatus | undefined>();
	const [adminNotes, setAdminNotes] = useState("");
	const [assignedTo, setAssignedTo] = useState("");
	const [priority, setPriority] = useState(false);

	// Reset form when solution changes
	useMemo(() => {
		if (solution) {
			setStatus(solution.status);
			setAdminNotes(solution.adminNotes ?? "");
			setAssignedTo(solution.assignedTo ?? "");
			setPriority(solution.priority);
		}
	}, [solution]);

	if (!solution) return null;

	const handleSave = async () => {
		const update: ISolutionUpdatePayload = {};
		if (status && status !== solution.status) update.status = status;
		if (adminNotes !== (solution.adminNotes ?? ""))
			update.adminNotes = adminNotes;
		if (assignedTo !== (solution.assignedTo ?? ""))
			update.assignedTo = assignedTo;
		if (priority !== solution.priority) update.priority = priority;

		await onSave(update);
	};

	return (
		<Dialog open={!!solution} onOpenChange={onClose}>
			<DialogContent className="max-w-lg">
				<DialogHeader>
					<DialogTitle>Edit Solution Request</DialogTitle>
					<DialogDescription>
						Update the status and admin details for this solution request.
					</DialogDescription>
				</DialogHeader>

				<div className="space-y-4 py-4">
					<div className="space-y-2">
						<Label>Status</Label>
						<Select
							value={status}
							onValueChange={(value) => setStatus(value as SolutionStatus)}
						>
							<SelectTrigger>
								<SelectValue placeholder="Select status" />
							</SelectTrigger>
							<SelectContent>
								{Object.values(SolutionStatus).map((s) => (
									<SelectItem key={s} value={s}>
										{SolutionStatusLabels[s]}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>

					<div className="space-y-2">
						<Label>Assigned To</Label>
						<Input
							value={assignedTo}
							onChange={(e) => setAssignedTo(e.target.value)}
							placeholder="Enter assignee name or email"
						/>
					</div>

					<div className="space-y-2">
						<Label>Admin Notes</Label>
						<Textarea
							value={adminNotes}
							onChange={(e) => setAdminNotes(e.target.value)}
							placeholder="Internal notes about this request..."
							rows={4}
						/>
					</div>

					<div className="flex items-center justify-between">
						<div className="space-y-0.5">
							<Label>Priority</Label>
							<p className="text-sm text-muted-foreground">
								Mark this request as high priority
							</p>
						</div>
						<Switch checked={priority} onCheckedChange={setPriority} />
					</div>
				</div>

				<DialogFooter>
					<Button variant="outline" onClick={onClose} disabled={isUpdating}>
						Cancel
					</Button>
					<Button onClick={handleSave} disabled={isUpdating}>
						{isUpdating && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
						Save Changes
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
