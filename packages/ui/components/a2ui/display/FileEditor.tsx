"use client";

import type { ComponentProps } from "../ComponentRegistry";
import type { FileEditorComponent } from "../types";
import { A2UIFilePreview } from "./FilePreview";

export function A2UIFileEditor(props: ComponentProps<FileEditorComponent>) {
	return <A2UIFilePreview {...props} />;
}
