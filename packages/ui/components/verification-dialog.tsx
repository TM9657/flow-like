"use client";
import { Button } from "./ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "./ui/dialog";

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
			<DialogTrigger asChild>{children}</DialogTrigger>
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
