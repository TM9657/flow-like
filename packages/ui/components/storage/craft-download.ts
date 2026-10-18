export function downloadCraftFile(file: File) {
	const url = URL.createObjectURL(file);
	const link = document.createElement("a");
	link.href = url;
	link.download = file.name;
	document.body.append(link);
	link.click();
	link.remove();
	// Keep the URL alive until the browser has accepted the download.
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}
