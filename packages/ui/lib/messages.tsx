import { toast } from "sonner";

export function toastSuccess(message: string, icon: React.ReactNode) {
	toast.success(
		<div className="flex flex-row items-center gap-2">
			{icon}
			{message}
		</div>,
	);
}

export function toastWarning(message: string, icon: React.ReactNode) {
	toast.warning(
		<div className="flex flex-row items-center gap-2">
			{icon}
			{message}
		</div>,
	);
}

export function toastError(message: string, icon: React.ReactNode) {
	toast.error(
		<div className="flex flex-row items-center gap-2">
			{icon}
			{message}
		</div>,
	);
}
