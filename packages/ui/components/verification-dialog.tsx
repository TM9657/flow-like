"use client";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "./ui/dialog";
import { Button } from "./ui/button";

export function VerificationDialog({
	children,
	dialog,
	onConfirm,
	onCancel = () => {},
}: Readonly<{
	children: React.ReactNode;
	dialog: string;
	onConfirm: () => void;
	onCancel?: () => void;
}>) {
	return (
		<Dialog
			onOpenChange={(open) => {
				if (!open) onCancel?.();
			}}
		>
			<DialogTrigger>{children}</DialogTrigger>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Are you absolutely sure?</DialogTitle>
					<DialogDescription>{dialog}</DialogDescription>
					<DialogFooter>
						<Button
							onClick={() => {
								onConfirm();
							}}
						>
							Confirm
						</Button>
					</DialogFooter>
				</DialogHeader>
			</DialogContent>
		</Dialog>
	);
}
