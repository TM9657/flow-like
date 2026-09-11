"use client";

import {
	Alert,
	AlertDescription,
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	IAppVisibility,
	Input,
	Label,
	RadioGroup,
	RadioGroupItem,
} from "@flow-like/flow-like-ui";
import { humanFileSize } from "@flow-like/flow-like-ui/lib/utils";
import { useTranslation } from "@flow-like/locales";
import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { TFunction } from "i18next";
import {
	EyeIcon,
	EyeOffIcon,
	InfoIcon,
	Loader2Icon,
	PackageIcon,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { appsDB } from "../../../lib/apps-db";
import {
	type IArchiveInfo,
	type IArchiveProgress,
	type IImportReport,
	type ImportMode,
	cancelArchiveOperation,
	describeArchiveError,
	isArchiveCancelled,
	isWrongPassword,
	listenArchiveProgress,
	newOperationId,
} from "../../../lib/archive-operations";
import { ArchiveProgressView } from "./ArchiveProgressView";

export interface ImportArchiveDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	path: string | null;
	onImported: () => Promise<void> | void;
}

type Step =
	| { kind: "inspect" }
	| { kind: "password"; error: string | null; verifying: boolean }
	| { kind: "mode"; info: IArchiveInfo }
	| { kind: "progress"; progress: IArchiveProgress | null; cancelling: boolean }
	| { kind: "done" };

function inspectArchive(path: string, password?: string) {
	return invoke<IArchiveInfo>("inspect_app_archive", {
		path,
		...(password ? { password } : {}),
	});
}

function describeImportReport(
	t: TFunction<"common">,
	report: IImportReport,
): string {
	const parts = [
		t("countFilesRestored", {
			defaultValue_one: "{{count}} file restored",
			defaultValue_other: "{{count}} files restored",
			count: report.restored_files,
		}),
	];
	if (report.deleted_files > 0) {
		parts.push(
			t("countFilesDeleted", {
				defaultValue_one: "{{count}} file deleted",
				defaultValue_other: "{{count}} files deleted",
				count: report.deleted_files,
			}),
		);
	}
	return parts.join(" · ");
}

function stepDescription(t: TFunction<"common">, step: Step): string {
	switch (step.kind) {
		case "inspect":
			return t("inspectingArchive", "Inspecting archive…");
		case "password":
			return t(
				"thisArchiveIsEncryptedEnterThePasswordToContinue",
				"This archive is encrypted. Enter the password to continue.",
			);
		case "mode":
			return t(
				"thisAppAlreadyExistsLocallyChooseHowToImportIt",
				"This app already exists locally. Choose how to import it.",
			);
		case "progress":
			return t("importingApp", "Importing app...");
		case "done":
			return t("done", "Done");
	}
}

function Spinner() {
	return (
		<div className="flex items-center justify-center py-6">
			<Loader2Icon className="h-6 w-6 animate-spin text-muted-foreground" />
		</div>
	);
}

function ArchiveSummary({ info }: { info: IArchiveInfo }) {
	const { t } = useTranslation("common");
	if (info.file_count == null) return null;
	return (
		<p className="text-xs text-muted-foreground">
			{t("countFiles", {
				defaultValue_one: "{{count}} file",
				defaultValue_other: "{{count}} files",
				count: info.file_count,
			})}
			{info.total_bytes != null && ` · ${humanFileSize(info.total_bytes)}`}
		</p>
	);
}

function ModeOption({
	value,
	title,
	description,
}: {
	value: ImportMode;
	title: string;
	description: string;
}) {
	const id = `import-mode-${value}`;
	return (
		<Label
			htmlFor={id}
			className="flex cursor-pointer items-start gap-3 rounded-lg border p-3 has-[[data-state=checked]]:border-primary"
		>
			<RadioGroupItem id={id} value={value} className="mt-0.5" />
			<span className="grid gap-1">
				<span className="text-sm font-medium">{title}</span>
				<span className="text-xs font-normal text-muted-foreground">
					{description}
				</span>
			</span>
		</Label>
	);
}

function ModeStep({
	info,
	mode,
	onModeChange,
}: {
	info: IArchiveInfo;
	mode: ImportMode;
	onModeChange: (mode: ImportMode) => void;
}) {
	const { t } = useTranslation("common");
	const keepsVisibility =
		info.local_visibility != null &&
		info.local_visibility !== IAppVisibility.Offline;
	return (
		<div className="space-y-3">
			<ArchiveSummary info={info} />
			<RadioGroup
				value={mode}
				onValueChange={(next) => onModeChange(next as ImportMode)}
				className="gap-2"
			>
				<ModeOption
					value="merge"
					title={t("merge", "Merge")}
					description={t(
						"keepsLocalFilesThatAreNotInTheArchiveAndOverwritesMatchingOnes",
						"Keeps local files that are not in the archive and overwrites matching ones.",
					)}
				/>
				<ModeOption
					value="replace"
					title={t("replace", "Replace")}
					description={t(
						"makesTheLocalAppIdenticalToTheArchiveLocalFilesNotInTheArchiveAreDeleted",
						"Makes the local app identical to the archive. Local files not in the archive are deleted.",
					)}
				/>
			</RadioGroup>
			{keepsVisibility && (
				<Alert>
					<InfoIcon />
					<AlertDescription>
						{t(
							"theLocalCopyKeepsItsCurrentVisibility",
							"The local copy keeps its current visibility.",
						)}
					</AlertDescription>
				</Alert>
			)}
		</div>
	);
}

function PasswordStep({
	password,
	showPassword,
	error,
	inputRef,
	onPasswordChange,
	onToggleShowPassword,
}: {
	password: string;
	showPassword: boolean;
	error: string | null;
	inputRef: React.RefObject<HTMLInputElement | null>;
	onPasswordChange: (value: string) => void;
	onToggleShowPassword: () => void;
}) {
	const { t } = useTranslation("common");
	return (
		<div className="grid gap-2">
			<Label htmlFor="import-password" className="text-xs">
				{t("password", "Password")}
			</Label>
			<div className="relative">
				<Input
					ref={inputRef}
					id="import-password"
					type={showPassword ? "text" : "password"}
					value={password}
					onChange={(e) => onPasswordChange(e.target.value)}
					placeholder={t("enterPassword", "Enter password")}
					aria-invalid={error != null}
					autoFocus
				/>
				<Button
					type="button"
					variant="ghost"
					size="icon"
					className="absolute right-1 top-1 h-7 w-7"
					onClick={onToggleShowPassword}
					aria-label={
						showPassword
							? t("hidePassword", "Hide password")
							: t("showPassword", "Show password")
					}
				>
					{showPassword ? (
						<EyeOffIcon className="w-4 h-4" />
					) : (
						<EyeIcon className="w-4 h-4" />
					)}
				</Button>
			</div>
			{error && <p className="text-xs text-destructive">{error}</p>}
		</div>
	);
}

const ImportArchiveDialog: React.FC<ImportArchiveDialogProps> = ({
	open,
	onOpenChange,
	path,
	onImported,
}) => {
	const { t } = useTranslation("common");
	const [step, setStep] = useState<Step>({ kind: "inspect" });
	const [password, setPassword] = useState("");
	const [showPassword, setShowPassword] = useState(false);
	const [mode, setMode] = useState<ImportMode>("merge");
	const sessionRef = useRef(0);
	const operationRef = useRef<string | null>(null);
	const passwordRef = useRef<HTMLInputElement>(null);
	const callbacksRef = useRef({ onOpenChange, onImported });

	useEffect(() => {
		callbacksRef.current = { onOpenChange, onImported };
	});

	const importing = step.kind === "progress";
	const isCurrent = useCallback(
		(session: number) => session === sessionRef.current,
		[],
	);

	const runImport = useCallback(
		async (archivePassword: string | undefined, importMode: ImportMode) => {
			if (!path) return;
			const operationId = newOperationId();
			operationRef.current = operationId;
			setStep({ kind: "progress", progress: null, cancelling: false });
			let unlisten: UnlistenFn | null = null;
			const release = () => {
				unlisten?.();
				unlisten = null;
				if (operationRef.current === operationId) operationRef.current = null;
			};
			let report: IImportReport;
			try {
				unlisten = await listenArchiveProgress(operationId, (progress) => {
					if (operationRef.current !== operationId) return;
					setStep((prev) =>
						prev.kind === "progress" ? { ...prev, progress } : prev,
					);
				});
				report = await invoke<IImportReport>("import_app_from_file", {
					path,
					...(archivePassword ? { password: archivePassword } : {}),
					mode: importMode,
					operationId,
				});
				release();
				await appsDB.visibility.put({
					appId: report.app.id,
					visibility: report.app.visibility ?? IAppVisibility.Offline,
				});
			} catch (error) {
				release();
				if (isArchiveCancelled(error)) {
					toast.info(t("importCancelled", "Import cancelled"));
				} else {
					console.error("Import error:", error);
					toast.error(t("failedToImportApp", "Failed to import app"), {
						description: describeArchiveError(error),
					});
				}
				callbacksRef.current.onOpenChange(false);
				return;
			}
			toast.success(
				t("appImportedSuccessfully", "App imported successfully!"),
				{
					description: describeImportReport(t, report),
				},
			);
			setStep({ kind: "done" });
			callbacksRef.current.onOpenChange(false);
			try {
				await callbacksRef.current.onImported();
			} catch (error) {
				console.error("Refresh after import failed:", error);
			}
		},
		[path, t],
	);

	const proceed = useCallback(
		async (archivePassword: string | undefined, info: IArchiveInfo) => {
			if (info.encrypted && archivePassword === undefined) {
				setStep({ kind: "password", error: null, verifying: false });
				return;
			}
			if (info.exists_locally) {
				setMode("merge");
				setStep({ kind: "mode", info });
				return;
			}
			await runImport(archivePassword, "merge");
		},
		[runImport],
	);

	useEffect(() => {
		if (!open || !path || operationRef.current) return;
		const session = ++sessionRef.current;
		setStep({ kind: "inspect" });
		setPassword("");
		setShowPassword(false);
		setMode("merge");
		inspectArchive(path)
			.then((info) => {
				if (isCurrent(session)) return proceed(undefined, info);
			})
			.catch((error) => {
				if (!isCurrent(session)) return;
				console.error("Archive inspect error:", error);
				toast.error(t("couldNotReadArchive", "Could not read archive"), {
					description: describeArchiveError(error),
				});
				callbacksRef.current.onOpenChange(false);
			});
		return () => {
			sessionRef.current++;
		};
	}, [open, path, proceed, isCurrent, t]);

	const handlePasswordSubmit = useCallback(
		async (event: React.FormEvent) => {
			event.preventDefault();
			if (!path || step.kind !== "password" || step.verifying || !password) {
				return;
			}
			const session = sessionRef.current;
			setStep({ kind: "password", error: null, verifying: true });
			try {
				const info = await inspectArchive(path, password);
				if (!isCurrent(session)) return;
				await proceed(password, info);
			} catch (error) {
				if (!isCurrent(session)) return;
				setStep({
					kind: "password",
					error: isWrongPassword(error)
						? t("wrongPassword", "Wrong password")
						: describeArchiveError(error),
					verifying: false,
				});
				passwordRef.current?.focus();
				passwordRef.current?.select();
			}
		},
		[path, step, password, proceed, isCurrent, t],
	);

	const handleModeConfirm = useCallback(() => {
		if (step.kind !== "mode") return;
		void runImport(password || undefined, mode);
	}, [step, runImport, password, mode]);

	const handleCancelImport = useCallback(async () => {
		const operationId = operationRef.current;
		if (!operationId) return;
		const setCancelling = (cancelling: boolean) =>
			setStep((prev) =>
				prev.kind === "progress" ? { ...prev, cancelling } : prev,
			);
		setCancelling(true);
		try {
			await cancelArchiveOperation(operationId);
		} catch (error) {
			console.error("Cancel import error:", error);
			setCancelling(false);
		}
	}, []);

	const handleOpenChange = useCallback(
		(next: boolean) => {
			if (!next && importing) return;
			onOpenChange(next);
		},
		[importing, onOpenChange],
	);

	const closeButton = (
		<Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
			{t("cancel", "Cancel")}
		</Button>
	);

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent className="sm:max-w-[480px]">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<PackageIcon className="h-4 w-4 text-primary" />
						{t("importApp", "Import app")}
					</DialogTitle>
					<DialogDescription>{stepDescription(t, step)}</DialogDescription>
				</DialogHeader>

				{(step.kind === "inspect" || step.kind === "done") && <Spinner />}

				{step.kind === "password" && (
					<form
						id="import-password-form"
						onSubmit={handlePasswordSubmit}
						className="space-y-4"
					>
						<PasswordStep
							password={password}
							showPassword={showPassword}
							error={step.error}
							inputRef={passwordRef}
							onPasswordChange={setPassword}
							onToggleShowPassword={() => setShowPassword((s) => !s)}
						/>
					</form>
				)}

				{step.kind === "mode" && (
					<ModeStep info={step.info} mode={mode} onModeChange={setMode} />
				)}

				{step.kind === "progress" && (
					<ArchiveProgressView progress={step.progress} />
				)}

				{step.kind === "password" && (
					<DialogFooter className="gap-2">
						{closeButton}
						<Button
							type="submit"
							form="import-password-form"
							disabled={step.verifying || password === ""}
						>
							{step.verifying && (
								<Loader2Icon className="h-4 w-4 animate-spin" />
							)}
							{t("import", "Import")}
						</Button>
					</DialogFooter>
				)}

				{step.kind === "mode" && (
					<DialogFooter className="gap-2">
						{closeButton}
						<Button type="button" onClick={handleModeConfirm}>
							{t("import", "Import")}
						</Button>
					</DialogFooter>
				)}

				{step.kind === "progress" && (
					<DialogFooter className="gap-2">
						<Button
							type="button"
							variant="outline"
							onClick={handleCancelImport}
							disabled={step.cancelling}
						>
							{step.cancelling
								? t("cancelling", "Cancelling…")
								: t("cancel", "Cancel")}
						</Button>
					</DialogFooter>
				)}
			</DialogContent>
		</Dialog>
	);
};

export default ImportArchiveDialog;
