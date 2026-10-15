/** Open an image without parsing attachment data as document markup. */
export function openImagePreview(url: string): void {
	const preview = window.open("about:blank", "_blank");
	if (!preview) return;
	preview.opener = null;
	const image = preview.document.createElement("img");
	image.src = url;
	image.referrerPolicy = "no-referrer";
	image.style.maxWidth = "100%";
	image.style.height = "auto";
	preview.document.body.replaceChildren(image);
}
