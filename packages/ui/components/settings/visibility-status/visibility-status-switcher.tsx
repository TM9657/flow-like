"use client";

import {
	AlertTriangleIcon,
	ArrowRightIcon,
	ExternalLinkIcon,
	EyeIcon,
	InfoIcon,
	SettingsIcon,
	ShieldIcon,
	UsersIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import { useCallback } from "react";
import { toast } from "sonner";
import type { IApp } from "../../../types";
import { IAppVisibility } from "../../../types";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from "../../ui/alert-dialog";
import { Button } from "../../ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../ui/card";

export interface VisibilityStatusSwitcherProps {
	localApp: IApp;
	canEdit: boolean;
	onVisibilityChange: (
		appId: string,
		newVisibility: IAppVisibility,
	) => Promise<void>;
	docsUrl?: string;
}

export function VisibilityStatusSwitcher({
	localApp,
	canEdit,
	onVisibilityChange,
	docsUrl = "https://docs.flow-like.com/guides/Apps/visibility/",
}: Readonly<VisibilityStatusSwitcherProps>) {
	const switchVisibility = useCallback(
		async (newVisibility: IAppVisibility) => {
			if (localApp.visibility === newVisibility) {
				return;
			}
			await onVisibilityChange(localApp.id, newVisibility);
		},
		[localApp.visibility, localApp.id, onVisibilityChange],
	);

	const getVisibilityConfig = (visibility: IAppVisibility) => {
		switch (visibility) {
			case IAppVisibility.Offline:
				return {
					icon: <ShieldIcon className="w-4 h-4" />,
					title: "Offline",
					color: "bg-slate-500",
					description: "Only local, no syncing across devices",
				};
			case IAppVisibility.Private:
				return {
					icon: <EyeIcon className="w-4 h-4" />,
					title: "Private",
					color: "bg-blue-500",
					description: "Synced for your account only",
				};
			case IAppVisibility.Prototype:
				return {
					icon: <SettingsIcon className="w-4 h-4" />,
					title: "Prototype",
					color: "bg-yellow-500",
					description: "Development phase, invite collaborators",
				};
			case IAppVisibility.PublicRequestAccess:
				return {
					icon: <InfoIcon className="w-4 h-4" />,
					title: "Public Request",
					color: "bg-orange-500",
					description: "Visible, people can request to join",
				};
			case IAppVisibility.Public:
				return {
					icon: <ExternalLinkIcon className="w-4 h-4" />,
					title: "Public",
					color: "bg-emerald-500",
					description: "Everyone can join, visible in store",
				};
		}
	};

	const getAvailableTransitions = (
		currentVisibility: IAppVisibility,
	): IAppVisibility[] => {
		switch (currentVisibility) {
			case IAppVisibility.Offline:
				return [];
			case IAppVisibility.Private:
				return [IAppVisibility.Prototype];
			case IAppVisibility.Prototype:
				return [
					IAppVisibility.Private,
					IAppVisibility.PublicRequestAccess,
					IAppVisibility.Public,
				];
			case IAppVisibility.PublicRequestAccess:
				return [IAppVisibility.Prototype];
			case IAppVisibility.Public:
				return [IAppVisibility.Prototype];
			default:
				return [];
		}
	};

	const getTransitionWarning = (
		from: IAppVisibility,
		to: IAppVisibility,
	): {
		title: string;
		message: string;
		severity: "warning" | "danger" | "info";
	} => {
		if (from === IAppVisibility.Prototype && to === IAppVisibility.Private) {
			return {
				title: "Remove All Collaborators",
				message:
					"Switching to Private will remove all collaborators from your project. They will lose access immediately.",
				severity: "warning",
			};
		}

		if (
			from === IAppVisibility.Prototype &&
			(to === IAppVisibility.Public ||
				to === IAppVisibility.PublicRequestAccess)
		) {
			return {
				title: "Submit for Review",
				message:
					"Your app will be submitted for central revision. This process may take 1-3 business days. You'll be notified once the review is complete.",
				severity: "info",
			};
		}

		if (
			(from === IAppVisibility.Public ||
				from === IAppVisibility.PublicRequestAccess) &&
			to === IAppVisibility.Prototype
		) {
			return {
				title: "Return to Development",
				message:
					"Your app will be removed from public visibility and submitted for central revision to return to prototype status. This may take 1-3 business days.",
				severity: "warning",
			};
		}

		return {
			title: "Change Visibility",
			message: "Are you sure you want to change the visibility status?",
			severity: "info",
		};
	};

	const confirmVisibilityChange = (newVisibility: IAppVisibility) => {
		switchVisibility(newVisibility);

		const config = getVisibilityConfig(newVisibility);
		toast.success(`Visibility changed to ${config.title}`, {
			icon: config.icon,
		});
	};

	const currentVisibility = localApp.visibility ?? IAppVisibility.Offline;
	const availableTransitions = getAvailableTransitions(currentVisibility);
	const currentConfig = getVisibilityConfig(currentVisibility);

	if (!canEdit) {
		return null;
	}

	return (
		<Card>
			<CardHeader>
				<CardTitle className="flex items-center gap-2">
					<EyeIcon className="w-5 h-5" />
					Visibility Status
				</CardTitle>
				<CardDescription>
					Control who can access your app and how it&apos;s shared.{" "}
					<a href={docsUrl} target="_blank" rel="noreferrer">
						<Button
							variant="link"
							className="h-auto p-0 text-xs text-muted-foreground hover:text-foreground"
						>
							Learn more about visibility statuses
							<ExternalLinkIcon className="w-3 h-3 ml-1" />
						</Button>
					</a>
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-4">
				{/* Current Status */}
				<div className="flex items-center gap-3 p-4 bg-muted rounded-lg border">
					<div className={`w-3 h-3 rounded-full ${currentConfig.color}`} />
					<div>
						<div className="font-medium">Current: {currentConfig.title}</div>
						<div className="text-sm text-muted-foreground">
							{currentConfig.description}
						</div>
					</div>
				</div>

				{/* Available Transitions */}
				{availableTransitions.length > 0 ? (
					<div className="space-y-3">
						<div className="text-sm font-medium text-muted-foreground">
							Available transitions:
						</div>
						<div className="grid gap-2">
							{availableTransitions.map((visibility) => {
								const config = getVisibilityConfig(visibility);
								const warning = getTransitionWarning(
									currentVisibility,
									visibility,
								);

								return (
									<CustomVerificationDialog
										key={visibility}
										title={warning.title}
										description={warning.message}
										severity={warning.severity}
										confirmText="Change Visibility"
										onConfirm={() => confirmVisibilityChange(visibility)}
										content={
											<div className="flex items-center justify-center gap-2 p-3 bg-muted rounded-lg">
												<div
													className={`w-2 h-2 rounded-full ${currentConfig.color}`}
												/>
												<span className="text-sm font-medium">
													{currentConfig.title}
												</span>
												<ArrowRightIcon className="w-4 h-4 text-muted-foreground" />
												<div
													className={`w-2 h-2 rounded-full ${config.color}`}
												/>
												<span className="text-sm font-medium">
													{config.title}
												</span>
											</div>
										}
									>
										<Button
											variant="outline"
											className="w-full justify-between group hover:bg-muted/50 transition-colors h-fit"
										>
											<div className="flex items-center gap-3">
												<div
													className={`w-3 h-3 rounded-full ${config.color}`}
												/>
												<div className="text-left">
													<div className="font-medium">{config.title}</div>
													<div className="text-xs text-muted-foreground">
														{config.description}
													</div>
												</div>
											</div>
											<ArrowRightIcon className="w-4 h-4 opacity-0 group-hover:opacity-100 transition-opacity" />
										</Button>
									</CustomVerificationDialog>
								);
							})}
						</div>
					</div>
				) : (
					<div className="p-4 bg-muted/50 rounded-lg border-2 border-dashed border-muted-foreground/25">
						<div className="flex items-center gap-2 text-muted-foreground">
							<InfoIcon className="w-4 h-4" />
							<span className="text-sm">
								{currentVisibility === IAppVisibility.Offline
									? "No transitions available from Offline status"
									: "No transitions available from current status"}
							</span>
						</div>
					</div>
				)}

				{/* Info about restrictions */}
				<div className="text-xs text-muted-foreground space-y-1 border-t pt-3">
					<div className="flex items-center gap-1">
						<ShieldIcon className="w-3 h-3" />
						<span>Offline apps cannot change visibility status</span>
					</div>
					<div className="flex items-center gap-1">
						<UsersIcon className="w-3 h-3" />
						<span>Public transitions require central review (1-3 days)</span>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}

interface CustomVerificationDialogProps {
	children: ReactNode;
	title: string;
	description: string;
	severity: "warning" | "danger" | "info";
	confirmText?: string;
	cancelText?: string;
	onConfirm: () => void;
	content?: ReactNode;
}

function CustomVerificationDialog({
	children,
	title,
	description,
	severity,
	confirmText = "Confirm",
	cancelText = "Cancel",
	onConfirm,
	content,
}: Readonly<CustomVerificationDialogProps>) {
	const getSeverityConfig = () => {
		switch (severity) {
			case "danger":
				return {
					icon: <AlertTriangleIcon className="h-5 w-5 text-destructive" />,
					iconBg: "bg-destructive/10",
					buttonVariant: "destructive" as const,
				};
			case "warning":
				return {
					icon: <AlertTriangleIcon className="h-5 w-5 text-orange-500" />,
					iconBg: "bg-orange-50 dark:bg-orange-950",
					buttonVariant: "default" as const,
				};
			case "info":
			default:
				return {
					icon: <InfoIcon className="h-5 w-5 text-blue-500" />,
					iconBg: "bg-blue-50 dark:bg-blue-950",
					buttonVariant: "default" as const,
				};
		}
	};

	const config = getSeverityConfig();

	return (
		<AlertDialog>
			<AlertDialogTrigger asChild>{children}</AlertDialogTrigger>
			<AlertDialogContent className="sm:max-w-[425px]">
				<AlertDialogHeader>
					<div className="flex items-center gap-3">
						<div className={`p-2 rounded-full ${config.iconBg}`}>
							{config.icon}
						</div>
						<AlertDialogTitle className="text-left">{title}</AlertDialogTitle>
					</div>
					<AlertDialogDescription className="text-left text-muted-foreground">
						{description}
					</AlertDialogDescription>
				</AlertDialogHeader>
				{content && <div className="py-4">{content}</div>}
				<AlertDialogFooter className="flex-col sm:flex-row gap-2">
					<AlertDialogCancel asChild>
						<Button variant="outline" className="w-full sm:w-auto">
							{cancelText}
						</Button>
					</AlertDialogCancel>
					<AlertDialogAction asChild>
						<Button
							variant={config.buttonVariant}
							onClick={onConfirm}
							className="w-full sm:w-auto"
						>
							{confirmText}
						</Button>
					</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}
