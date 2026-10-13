import { describe, expect, test } from "bun:test";
import { createI18n } from "@flow-like/locales";
import { renderToStaticMarkup } from "react-dom/server";
import { DataSensitivityBadges } from "./data-classification";

createI18n({ language: "en" });

describe("DataSensitivityBadges", () => {
	test("renders nothing for an unlabelled resource", () => {
		for (const value of [undefined, {}, { pii: false }]) {
			expect(
				renderToStaticMarkup(<DataSensitivityBadges value={value} />),
			).toBe("");
		}
	});

	test("shows the level in its tone and flags personal data", () => {
		const markup = renderToStaticMarkup(
			<DataSensitivityBadges
				value={{ classification: "restricted", pii: true }}
			/>,
		);
		expect(markup).toContain("Restricted");
		expect(markup).toContain("text-destructive");
		expect(markup).toContain("PII");
		expect(markup).toContain('title="Contains personal data"');
	});

	test("uses the info and warning tones for the middle levels", () => {
		const tone = (classification: "internal" | "confidential") =>
			renderToStaticMarkup(
				<DataSensitivityBadges value={{ classification }} />,
			);
		expect(tone("internal")).toContain("bg-info-bg");
		expect(tone("confidential")).toContain("bg-warning-bg");
		expect(tone("internal")).not.toContain("PII");
	});
});
