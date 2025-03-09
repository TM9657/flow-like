import CargoLicense from "../src-tauri/assets/licenses.json";
import NodeLicense from "../src-tauri/assets/licenses-node.json";
import fs from "fs";
let license_bundled: {
	name: string;
	license: string | null;
	author: string | null;
	link: string | null;
}[] = [];

for (const license of CargoLicense) {
	license_bundled.push({
		name: license.name,
		license: license.license,
		author: license.authors,
		link: license.repository,
	});
}

for (const license of NodeLicense) {
	license_bundled.push({
		name: license.name,
		license: license.licenseType,
		author: license.author,
		link: license.link,
	});
}

license_bundled = license_bundled.sort((a, b) => a.name.localeCompare(b.name));

const markdown = `
Generated: ${new Date().toISOString()}
# Licenses
## On the shoulders of giants
Without these packages, Flow-Like would not be possible. Thank you to all the developers who have contributed to these projects.
Let us know if we missed any licenses or if you have any questions.

${license_bundled
	.map((license) => {
		return `
### ${license.name}
${license.author ? `**Author**: ${license.author}` : ""}
${license.license ? `**License**: ${license.license}` : ""}
${license.link ? `[Repository](${license.link})` : ""}
`;
	})
	.join("\n")}
`;

fs.writeFileSync("./src-tauri/assets/licenses.md", markdown);
