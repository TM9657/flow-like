import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { craftSaveLabel, craftViewer } from "./craft-viewers";
import { createStorageCraftFile } from "./storage-craft";

describe("storage editor routing and save targets", () => {
	it.each([
		["report.PDF", "pdfcraft", "pdf"],
		["slides.pptx", "deckcraft", "pptx"],
		["slides.potx", "deckcraft", "pptx"],
		["table.xls", "gridcraft", "xlsx"],
		["table.ods", "gridcraft", "xlsx"],
		["table.xlsm", "gridcraft", "xlsm"],
		["shape.svg", "vectorcraft", "svg"],
		["drawing.dxf", "cadcraft", "dxf"],
		["part.solidcraft", "solidcraft", "solidcraft"],
		["part.STEP", "solidcraft", "solidcraft"],
		["part.stp", "solidcraft", "solidcraft"],
		["part.iges", "solidcraft", "solidcraft"],
		["part.igs", "solidcraft", "solidcraft"],
		["part.stl", "solidcraft", "solidcraft"],
		["part.obj", "solidcraft", "solidcraft"],
		["part.3mf", "solidcraft", "solidcraft"],
		["part.glb", "solidcraft", "solidcraft"],
		["part.gltf", "solidcraft", "solidcraft"],
		["sound.mp3", "soundcraft", "wav"],
		["sound.flac", "soundcraft", "flac"],
		["movie.mov", "filmcraft", "mp4"],
		["photo.png", "photocraft", "png"],
		["animation.gif", "photocraft", "pcraft"],
	])("routes %s to %s and exports %s", (name, id, format) => {
		expect(craftViewer(name)).toMatchObject({ id, outputExtension: format });
	});
	it.each([
		"legacy.ppt",
		"drawing.dwg",
		"part.FCStd",
		"part.3dm",
		"slides.odp",
		"old.doc",
		"file.aep",
		"file.indd",
		"source.ts",
		"config.json",
		"notes.txt",
		"photo.heic",
	])("keeps unsupported or ambiguous %s out of the editor", (name) =>
		expect(craftViewer(name)).toBeUndefined(),
	);
	it("preserves storage address escaping when saving the original", () => {
		const location = "folder/a%2Fb%2520c.xlsx";
		const file = createStorageCraftFile(
			new File(["bytes"], "forged.xlsx"),
			location,
			requireViewer(location),
			"id",
		);
		expect(file.name).toBe("a%2Fb%2520c.xlsx");
		expect(craftSaveLabel(requireViewer(location))).toBe("Save");
	});
	it("creates a distinct valid filename for formats imported through conversion", () => {
		const location = "folder/a%2Fb.xls";
		const file = createStorageCraftFile(
			new File(["bytes"], "forged.exe"),
			location,
			requireViewer(location),
			"unique",
		);
		expect(file.name).toBe("a%2Fb-converted-unique.xlsx");
		expect(craftSaveLabel(requireViewer(location))).toBe("Save as .xlsx");
	});
	it("saves imported CAD as a separate project while preserving native project names", () => {
		const imported = "parts/bracket%2Fleft.step";
		const converted = createStorageCraftFile(
			new File(["project"], "forged.step"),
			imported,
			requireViewer(imported),
			"unique",
		);
		expect(converted.name).toBe("bracket%2Fleft-converted-unique.solidcraft");
		expect(craftSaveLabel(requireViewer(imported))).toBe("Save as .solidcraft");
		const native = `parts/${converted.name}`;
		const saved = createStorageCraftFile(
			new File(["updated project"], "forged.step"),
			native,
			requireViewer(native),
			"another-id",
		);
		expect(saved.name).toBe(converted.name);
		expect(craftSaveLabel(requireViewer(native))).toBe("Save");
	});
});

function requireViewer(name: string) {
	const viewer = craftViewer(name);
	if (!viewer) throw new Error(`Missing viewer for ${name}`);
	return viewer;
}

it("keeps runtime limits and advertised imports aligned with storage routing", () => {
	for (const filename of [
		"file.pdf",
		"file.pptx",
		"file.xlsx",
		"file.svg",
		"file.png",
		"file.wav",
		"file.mp4",
		"file.dxf",
		"file.solidcraft",
		"file.step",
		"file.stp",
		"file.iges",
		"file.igs",
		"file.stl",
		"file.obj",
		"file.3mf",
		"file.glb",
		"file.gltf",
		"file.idml",
	]) {
		const viewer = requireViewer(filename);
		const pin = JSON.parse(
			readFileSync(
				resolve(
					import.meta.dirname,
					"../../../craft-viewers",
					viewer.id,
					"upstream.json",
				),
				"utf8",
			),
		);
		expect(pin.extensions).toContain(viewer.extension);
		expect(pin.maxBytes).toBe(viewer.maxBytes);
		expect(pin.label).toBe(viewer.label);
	}
});
