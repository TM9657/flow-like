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
	const url = await ask("Callback URL: ");
	console.log(`
window.dispatchEvent(
  new CustomEvent("debug-oidc", { detail: {
    url: "${url}",
  } })
);`);
}

main();
