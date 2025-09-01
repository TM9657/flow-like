"use client";
import { Bot, FileText, Layout, Rocket, Settings } from "lucide-react";
import { useRouter } from "next/navigation";
import { useState } from "react";
import Crossfire from "react-canvas-confetti/dist/presets/crossfire";
import { useAuth } from "react-oidc-context";
import { toast } from "sonner";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	nowSystemTime,
	useBackend,
	useInvoke,
} from "../../..";
import type { IMetadata } from "../../../lib";
import { ModelModal } from "./model-modal";
import { ModelSection } from "./model-selection";
import { StepNavigation } from "./navigation-steps";
import { ProgressBar, StepIndicator } from "./progress-indicator";
import { ConnectivityStep } from "./steps/connectivity";
import { AppDetailsStep } from "./steps/details";
import { ReviewStep } from "./steps/review";
import { TemplateModal } from "./template-modal";
import { TemplateSection } from "./template-selection";

export type ICreationStep = {
	id: string;
	title: string;
	description: string;
	icon: React.ComponentType<any>;
	required: boolean;
};

const STEPS: ICreationStep[] = [
	{
		id: "details",
		title: "App Details",
		description: "Name and describe your app",
		icon: FileText,
		required: true,
	},
	{
		id: "connectivity",
		title: "Connectivity",
		description: "Choose online or offline mode",
		icon: Settings,
		required: true,
	},
	{
		id: "template",
		title: "Template",
		description: "Select a starting template",
		icon: Layout,
		required: false,
	},
	{
		id: "models",
		title: "AI Models",
		description: "Configure AI capabilities",
		icon: Bot,
		required: false,
	},
	{
		id: "review",
		title: "Review",
		description: "Confirm and create",
		icon: Rocket,
		required: true,
	},
];

export function CreateAppPage() {
	const backend = useBackend();
	const auth = useAuth();
	const router = useRouter();
	const templates = useInvoke(
		backend.templateState.getTemplates,
		backend.templateState,
		[],
	);
	const apps = useInvoke(backend.appState.getApps, backend.appState, []);
	const currentProfile = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
	);

	const [currentStepIndex, setCurrentStepIndex] = useState(0);
	const [selectedTemplate, setSelectedTemplate] = useState<[string, string]>([
		"",
		"",
	]);
	const [skipTemplate, setSkipTemplate] = useState(true);
	const [selectedModels, setSelectedModels] = useState<string[]>([]);
	const [skipModels, setSkipModels] = useState(false);
	const [isCreating, setIsCreating] = useState(false);
	const [showConfetti, setShowConfetti] = useState(false);
	const [showTemplateModal, setShowTemplateModal] = useState(false);
	const [showModelModal, setShowModelModal] = useState(false);
	const [isOffline, setIsOffline] = useState(true);
	const [meta, setMeta] = useState<IMetadata>({
		description: "",
		name: "",
		tags: [],
		use_case: "",
		created_at: nowSystemTime(),
		updated_at: nowSystemTime(),
		preview_media: [],
	});

	const currentStep = STEPS[currentStepIndex];
	const isLastStep = currentStepIndex === STEPS.length - 1;
	const isFirstStep = currentStepIndex === 0;

	const canProceedFromCurrentStep = () => {
		switch (currentStep.id) {
			case "details":
				return meta.name.trim() !== "" && meta.description.trim() !== "";
			case "connectivity":
				return true; // Always can proceed from connectivity
			case "template":
				return skipTemplate || selectedTemplate[0] !== "";
			case "models":
				return skipModels || selectedModels.length > 0;
			case "review":
				return true;
			default:
				return false;
		}
	};

	const handleNext = () => {
		if (!canProceedFromCurrentStep()) return;
		if (isLastStep) {
			handleCreateApp();
		} else {
			setCurrentStepIndex((prev) => prev + 1);
		}
	};

	const handleBack = () => {
		if (!isFirstStep) {
			setCurrentStepIndex((prev) => prev - 1);
		}
	};

	const handleCreateApp = async () => {
		setIsCreating(true);
		try {
			const template = skipTemplate
				? undefined
				: await backend.templateState.getTemplate(
						selectedTemplate[0],
						selectedTemplate[1],
					);
			const app = await backend.appState.createApp(
				meta,
				selectedModels,
				!isOffline,
				template,
			);

			try {
				if(currentProfile.data) await backend.userState.updateProfileApp(
					currentProfile.data,
					{
						app_id: app.id,
						favorite: false,
						pinned: false,
					},
					"Upsert"
				)
			}catch(e) {
				console.error("Failed to create app:", e);
			}

			setShowConfetti(true);
			toast(`${isOffline ? "Offline" : "Online"} app created successfully! 🎉`);
			await apps.refetch();
			setTimeout(() => {
				router.push("/library");
			}, 2000);
		} catch (error) {
			console.error("Failed to create app:", error);
			toast("Failed to create app");
		} finally {
			setIsCreating(false);
		}
	};

	const progressPercentage = ((currentStepIndex + 1) / STEPS.length) * 100;

	return (
		<main className="relative min-h-screen bg-linear-to-br from-background via-background to-muted/20 p-6">
			{showConfetti && (
				<div className="absolute z-50 pointer-events-none top-0 left-0 right-0 bottom-0">
					<Crossfire className="" autorun={{ speed: 1 }} />
				</div>
			)}

			<div className="max-w-4xl mx-auto space-y-8">
				<PageHeader />

				<StepIndicator
					steps={STEPS}
					currentStepIndex={currentStepIndex}
					onStepClick={setCurrentStepIndex}
				/>

				<ProgressBar percentage={progressPercentage} />

				<StepContent
					currentStep={currentStep}
					meta={meta}
					setMeta={setMeta}
					isOffline={isOffline}
					setIsOffline={setIsOffline}
					isAuthenticated={auth?.isAuthenticated}
					templates={templates.data || []}
					selectedTemplate={selectedTemplate}
					skipTemplate={skipTemplate}
					onSelectTemplate={(appId, templateId) => {
						setSelectedTemplate([appId, templateId]);
					}}
					onSkipTemplate={setSkipTemplate}
					onShowTemplateModal={() => setShowTemplateModal(true)}
					models={currentProfile.data?.hub_profile.bits || []}
					selectedModels={selectedModels}
					skipModels={skipModels}
					onUpdateModels={setSelectedModels}
					onSkipModels={setSkipModels}
					onShowModelModal={() => setShowModelModal(true)}
				/>

				<StepNavigation
					isFirstStep={isFirstStep}
					isLastStep={isLastStep}
					canProceed={canProceedFromCurrentStep()}
					isCreating={isCreating || showConfetti}
					onBack={handleBack}
					onNext={handleNext}
				/>
			</div>

			<TemplateModal
				open={showTemplateModal}
				onClose={() => setShowTemplateModal(false)}
				templates={
					(templates.data?.filter(
						([_appId, _templateId, metadata]) => metadata,
					) as any) || []
				}
				selectedTemplate={selectedTemplate}
				onSelectTemplate={(appId, templateId) => {
					setSelectedTemplate([appId, templateId]);
					setShowTemplateModal(false);
				}}
			/>

			<ModelModal
				open={showModelModal}
				onClose={() => setShowModelModal(false)}
				models={currentProfile.data?.hub_profile.bits || []}
				selectedModels={selectedModels}
				onUpdateModels={setSelectedModels}
			/>
		</main>
	);
}

function PageHeader() {
	return (
		<div className="text-center space-y-4">
			<div className="flex items-center justify-center gap-3 mb-4">
				<div className="p-3 bg-linear-to-br from-primary to-primary/80 rounded-xl shadow-lg">
					<Rocket className="h-8 w-8 text-primary-foreground" />
				</div>
				<h1 className="text-4xl font-bold">
					Create <span className="highlight">New App</span>
				</h1>
			</div>
			<p className="text-lg text-muted-foreground max-w-2xl mx-auto">
				Build your next AI-powered application with our guided setup wizard.
			</p>
		</div>
	);
}

function StepContent({
	currentStep,
	...props
}: Readonly<{
	currentStep: ICreationStep;
	meta: IMetadata;
	setMeta: (meta: IMetadata | ((prev: IMetadata) => IMetadata)) => void;
	isOffline: boolean;
	setIsOffline: (offline: boolean) => void;
	isAuthenticated?: boolean;
	templates: any[];
	selectedTemplate: [string, string];
	skipTemplate: boolean;
	onSelectTemplate: (appId: string, templateId: string) => void;
	onSkipTemplate: (skip: boolean) => void;
	onShowTemplateModal: () => void;
	models: any[];
	selectedModels: string[];
	skipModels: boolean;
	onUpdateModels: (models: string[]) => void;
	onSkipModels: (skip: boolean) => void;
	onShowModelModal: () => void;
}>) {
	return (
		<Card className="min-h-[400px]">
			<CardHeader>
				<div className="flex items-center gap-3">
					<div className="p-2 bg-primary/10 rounded-lg">
						<currentStep.icon className="h-6 w-6 text-primary" />
					</div>
					<div>
						<CardTitle className="text-2xl">{currentStep.title}</CardTitle>
						<CardDescription className="text-base">
							{currentStep.description}
						</CardDescription>
					</div>
				</div>
			</CardHeader>
			<CardContent>
				{currentStep.id === "details" && (
					<AppDetailsStep meta={props.meta} setMeta={props.setMeta} />
				)}
				{currentStep.id === "connectivity" && (
					<ConnectivityStep
						isOffline={props.isOffline}
						setIsOffline={props.setIsOffline}
						isAuthenticated={props.isAuthenticated}
					/>
				)}
				{currentStep.id === "template" && (
					<TemplateSection
						templates={props.templates}
						selectedTemplate={props.selectedTemplate}
						skipTemplate={props.skipTemplate}
						onSelectTemplate={props.onSelectTemplate}
						onSkipTemplate={props.onSkipTemplate}
						onShowModal={props.onShowTemplateModal}
					/>
				)}
				{currentStep.id === "models" && (
					<ModelSection
						models={props.models}
						selectedModels={props.selectedModels}
						skipModels={props.skipModels}
						onUpdateModels={props.onUpdateModels}
						onSkipModels={props.onSkipModels}
						onShowModal={props.onShowModelModal}
					/>
				)}
				{currentStep.id === "review" && (
					<ReviewStep
						meta={props.meta}
						isOffline={props.isOffline}
						selectedTemplate={props.selectedTemplate}
						skipTemplate={props.skipTemplate}
						selectedModels={props.selectedModels}
						skipModels={props.skipModels}
					/>
				)}
			</CardContent>
		</Card>
	);
}
