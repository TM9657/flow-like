import readline from "readline";

function ask(question: string): Promise<string> {
	const rl = readline.createInterface({
		input: process.stdin,
		output: process.stdout,
	});
	return new Promise((resolve) => {
		rl.question(question, (answer) => {
			rl.close();
			resolve(answer);
		});
	});
}

async function main() {
	console.log("Debug Auth Helper");
	console.log("=================");
	console.log("1. Internal OIDC (flow-like://auth)");
	console.log("2. Thirdparty OAuth/OIDC (flow-like://thirdparty/callback)");
	console.log("");

	const choice = await ask("Select type (1 or 2): ");
	const url = await ask("Callback URL: ");

	if (choice === "2") {
		// Thirdparty OAuth/OIDC callback
		console.log(`
// Paste this in your browser console:
window.dispatchEvent(
  new CustomEvent("debug-thirdparty", { detail: {
    url: "${url}",
  } })
);`);
	} else {
		// Internal OIDC
		console.log(`
// Paste this in your browser console:
window.dispatchEvent(
  new CustomEvent("debug-oidc", { detail: {
    url: "${url}",
  } })
);`);
	}
}

main();
