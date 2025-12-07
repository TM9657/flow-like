import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	Input,
	Label,
	RadioGroup,
	RadioGroupItem,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Textarea,
} from "@tm9657/flow-like-ui";
import React, { useState, useCallback } from "react";
import {
	LuArrowLeft,
	LuArrowRight,
	LuBuilding2,
	LuCheck,
	LuCreditCard,
	LuFileText,
	LuLoader,
	LuShieldCheck,
	LuSparkles,
	LuTestTube,
	LuUpload,
	LuUser,
	LuUsers,
	LuX,
	LuZap,
} from "react-icons/lu";

interface FormData {
	name: string;
	email: string;
	company: string;
	applicationType: "internal" | "external" | "";
	dataSecurity: string;
	description: string;
	exampleInput: string;
	expectedOutput: string;
	files: UploadedFile[];
	userCount: string;
	userType: "internal" | "external" | "both" | "";
	technicalLevel: "technical" | "non-technical" | "mixed" | "";
	timeline: string;
	additionalNotes: string;
	pricingTier: "standard" | "appstore" | "";
	payDeposit: boolean;
}

interface UploadedFile {
	name: string;
	key: string;
	downloadUrl: string;
	size: number;
}

const dataSecurityOptions = [
	{
		value: "public",
		label: "Public",
		description: "Data can be freely shared with anyone",
	},
	{
		value: "internal",
		label: "Internal",
		description: "For internal use within your organization",
	},
	{
		value: "confidential",
		label: "Confidential",
		description: "Restricted to specific teams or departments",
	},
	{
		value: "secret",
		label: "Secret",
		description: "Highly sensitive, requires strict access control",
	},
];

const steps = [
	{ id: 1, title: "Contact", icon: LuUser },
	{ id: 2, title: "Pricing", icon: LuCreditCard },
	{ id: 3, title: "Application", icon: LuBuilding2 },
	{ id: 4, title: "Security", icon: LuShieldCheck },
	{ id: 5, title: "Details", icon: LuFileText },
	{ id: 6, title: "Examples", icon: LuTestTube },
	{ id: 7, title: "Users", icon: LuUsers },
	{ id: 8, title: "Review", icon: LuSparkles },
];

export function SolutionForm() {
	const [currentStep, setCurrentStep] = useState(1);
	const [isSubmitting, setIsSubmitting] = useState(false);
	const [isUploading, setIsUploading] = useState(false);
	const [submitSuccess, setSubmitSuccess] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const [formData, setFormData] = useState<FormData>({
		name: "",
		email: "",
		company: "",
		applicationType: "",
		dataSecurity: "",
		description: "",
		exampleInput: "",
		expectedOutput: "",
		files: [],
		userCount: "",
		userType: "",
		technicalLevel: "",
		timeline: "",
		additionalNotes: "",
		pricingTier: "",
		payDeposit: false,
	});

	const updateFormData = useCallback(
		<K extends keyof FormData>(field: K, value: FormData[K]) => {
			setFormData((prev) => ({ ...prev, [field]: value }));
		},
		[],
	);

	const handleFileUpload = useCallback(
		async (e: React.ChangeEvent<HTMLInputElement>) => {
			const files = e.target.files;
			if (!files || files.length === 0) return;

			setIsUploading(true);
			setError(null);

			try {
				for (const file of Array.from(files)) {
					const ext = file.name.split(".").pop() || "bin";

					const presignRes = await fetch(
						`https://api.flow-like.com/api/v1/solution/upload?extension=${ext}&content_type=${encodeURIComponent(file.type)}`,
						{ method: "GET" },
					);

					if (!presignRes.ok) {
						throw new Error("Failed to get upload URL");
					}

					const presignData = await presignRes.json();

					const uploadRes = await fetch(presignData.uploadUrl, {
						method: "PUT",
						body: file,
					});

					if (!uploadRes.ok) {
						throw new Error(`Failed to upload file: ${file.name}`);
					}

					setFormData((prev) => ({
						...prev,
						files: [
							...prev.files,
							{
								name: file.name,
								key: presignData.key,
								downloadUrl: presignData.downloadUrl,
								size: file.size,
							},
						],
					}));
				}
			} catch (err) {
				setError(err instanceof Error ? err.message : "Failed to upload file");
			} finally {
				setIsUploading(false);
				e.target.value = "";
			}
		},
		[],
	);

	const removeFile = useCallback((key: string) => {
		setFormData((prev) => ({
			...prev,
			files: prev.files.filter((f) => f.key !== key),
		}));
	}, []);

	const handleSubmit = async () => {
		setIsSubmitting(true);
		setError(null);

		try {
			const res = await fetch("https://api.flow-like.com/api/v1/solution", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(formData),
			});

			if (!res.ok) {
				const data = await res.json().catch(() => ({}));
				throw new Error(data.message || "Failed to submit form");
			}

			const data = await res.json();

			// Redirect to Stripe Checkout
			if (data.checkoutUrl) {
				window.location.href = data.checkoutUrl;
			} else {
				setSubmitSuccess(true);
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to submit form");
		} finally {
			setIsSubmitting(false);
		}
	};

	const canProceed = useCallback(() => {
		switch (currentStep) {
			case 1:
				return formData.name && formData.email && formData.company;
			case 2:
				return formData.pricingTier !== "";
			case 3:
				return formData.applicationType !== "";
			case 4:
				return formData.dataSecurity !== "";
			case 5:
				return formData.description.length >= 50;
			case 6:
				return (
					formData.exampleInput.length >= 20 &&
					formData.expectedOutput.length >= 20
				);
			case 7:
				return (
					formData.userCount && formData.userType && formData.technicalLevel
				);
			case 8:
				return true;
			default:
				return false;
		}
	}, [currentStep, formData]);

	const nextStep = () => {
		if (canProceed() && currentStep < 8) {
			setCurrentStep((prev) => prev + 1);
		}
	};

	const prevStep = () => {
		if (currentStep > 1) {
			setCurrentStep((prev) => prev - 1);
		}
	};

	if (submitSuccess) {
		return (
			<Card className="border-emerald-500/30 bg-emerald-500/5">
				<CardContent className="py-12 text-center">
					<div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-emerald-500/20 text-emerald-500 mb-6">
						<LuCheck className="w-8 h-8" />
					</div>
					<h3 className="text-2xl font-semibold mb-2">Request Submitted!</h3>
					<p className="text-muted-foreground max-w-md mx-auto">
						We've received your project details. Our team will review your
						requirements and get back to with a project
						assessment shortly.
					</p>
				</CardContent>
			</Card>
		);
	}

	return (
		<Card className="border-border/50 overflow-hidden">
			<CardHeader className="pb-4">
				{/* Mobile: Show current step indicator */}
				<div className="flex sm:hidden items-center justify-between mb-4">
					<div className="flex items-center gap-3">
						<div className="flex items-center justify-center w-10 h-10 rounded-full bg-primary text-primary-foreground">
							{React.createElement(steps[currentStep - 1].icon, {
								className: "w-5 h-5",
							})}
						</div>
						<div>
							<div className="font-medium">{steps[currentStep - 1].title}</div>
							<div className="text-xs text-muted-foreground">
								Step {currentStep} of 8
							</div>
						</div>
					</div>
					<div className="flex gap-1">
						{steps.map((step) => (
							<div
								key={step.id}
								className={`w-2 h-2 rounded-full transition-colors ${
									step.id === currentStep
										? "bg-primary"
										: step.id < currentStep
											? "bg-emerald-500"
											: "bg-muted"
								}`}
							/>
						))}
					</div>
				</div>

				{/* Desktop: Full progress bar */}
				<div className="hidden sm:flex items-center justify-between mb-4 overflow-x-auto">
					{steps.map((step, index) => (
						<div key={step.id} className="flex items-center shrink-0">
							<button
								type="button"
								onClick={() => step.id < currentStep && setCurrentStep(step.id)}
								disabled={step.id > currentStep}
								className={`flex items-center justify-center w-8 h-8 md:w-10 md:h-10 rounded-full transition-all ${
									step.id === currentStep
										? "bg-primary text-primary-foreground"
										: step.id < currentStep
											? "bg-emerald-500 text-white cursor-pointer hover:bg-emerald-600"
											: "bg-muted text-muted-foreground"
								}`}
							>
								{step.id < currentStep ? (
									<LuCheck className="w-4 h-4 md:w-5 md:h-5" />
								) : (
									<step.icon className="w-4 h-4 md:w-5 md:h-5" />
								)}
							</button>
							{index < steps.length - 1 && (
								<div
									className={`w-4 sm:w-6 md:w-10 lg:w-14 h-0.5 mx-0.5 sm:mx-1 ${
										step.id < currentStep ? "bg-emerald-500" : "bg-muted"
									}`}
								/>
							)}
						</div>
					))}
				</div>
				<div className="hidden sm:block">
					<CardTitle>{steps[currentStep - 1].title}</CardTitle>
					<CardDescription>Step {currentStep} of 8</CardDescription>
				</div>
			</CardHeader>

			<CardContent className="space-y-6">
				{error && (
					<div className="p-4 rounded-lg bg-destructive/10 border border-destructive/30 text-destructive text-sm">
						{error}
					</div>
				)}

				{currentStep === 1 && (
					<div className="space-y-4">
						<div className="grid sm:grid-cols-2 gap-4">
							<div className="space-y-2">
								<Label htmlFor="name">Full Name *</Label>
								<Input
									id="name"
									placeholder="John Doe"
									value={formData.name}
									onChange={(e) => updateFormData("name", e.target.value)}
								/>
							</div>
							<div className="space-y-2">
								<Label htmlFor="email">Work Email *</Label>
								<Input
									id="email"
									type="email"
									placeholder="john@company.com"
									value={formData.email}
									onChange={(e) => updateFormData("email", e.target.value)}
								/>
							</div>
						</div>
						<div className="space-y-2">
							<Label htmlFor="company">Company Name *</Label>
							<Input
								id="company"
								placeholder="Acme Inc."
								value={formData.company}
								onChange={(e) => updateFormData("company", e.target.value)}
							/>
						</div>
					</div>
				)}

				{currentStep === 2 && (
					<div className="space-y-4">
						<Label>Select Your Package *</Label>
						<p className="text-sm text-muted-foreground">
							Choose the pricing tier that best fits your needs. Pay after
							delivery — or pay a deposit for priority.
						</p>
						<RadioGroup
							value={formData.pricingTier}
							onValueChange={(value) =>
								updateFormData("pricingTier", value as "standard" | "appstore")
							}
							className="grid gap-4"
						>
							<Label
								htmlFor="standard"
								className={`flex items-start gap-4 p-6 rounded-xl border cursor-pointer transition-all ${
									formData.pricingTier === "standard"
										? "border-primary bg-primary/5 ring-2 ring-primary/20"
										: "border-border hover:border-primary/50"
								}`}
							>
								<RadioGroupItem
									value="standard"
									id="standard"
									className="mt-1"
								/>
								<div className="flex-1">
									<div className="flex items-center justify-between">
										<div className="font-semibold text-lg">Standard</div>
										<div className="text-right">
											<div className="text-2xl font-bold text-primary">
												€2,400
											</div>
											<div className="text-xs text-muted-foreground">
												Pay after delivery
											</div>
										</div>
									</div>
									<div className="text-sm text-muted-foreground mt-1">
										Full source code ownership. Deploy anywhere you want.
									</div>
									<div className="flex flex-wrap gap-2 mt-3">
										<span className="text-xs px-2 py-1 rounded-full bg-primary/10 text-primary font-medium">
											🔋 Infrastructure included
										</span>
										<span className="text-xs px-2 py-1 rounded-full bg-primary/10 text-primary font-medium">
											∞ Unlimited usage
										</span>
										<span className="text-xs px-2 py-1 rounded-full bg-muted">
											Source code
										</span>
										<span className="text-xs px-2 py-1 rounded-full bg-muted">
											30 days support
										</span>
									</div>
								</div>
							</Label>
							<Label
								htmlFor="appstore"
								className={`relative flex items-start gap-4 p-6 rounded-xl border cursor-pointer transition-all ${
									formData.pricingTier === "appstore"
										? "border-emerald-500 bg-emerald-500/5 ring-2 ring-emerald-500/20"
										: "border-border hover:border-emerald-500/50"
								}`}
							>
								<div className="absolute -top-2 right-4 px-2 py-0.5 text-xs font-medium bg-emerald-500 text-white rounded-full">
									Save €401
								</div>
								<RadioGroupItem
									value="appstore"
									id="appstore"
									className="mt-1"
								/>
								<div className="flex-1">
									<div className="flex items-center justify-between">
										<div className="font-semibold text-lg">App Store</div>
										<div className="text-right">
											<div className="flex items-baseline gap-2 justify-end">
												<span className="text-sm text-muted-foreground line-through">
													€2,400
												</span>
												<span className="text-2xl font-bold text-emerald-500">
													€1,999
												</span>
											</div>
											<div className="text-xs text-muted-foreground">
												Pay after delivery
											</div>
										</div>
									</div>
									<div className="text-sm text-muted-foreground mt-1">
										Your app gets published in the Flow-Like App Store for
										others to use.
									</div>
									<div className="flex flex-wrap gap-2 mt-3">
										<span className="text-xs px-2 py-1 rounded-full bg-emerald-500/10 text-emerald-600 font-medium">
											🔋 Infrastructure included
										</span>
										<span className="text-xs px-2 py-1 rounded-full bg-emerald-500/10 text-emerald-600 font-medium">
											∞ Unlimited usage
										</span>
										<span className="text-xs px-2 py-1 rounded-full bg-emerald-500/10 text-emerald-600">
											Published in App Store
										</span>
										<span className="text-xs px-2 py-1 rounded-full bg-muted">
											30 days support
										</span>
									</div>
								</div>
							</Label>
						</RadioGroup>

						{formData.pricingTier === "appstore" && (
							<div className="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/20">
								<div className="text-sm text-emerald-700 dark:text-emerald-400">
									<strong>App Store Tier:</strong> Your automation will be
									published in our App Store, allowing other Flow-Like users to
									discover and use it. You'll be credited as the creator.
								</div>
							</div>
						)}
					</div>
				)}

				{currentStep === 3 && (
					<div className="space-y-4">
						<Label>Application Type *</Label>
						<p className="text-sm text-muted-foreground">
							Will this application be used within your organization or by
							external customers?
						</p>
						<RadioGroup
							value={formData.applicationType}
							onValueChange={(value) =>
								updateFormData(
									"applicationType",
									value as "internal" | "external",
								)
							}
							className="grid sm:grid-cols-2 gap-4"
						>
							<Label
								htmlFor="internal"
								className={`flex items-start gap-4 p-4 rounded-xl border cursor-pointer transition-all ${
									formData.applicationType === "internal"
										? "border-primary bg-primary/5"
										: "border-border hover:border-primary/50"
								}`}
							>
								<RadioGroupItem value="internal" id="internal" />
								<div>
									<div className="font-medium">Internal Application</div>
									<div className="text-sm text-muted-foreground">
										Used by your employees or team members
									</div>
								</div>
							</Label>
							<Label
								htmlFor="external"
								className={`flex items-start gap-4 p-4 rounded-xl border cursor-pointer transition-all ${
									formData.applicationType === "external"
										? "border-primary bg-primary/5"
										: "border-border hover:border-primary/50"
								}`}
							>
								<RadioGroupItem value="external" id="external" />
								<div>
									<div className="font-medium">External Application</div>
									<div className="text-sm text-muted-foreground">
										Used by your customers or partners
									</div>
								</div>
							</Label>
						</RadioGroup>
					</div>
				)}

				{currentStep === 4 && (
					<div className="space-y-4">
						<Label>Data Classification Level *</Label>
						<p className="text-sm text-muted-foreground">
							Select the sensitivity level of the data your application will
							handle.
						</p>
						<Select
							value={formData.dataSecurity}
							onValueChange={(value) => updateFormData("dataSecurity", value)}
						>
							<SelectTrigger className="w-full">
								<SelectValue placeholder="Select data classification..." />
							</SelectTrigger>
							<SelectContent>
								{dataSecurityOptions.map((option) => (
									<SelectItem key={option.value} value={option.value}>
										<div className="flex flex-col">
											<span className="font-medium">{option.label}</span>
											<span className="text-xs text-muted-foreground">
												{option.description}
											</span>
										</div>
									</SelectItem>
								))}
							</SelectContent>
						</Select>

						{formData.dataSecurity && (
							<div className="p-4 rounded-xl bg-muted/30 border border-border/50">
								<div className="font-medium capitalize mb-1">
									{
										dataSecurityOptions.find(
											(o) => o.value === formData.dataSecurity,
										)?.label
									}
								</div>
								<div className="text-sm text-muted-foreground">
									{
										dataSecurityOptions.find(
											(o) => o.value === formData.dataSecurity,
										)?.description
									}
								</div>
							</div>
						)}
					</div>
				)}

				{currentStep === 5 && (
					<div className="space-y-4">
						<div className="space-y-2">
							<Label htmlFor="description">Use Case Description *</Label>
							<p className="text-sm text-muted-foreground">
								Describe what you want to automate or build. Be as detailed as
								possible. (min. 50 characters)
							</p>
							<Textarea
								id="description"
								placeholder="Describe your automation or GenAI use case in detail. What problem are you solving? What are the inputs and expected outputs? What systems need to be integrated?"
								className="min-h-[200px]"
								value={formData.description}
								onChange={(e) => updateFormData("description", e.target.value)}
							/>
							<p className="text-xs text-muted-foreground text-right">
								{formData.description.length} / 50+ characters
							</p>
						</div>

						<div className="space-y-2">
							<Label>Example Data (Optional)</Label>
							<p className="text-sm text-muted-foreground">
								Upload sample files, documents, or data that will help us
								understand your use case better.
							</p>
							<div className="flex flex-col gap-4">
								<div className="relative">
									<input
										type="file"
										multiple
										onChange={handleFileUpload}
										disabled={isUploading}
										className="absolute inset-0 w-full h-full opacity-0 cursor-pointer disabled:cursor-not-allowed"
									/>
									<div className="flex items-center justify-center gap-2 p-8 rounded-xl border-2 border-dashed border-border hover:border-primary/50 transition-colors">
										{isUploading ? (
											<LuLoader className="w-5 h-5 animate-spin text-muted-foreground" />
										) : (
											<LuUpload className="w-5 h-5 text-muted-foreground" />
										)}
										<span className="text-sm text-muted-foreground">
											{isUploading
												? "Uploading..."
												: "Drop files here or click to upload"}
										</span>
									</div>
								</div>

								{formData.files.length > 0 && (
									<div className="space-y-2">
										{formData.files.map((file) => (
											<div
												key={file.key}
												className="flex items-center justify-between p-3 rounded-lg bg-muted/50"
											>
												<div className="flex items-center gap-2 min-w-0">
													<LuFileText className="w-4 h-4 text-muted-foreground shrink-0" />
													<span className="text-sm truncate">{file.name}</span>
													<span className="text-xs text-muted-foreground shrink-0">
														({Math.round(file.size / 1024)} KB)
													</span>
												</div>
												<Button
													variant="ghost"
													size="icon"
													className="shrink-0 h-8 w-8"
													onClick={() => removeFile(file.key)}
												>
													<LuX className="w-4 h-4" />
												</Button>
											</div>
										))}
									</div>
								)}
							</div>
						</div>
					</div>
				)}

				{currentStep === 6 && (
					<div className="space-y-6">
						<div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20">
							<div className="flex items-start gap-3">
								<LuTestTube className="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
								<div className="text-sm text-amber-700 dark:text-amber-400">
									<strong>Why examples matter:</strong> Providing concrete
									input/output examples helps us verify the automation works
									correctly and serves as test cases during development.
								</div>
							</div>
						</div>

						<div className="space-y-2">
							<Label htmlFor="exampleInput">Example Input *</Label>
							<p className="text-sm text-muted-foreground">
								Provide a realistic example of what the automation will receive.
								For Q&A bots, this could be questions. For data processing,
								provide sample data. (min. 20 characters)
							</p>
							<Textarea
								id="exampleInput"
								placeholder="Example: 'What are the opening hours for the Munich office?' or paste sample CSV data, email content, etc."
								className="min-h-[150px] font-mono text-sm"
								value={formData.exampleInput}
								onChange={(e) => updateFormData("exampleInput", e.target.value)}
							/>
							<p className="text-xs text-muted-foreground text-right">
								{formData.exampleInput.length} / 20+ characters
							</p>
						</div>

						<div className="space-y-2">
							<Label htmlFor="expectedOutput">Expected Output *</Label>
							<p className="text-sm text-muted-foreground">
								What should the automation produce for the input above? Be
								specific about format, tone, and content. (min. 20 characters)
							</p>
							<Textarea
								id="expectedOutput"
								placeholder="Example: 'The Munich office is open Monday to Friday, 9 AM to 6 PM CET.' or describe the expected data format, actions taken, etc."
								className="min-h-[150px] font-mono text-sm"
								value={formData.expectedOutput}
								onChange={(e) =>
									updateFormData("expectedOutput", e.target.value)
								}
							/>
							<p className="text-xs text-muted-foreground text-right">
								{formData.expectedOutput.length} / 20+ characters
							</p>
						</div>

						<div className="p-4 rounded-xl bg-muted/30 border border-border/50">
							<div className="text-sm text-muted-foreground">
								<strong>Tip:</strong> The more examples you provide, the better
								we can verify your automation works. You can add additional
								examples in the "Additional Notes" section or upload files with
								test data.
							</div>
						</div>
					</div>
				)}

				{currentStep === 7 && (
					<div className="space-y-6">
						<div className="space-y-2">
							<Label htmlFor="userCount">Expected Number of Users *</Label>
							<Input
								id="userCount"
								placeholder="e.g., 10-50, 100+, etc."
								value={formData.userCount}
								onChange={(e) => updateFormData("userCount", e.target.value)}
							/>
						</div>

						<div className="space-y-4">
							<Label>User Type *</Label>
							<RadioGroup
								value={formData.userType}
								onValueChange={(value) =>
									updateFormData(
										"userType",
										value as "internal" | "external" | "both",
									)
								}
								className="grid sm:grid-cols-3 gap-4"
							>
								{[
									{ value: "internal", label: "Internal Only" },
									{ value: "external", label: "External Only" },
									{ value: "both", label: "Both" },
								].map((option) => (
									<Label
										key={option.value}
										htmlFor={`userType-${option.value}`}
										className={`flex items-center gap-3 p-4 rounded-xl border cursor-pointer transition-all ${
											formData.userType === option.value
												? "border-primary bg-primary/5"
												: "border-border hover:border-primary/50"
										}`}
									>
										<RadioGroupItem
											value={option.value}
											id={`userType-${option.value}`}
										/>
										<span>{option.label}</span>
									</Label>
								))}
							</RadioGroup>
						</div>

						<div className="space-y-4">
							<Label>Technical Level of Users *</Label>
							<RadioGroup
								value={formData.technicalLevel}
								onValueChange={(value) =>
									updateFormData(
										"technicalLevel",
										value as "technical" | "non-technical" | "mixed",
									)
								}
								className="grid sm:grid-cols-3 gap-4"
							>
								{[
									{ value: "technical", label: "Technical" },
									{ value: "non-technical", label: "Non-Technical" },
									{ value: "mixed", label: "Mixed" },
								].map((option) => (
									<Label
										key={option.value}
										htmlFor={`techLevel-${option.value}`}
										className={`flex items-center gap-3 p-4 rounded-xl border cursor-pointer transition-all ${
											formData.technicalLevel === option.value
												? "border-primary bg-primary/5"
												: "border-border hover:border-primary/50"
										}`}
									>
										<RadioGroupItem
											value={option.value}
											id={`techLevel-${option.value}`}
										/>
										<span>{option.label}</span>
									</Label>
								))}
							</RadioGroup>
						</div>

						<div className="space-y-2">
							<Label htmlFor="timeline">Preferred Timeline (Start)</Label>
							<Input
								id="timeline"
								placeholder="e.g., ASAP, within 2 weeks, flexible"
								value={formData.timeline}
								onChange={(e) => updateFormData("timeline", e.target.value)}
							/>
						</div>
					</div>
				)}

				{currentStep === 8 && (
					<div className="space-y-6">
						{/* Pricing Summary */}
						<div
							className={`p-4 rounded-xl border-2 ${
								formData.pricingTier === "appstore"
									? "border-emerald-500 bg-emerald-500/5"
									: "border-primary bg-primary/5"
							}`}
						>
							<div className="flex items-center justify-between">
								<div>
									<div className="text-sm text-muted-foreground">
										Selected Package
									</div>
									<div className="font-semibold text-lg">
										{formData.pricingTier === "appstore"
											? "App Store"
											: "Standard"}
									</div>
								</div>
								<div className="text-right">
									<div
										className={`text-2xl font-bold ${
											formData.pricingTier === "appstore"
												? "text-emerald-500"
												: "text-primary"
										}`}
									>
										{formData.pricingTier === "appstore" ? "€1,999" : "€2,400"}
									</div>
									<div className="text-xs text-muted-foreground">
										Full amount invoiced after delivery
									</div>
								</div>
							</div>
						</div>

						{/* Priority Deposit Option */}
						<div
							className={`p-4 rounded-xl border-2 cursor-pointer transition-all ${
								formData.payDeposit
									? "border-amber-500 bg-amber-500/10"
									: "border-border hover:border-amber-500/50"
							}`}
							onClick={() => updateFormData("payDeposit", !formData.payDeposit)}
						>
							<div className="flex items-start gap-4">
								<div
									className={`w-5 h-5 rounded border-2 flex items-center justify-center shrink-0 mt-0.5 transition-colors ${
										formData.payDeposit
											? "border-amber-500 bg-amber-500"
											: "border-muted-foreground"
									}`}
								>
									{formData.payDeposit && (
										<LuCheck className="w-3 h-3 text-white" />
									)}
								</div>
								<div className="flex-1">
									<div className="flex items-center gap-2 mb-1">
										<LuZap className="w-4 h-4 text-amber-500" />
										<span className="font-semibold">
											Priority Queue — €500 deposit
										</span>
									</div>
									<p className="text-sm text-muted-foreground">
										Pay a €500 deposit now to jump the queue. Guaranteed to
										start within 2 weeks. Deposit is deducted from the final
										invoice.
									</p>
								</div>
							</div>
						</div>

						<div className="grid gap-4 sm:grid-cols-2">
							<div className="p-4 rounded-xl bg-muted/50">
								<div className="text-sm text-muted-foreground">Contact</div>
								<div className="font-medium">{formData.name}</div>
								<div className="text-sm text-muted-foreground">
									{formData.email}
								</div>
								<div className="text-sm text-muted-foreground">
									{formData.company}
								</div>
							</div>
							<div className="p-4 rounded-xl bg-muted/50">
								<div className="text-sm text-muted-foreground">
									Application Type
								</div>
								<div className="font-medium capitalize">
									{formData.applicationType}
								</div>
							</div>
							<div className="p-4 rounded-xl bg-muted/50">
								<div className="text-sm text-muted-foreground">
									Data Classification
								</div>
								<div className="font-medium capitalize">
									{dataSecurityOptions.find(
										(o) => o.value === formData.dataSecurity,
									)?.label || "Not selected"}
								</div>
							</div>
							<div className="p-4 rounded-xl bg-muted/50">
								<div className="text-sm text-muted-foreground">
									Target Users
								</div>
								<div className="font-medium">
									{formData.userCount} users ({formData.userType},{" "}
									{formData.technicalLevel})
								</div>
							</div>
						</div>

						<div className="p-4 rounded-xl bg-muted/50">
							<div className="text-sm text-muted-foreground mb-2">
								Use Case Description
							</div>
							<div className="text-sm whitespace-pre-wrap">
								{formData.description}
							</div>
						</div>

						<div className="grid gap-4 sm:grid-cols-2">
							<div className="p-4 rounded-xl bg-muted/50">
								<div className="text-sm text-muted-foreground mb-2">
									Example Input
								</div>
								<div className="text-sm font-mono whitespace-pre-wrap bg-background/50 p-2 rounded">
									{formData.exampleInput}
								</div>
							</div>
							<div className="p-4 rounded-xl bg-muted/50">
								<div className="text-sm text-muted-foreground mb-2">
									Expected Output
								</div>
								<div className="text-sm font-mono whitespace-pre-wrap bg-background/50 p-2 rounded">
									{formData.expectedOutput}
								</div>
							</div>
						</div>

						{formData.files.length > 0 && (
							<div className="p-4 rounded-xl bg-muted/50">
								<div className="text-sm text-muted-foreground mb-2">
									Attached Files
								</div>
								<div className="flex flex-wrap gap-2">
									{formData.files.map((file) => (
										<span
											key={file.key}
											className="inline-flex items-center gap-1 px-2 py-1 rounded bg-background text-xs"
										>
											<LuFileText className="w-3 h-3" />
											{file.name}
										</span>
									))}
								</div>
							</div>
						)}

						<div className="space-y-2">
							<Label htmlFor="additionalNotes">
								Additional Notes (Optional)
							</Label>
							<Textarea
								id="additionalNotes"
								placeholder="Any other information you'd like to share..."
								value={formData.additionalNotes}
								onChange={(e) =>
									updateFormData("additionalNotes", e.target.value)
								}
							/>
						</div>
					</div>
				)}

				<div className="flex items-center justify-between pt-4 border-t border-border/50">
					<Button
						variant="outline"
						onClick={prevStep}
						disabled={currentStep === 1}
						className="gap-2"
					>
						<LuArrowLeft className="w-4 h-4" />
						Back
					</Button>

					{currentStep < 8 ? (
						<Button
							onClick={nextStep}
							disabled={!canProceed()}
							className="gap-2"
						>
							Continue
							<LuArrowRight className="w-4 h-4" />
						</Button>
					) : (
						<Button
							onClick={handleSubmit}
							disabled={isSubmitting}
							className={`gap-2 ${
								formData.payDeposit
									? "bg-amber-600 hover:bg-amber-700"
									: formData.pricingTier === "appstore"
										? "bg-emerald-600 hover:bg-emerald-700"
										: "bg-primary hover:bg-primary/90"
							}`}
						>
							{isSubmitting ? (
								<>
									<LuLoader className="w-4 h-4 animate-spin" />
									Processing...
								</>
							) : formData.payDeposit ? (
								<>
									Pay €500 Deposit
									<LuZap className="w-4 h-4" />
								</>
							) : (
								<>
									Submit Request
									<LuArrowRight className="w-4 h-4" />
								</>
							)}
						</Button>
					)}
				</div>
			</CardContent>
		</Card>
	);
}
