"use client";
import type { UseQueryResult } from "@tanstack/react-query";
import {
	CheckIcon,
	ClockIcon,
	DownloadCloudIcon,
	ExternalLinkIcon,
	PlusIcon,
	SparklesIcon,
	TrashIcon,
	XIcon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useHub } from "../../hooks/use-hub";
import { useInvoke } from "../../hooks/use-invoke";
import type { IBit } from "../../lib/schema/bit/bit";
import type { IEmbeddingModelParameters } from "../../lib/schema/bit/bit/embedding-model-parameters";
import type {
	IBitModelClassification,
	ILlmParameters,
} from "../../lib/schema/bit/bit/llm-parameters";
import { humanFileSize } from "../../lib/utils";
import { useBackend } from "../../state/backend-state";
import { useDownloadManager } from "../../state/download-manager";
import type { ISettingsProfile } from "../../types";
import { Avatar, AvatarFallback, AvatarImage } from "./avatar";
import { Badge } from "./badge";
import { Button } from "./button";
import {
	isEmbeddingBit,
	ModalityIcons,
	ModelTypeIcon,
	formatContextLength,
	getCapabilityIcon,
	supportsRemoteEmbeddingExecution,
} from "./model-card";
import { Progress } from "./progress";
import {
	Sheet,
	SheetClose,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from "./sheet";
import { Slider } from "./slider";

export interface ModelDetailSheetProps {
	bit: IBit | null;
	open: boolean;
	onOpenChange: (open: boolean) => void;
	webMode?: boolean;
}

export function ModelDetailSheet({
	bit,
	open,
	onOpenChange,
	webMode = false,
}: Readonly<ModelDetailSheetProps>) {
	const backend = useBackend();
	const { hub } = useHub();
	const download = useDownloadManager((s) => s.download);
	const onProgress = useDownloadManager((s) => s.onProgress);
	const isQueued = useDownloadManager((s) => s.isQueued);
	const getLatestPct = useDownloadManager((s) => s.getLatestPct);

	const [progress, setProgress] = useState<number | undefined>();
	const isQueuedState = useMemo(() => progress === 0, [progress]);

	const mountedRef = useRef(true);
	const lastPctRef = useRef(0);
	const lastUpdateRef = useRef(0);
	const unsubscribeRef = useRef<(() => void) | null>(null);

	useEffect(() => {
		if (!bit) return;
		mountedRef.current = true;
		const bitHash = bit.hash;
		const initial = getLatestPct(bitHash);
		if (typeof initial === "number") {
			setProgress(initial);
			lastPctRef.current = initial;
		} else if (isQueued(bitHash)) {
			setProgress(0);
			lastPctRef.current = 0;
		}

		unsubscribeRef.current = onProgress(bitHash, (dl) => {
			const rawProgress = dl.progress();
			const pct = Math.round(rawProgress * 100);
			const now = Date.now();
			const changed = Math.abs(pct - lastPctRef.current) >= 1;
			const due = now - lastUpdateRef.current >= 250;
			const completed =
				rawProgress >= 0.999 || dl.total().downloaded >= dl.total().max;
			if (!mountedRef.current) return;
			if (completed) {
				setProgress(undefined);
				lastPctRef.current = 0;
				lastUpdateRef.current = now;
				return;
			}
			if (changed || due) {
				setProgress(pct);
				lastPctRef.current = pct;
				lastUpdateRef.current = now;
			}
		});

		return () => {
			mountedRef.current = false;
			if (unsubscribeRef.current) {
				unsubscribeRef.current();
				unsubscribeRef.current = null;
			}
			lastPctRef.current = 0;
			lastUpdateRef.current = 0;
			setProgress(undefined);
		};
	}, [bit, getLatestPct, isQueued, onProgress]);

	const isInstalled: UseQueryResult<boolean> = useInvoke(
		backend.bitState.isBitInstalled,
		backend.bitState,
		// biome-ignore lint/style/noNonNullAssertion: bit is guaranteed by enabled flag
		[bit!],
		!!bit,
	);
	const bitSize: UseQueryResult<number> = useInvoke(
		backend.bitState.getBitSize,
		backend.bitState,
		// biome-ignore lint/style/noNonNullAssertion: bit is guaranteed by enabled flag
		[bit!],
		!!bit,
	);
	const currentProfile: UseQueryResult<ISettingsProfile> = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
	);
	const userInfo = useInvoke(backend.userState.getInfo, backend.userState, []);

	const isVirtualBit = useMemo(
		() => !bit?.download_link || (bitSize.data === 0 && bitSize.isSuccess),
		[bit?.download_link, bitSize.data, bitSize.isSuccess],
	);

	const tierInfo = useMemo(() => {
		if (!bit) return { isRestricted: false, requiredTier: null };
		const params = bit.parameters as {
			provider?: { params?: { tier?: string } };
		};
		const modelTier = params?.provider?.params?.tier;
		if (!modelTier || !hub?.tiers) {
			return { isRestricted: false, requiredTier: null };
		}
		const userTierKey = (userInfo.data?.tier ?? "FREE").toUpperCase();
		const userTierConfig = hub.tiers[userTierKey];
		if (!userTierConfig) {
			return { isRestricted: true, requiredTier: modelTier };
		}
		const allowedModelTiers = userTierConfig.llm_tiers ?? [];
		const isRestricted = !allowedModelTiers.includes(modelTier);
		return { isRestricted, requiredTier: isRestricted ? modelTier : null };
	}, [bit, hub?.tiers, userInfo.data?.tier]);

	const downloadBit = useCallback(
		async (b: IBit) => {
			if (!b.download_link || isVirtualBit) {
				await isInstalled.refetch();
				return;
			}
			setProgress(0);
			try {
				await download(b);
				await isInstalled.refetch();
			} finally {
				if (mountedRef.current) {
					setProgress(undefined);
					lastPctRef.current = 0;
					lastUpdateRef.current = 0;
				}
			}
		},
		[download, isInstalled, isVirtualBit],
	);

	const refetchIsInstalled = isInstalled.refetch;
	const handleDownload = useCallback(async () => {
		if (!bit) return;
		if (isInstalled.data) {
			await backend.bitState.deleteBit(bit);
			await refetchIsInstalled();
			return;
		}
		await downloadBit(bit);
	}, [
		bit,
		isInstalled.data,
		backend.bitState,
		downloadBit,
		refetchIsInstalled,
	]);

	const refetchCurrentProfile = currentProfile.refetch;
	const handleToggleProfile = useCallback(async () => {
		if (!bit) return;
		const profile = currentProfile.data;
		if (!profile) return;
		const bitIndex = profile.hub_profile.bits.findIndex(
			(id) => id.split(":").pop() === bit.id,
		);
		if (bitIndex === -1) {
			await downloadBit(bit);
			await backend.bitState.addBit(bit, profile);
		} else {
			await backend.bitState.removeBit(bit, profile);
		}
		await refetchCurrentProfile();
	}, [
		bit,
		currentProfile.data,
		downloadBit,
		backend.bitState,
		refetchCurrentProfile,
	]);

	if (!bit || !bit.meta.en) return null;

	const meta = bit.meta.en;
	const isInProfile =
		(currentProfile.data?.hub_profile.bits || []).findIndex(
			(id) => id.split(":")[1] === bit.id,
		) > -1;

	const params = bit.parameters as ILlmParameters | IEmbeddingModelParameters;
	const classification = (params as ILlmParameters)?.model_classification;
	const contextLength = (params as ILlmParameters)?.context_length;
	const embeddingParams = params as IEmbeddingModelParameters;
	const isHosted = bitSize.data === 0 || isVirtualBit;
	const canRunRemotely = supportsRemoteEmbeddingExecution(bit);
	const isEmbeddingModel = isEmbeddingBit(bit);

	return (
		<Sheet open={open} onOpenChange={onOpenChange}>
			<SheetContent className="overflow-y-auto px-4 md:min-w-2/5">
				<SheetHeader className="pb-4">
					<div className="flex items-start gap-3">
						<Avatar className="h-12 w-12 border">
							<AvatarImage src={meta.icon ?? "/app-logo.webp"} />
							<AvatarFallback>
								<ModelTypeIcon type={bit.type} className="h-5 w-5" />
							</AvatarFallback>
						</Avatar>
						<div className="flex-1 min-w-0">
							<SheetTitle className="flex items-center gap-2 text-lg">
								{meta.name}
								{isInProfile && (
									<SparklesIcon className="h-4 w-4 text-primary" />
								)}
							</SheetTitle>
							<SheetDescription>
								<ModalityIcons type={bit.type} />
							</SheetDescription>
						</div>
					</div>
					<SheetClose />
				</SheetHeader>

				<div className="space-y-6">
					{/* Download Progress */}
					{progress !== undefined && !isVirtualBit && (
						<div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50">
							{isQueuedState ? (
								<>
									<ClockIcon className="h-4 w-4 text-primary animate-pulse" />
									<span className="text-sm">Queued for download...</span>
								</>
							) : (
								<>
									<Progress value={progress} className="flex-1 h-2" />
									<span className="text-sm tabular-nums w-12 text-right">
										{progress}%
									</span>
								</>
							)}
						</div>
					)}

					{/* Status Badges */}
					<div className="flex flex-wrap gap-2">
						{isHosted ? (
							<Badge className="bg-sky-500/10 text-sky-600 border-sky-500/30">
								Hosted
							</Badge>
						) : isInstalled.data ? (
							<Badge className="bg-emerald-500/10 text-emerald-600 border-emerald-500/30">
								<CheckIcon className="h-3 w-3 mr-1" />
								Installed
							</Badge>
						) : (
							<Badge variant="outline">
								<DownloadCloudIcon className="h-3 w-3 mr-1" />
								{humanFileSize(bitSize.data ?? 0)}
							</Badge>
						)}
						{contextLength && (
							<Badge variant="outline">
								{formatContextLength(contextLength)}
							</Badge>
						)}
						{canRunRemotely && (
							<Badge className="bg-cyan-500/10 text-cyan-700 border-cyan-500/30">
								Remote
							</Badge>
						)}
						{isEmbeddingModel && !canRunRemotely && (
							<Badge className="bg-zinc-500/10 text-zinc-600 border-zinc-500/30">
								Local only
							</Badge>
						)}
						{tierInfo.isRestricted && tierInfo.requiredTier && (
							<Badge className="bg-amber-500/10 text-amber-600 border-amber-500/30">
								{tierInfo.requiredTier} Required
							</Badge>
						)}
					</div>

					{/* Description */}
					<div>
						<h4 className="text-sm font-medium mb-2">Description</h4>
						<p className="text-sm text-muted-foreground">{meta.description}</p>
					</div>

					{/* Capabilities */}
					{classification && (
						<ModelCapabilities classification={classification} />
					)}

					{/* Embedding Parameters */}
					{embeddingParams?.vector_length && (
						<div>
							<h4 className="text-sm font-medium mb-2">Embedding Details</h4>
							<div className="grid grid-cols-2 gap-2 text-sm">
								<div className="flex justify-between p-2 rounded bg-muted/50">
									<span className="text-muted-foreground">Vector Length</span>
									<span>{embeddingParams.vector_length}</span>
								</div>
								{embeddingParams.input_length && (
									<div className="flex justify-between p-2 rounded bg-muted/50">
										<span className="text-muted-foreground">Max Input</span>
										<span>{embeddingParams.input_length}</span>
									</div>
								)}
							</div>
						</div>
					)}

					{/* Tags */}
					{meta.tags.length > 0 && (
						<div>
							<h4 className="text-sm font-medium mb-2">Tags</h4>
							<div className="flex flex-wrap gap-1.5">
								{meta.tags.map((tag) => (
									<Badge key={tag} variant="outline" className="text-xs">
										{tag}
									</Badge>
								))}
							</div>
						</div>
					)}

					{/* Actions */}
					<div className="flex flex-col gap-2 pt-2">
						{!webMode && !isHosted && (
							<Button
								onClick={handleDownload}
								variant={isInstalled.data ? "destructive" : "default"}
								className="w-full"
								disabled={progress !== undefined}
							>
								{isInstalled.data ? (
									<>
										<TrashIcon className="h-4 w-4 mr-2" />
										Remove Download
									</>
								) : (
									<>
										<DownloadCloudIcon className="h-4 w-4 mr-2" />
										Download ({humanFileSize(bitSize.data ?? 0)})
									</>
								)}
							</Button>
						)}
						<Button
							onClick={handleToggleProfile}
							variant="outline"
							className="w-full"
						>
							{isInProfile ? (
								<>
									<XIcon className="h-4 w-4 mr-2" />
									Remove from Profile
								</>
							) : (
								<>
									<PlusIcon className="h-4 w-4 mr-2" />
									Add to Profile
								</>
							)}
						</Button>
						{bit.repository && (
							<Button
								variant="ghost"
								className="w-full"
								onClick={() => window.open(bit.repository ?? "", "_blank")}
							>
								<ExternalLinkIcon className="h-4 w-4 mr-2" />
								View Repository
							</Button>
						)}
					</div>
				</div>
			</SheetContent>
		</Sheet>
	);
}

function ModelCapabilities({
	classification,
}: Readonly<{ classification: IBitModelClassification }>) {
	const capabilities = Object.entries(classification).filter(
		([_, value]) => typeof value === "number",
	) as [string, number][];

	if (capabilities.length === 0) return null;

	return (
		<div>
			<h4 className="text-sm font-medium mb-3">Capabilities</h4>
			<div className="space-y-5">
				{capabilities.map(([key, value]) => {
					const { icon, label, color } = getCapabilityIcon(key);
					return (
						<div key={key} className="space-y-1">
							<div className="flex items-center justify-between text-sm">
								<span className="flex items-center gap-1.5">
									<span>{icon}</span>
									<span className="text-muted-foreground">{label}</span>
								</span>
								<span className={`font-medium ${color}`}>
									{Math.round(value * 100)}%
								</span>
							</div>
							<Slider
								value={[value * 100]}
								max={100}
								step={1}
								disabled
								className="pointer-events-none"
							/>
						</div>
					);
				})}
			</div>
		</div>
	);
}
