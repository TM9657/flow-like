import { useTranslation } from "@flow-like/locales";
import { CirclePlusIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import type { IPinAction } from "../flow-node";

export function FlowPinAction({
	action,
	index,
	input,
}: Readonly<{ action: IPinAction; index: number; input: boolean }>) {
	const { t } = useTranslation("flow");
	const [saving, setSaving] = useState(false);
	return (
		<button
			type="button"
			aria-label={t("addPin", "Add pin")}
			disabled={saving}
			className={`nodrag nopan absolute flex flex-row items-center gap-1 top-0 translate-y-[-50%] translate-x-[-6px] ${input ? "-left-3" : "-right-6"}`}
			style={{
				top: (index - 1) * 15,
				marginTop: "1.75rem",
				height: 15,
			}}
			onClick={async (event) => {
				event.stopPropagation();
				if (saving) return;
				setSaving(true);
				try {
					await action.onAction(action.pin);
				} catch (cause) {
					toast.error(
						cause instanceof Error
							? cause.message
							: t("addPinFailed", "Could not add the pin."),
					);
				} finally {
					setSaving(false);
				}
			}}
		>
			<CirclePlusIcon className="w-3 h-3 bg-background rounded-full" />
		</button>
	);
}
