import { execSync } from "child_process";
import {
	mkdirSync,
	readFileSync,
	readdirSync,
	statSync,
	writeFileSync,
} from "fs";
import { join, parse, relative } from "path";

// 1st: Make "../" the root directory

// 2nd: Call "cargo run --bin flow-like-schema"
console.log("Running cargo command: cargo run --bin schema-gen");
execSync("cargo run --bin schema-gen", { stdio: "inherit" });

// 3rd: Iterate over the "schema/**" folder recursively and generate the schema for each file
const schemaDir = "packages/schema";
const outputDir = "packages/ui/lib/schema";

// Utility function to recursively get all files in a directory
function getAllFiles(dir: string): string[] {
	let files: string[] = [];

	readdirSync(dir).forEach((file) => {
		const fullPath = join(dir, file);
		if (statSync(fullPath).isDirectory()) {
			files = files.concat(getAllFiles(fullPath));
		} else {
			files.push(fullPath);
		}
	});

	return files;
}

// Ensure the output directory exists
mkdirSync(outputDir, { recursive: true });

// Process each JSON file in the schema directory
const schemaFiles = getAllFiles(schemaDir).filter((file) =>
	file.endsWith(".json"),
);

schemaFiles.forEach((schemaFile) => {
	const relativePath = relative(schemaDir, schemaFile); // e.g., "llm/history.json"
	const parsedPath = parse(relativePath);
	const outputFilePath = join(
		outputDir,
		parsedPath.dir,
		`${parsedPath.name}.ts`,
	); // e.g., "lib/schema/llm/history.ts"

	// Ensure output subdirectories exist
	mkdirSync(parse(outputFilePath).dir, { recursive: true });

	// Command to generate TypeScript file using quicktype
	const quicktypeCommand = `bunx quicktype --just-types -o ${outputFilePath} -s schema ${schemaFile}`;

	console.log(`Processing: ${schemaFile} -> ${outputFilePath}`);
	execSync(quicktypeCommand);

	const prefixes = new Set<string>();

	let content = readFileSync(outputFilePath, "utf-8");

	content.matchAll(/interface (\w+) \{/g).forEach((match) => {
		prefixes.add(match[1]);
	});

	content.matchAll(/enum (\w+) \{/g).forEach((match) => {
		prefixes.add(match[1]);
	});

	prefixes.forEach((prefix) => {
		console.log(`Replacing: ${prefix} -> I${prefix}`);
		content = content.replaceAll(
			new RegExp(`\\b${prefix}(?=[ ;\\[])`, "g"),
			`I${prefix}`,
		);
	});

	writeFileSync(outputFilePath, content);
});

console.log("Schema generation completed.");
