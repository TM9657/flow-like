#!/usr/bin/env bun
/**
 * Generate scoped sink trigger JWTs for Flow-Like services
 *
 * Usage:
 *   bun run tools/gen-sink-jwt.ts --type cron --secret YOUR_SECRET
 *   bun run tools/gen-sink-jwt.ts --type discord --secret YOUR_SECRET
 *   bun run tools/gen-sink-jwt.ts --type telegram --secret YOUR_SECRET
 *   bun run tools/gen-sink-jwt.ts --type all --secret YOUR_SECRET
 *
 * The generated JWTs are scoped to specific sink types for security isolation.
 * If a service is compromised, it can only trigger events of its own type.
 */

import * as jose from "jose";

interface SinkTriggerClaims {
	sub: "sink-trigger";
	iss: "flow-like";
	sink_types: string[];
	app_ids?: string[];
	iat: number;
	exp?: number;
}

const SINK_TYPES = ["cron", "discord", "telegram", "github", "rss", "mqtt", "email"] as const;

type SinkType = (typeof SINK_TYPES)[number];

async function generateJwt(sinkTypes: string[], secret: string, expiresIn?: string): Promise<string> {
	const encoder = new TextEncoder();
	const secretKey = encoder.encode(secret);

	const claims: Partial<SinkTriggerClaims> = {
		sub: "sink-trigger",
		iss: "flow-like",
		sink_types: sinkTypes,
	};

	let builder = new jose.SignJWT(claims as jose.JWTPayload).setProtectedHeader({ alg: "HS256" }).setIssuedAt();

	if (expiresIn) {
		builder = builder.setExpirationTime(expiresIn);
	}

	return builder.sign(secretKey);
}

async function main() {
	const args = process.argv.slice(2);

	let type: string | undefined;
	let secret: string | undefined;
	let expiresIn: string | undefined;

	for (let i = 0; i < args.length; i++) {
		if (args[i] === "--type" || args[i] === "-t") {
			type = args[++i];
		} else if (args[i] === "--secret" || args[i] === "-s") {
			secret = args[++i];
		} else if (args[i] === "--expires" || args[i] === "-e") {
			expiresIn = args[++i];
		} else if (args[i] === "--help" || args[i] === "-h") {
			printUsage();
			process.exit(0);
		}
	}

	// Try to get secret from environment if not provided
	if (!secret) {
		secret = process.env.SINK_TRIGGER_JWT_SECRET;
	}

	if (!type || !secret) {
		console.error("Error: --type and --secret are required\n");
		printUsage();
		process.exit(1);
	}

	if (type === "all") {
		console.log("Generating JWTs for all sink types:\n");
		console.log("=".repeat(60));

		for (const sinkType of SINK_TYPES) {
			const jwt = await generateJwt([sinkType], secret, expiresIn);
			console.log(`\n${sinkType.toUpperCase()}:`);
			console.log(`  ${sinkType.toUpperCase()}_TRIGGER_JWT=${jwt}`);
		}

		console.log("\n" + "=".repeat(60));
		console.log("\nAdd these to your .env file or secrets manager.");
	} else {
		if (!SINK_TYPES.includes(type as SinkType)) {
			console.error(`Error: Unknown sink type "${type}"`);
			console.error(`Valid types: ${SINK_TYPES.join(", ")}, all`);
			process.exit(1);
		}

		const jwt = await generateJwt([type], secret, expiresIn);

		console.log(`\n${type.toUpperCase()}_TRIGGER_JWT=${jwt}\n`);

		// Also output for easy copy-paste
		console.log("JWT Claims:");
		const [, payload] = jwt.split(".");
		const claims = JSON.parse(Buffer.from(payload, "base64").toString());
		console.log(JSON.stringify(claims, null, 2));
	}
}

function printUsage() {
	console.log(`
Generate scoped sink trigger JWTs for Flow-Like services

Usage:
  bun run tools/gen-sink-jwt.ts --type <type> --secret <secret> [--expires <duration>]

Options:
  -t, --type     Sink type: ${SINK_TYPES.join(", ")}, or "all"
  -s, --secret   JWT signing secret (or set SINK_TRIGGER_JWT_SECRET env var)
  -e, --expires  Optional expiration (e.g., "10y" for 10 years, "30d" for 30 days)
  -h, --help     Show this help

Examples:
  # Generate JWT for cron service (no expiration)
  bun run tools/gen-sink-jwt.ts --type cron --secret my-super-secret

  # Generate JWT for discord bot with 10 year expiration
  bun run tools/gen-sink-jwt.ts --type discord --secret my-super-secret --expires 10y

  # Generate JWTs for all sink types
  bun run tools/gen-sink-jwt.ts --type all --secret my-super-secret

Security Notes:
  - Each service should get its own scoped JWT
  - If a service is compromised, it can only trigger events of its own type
  - Store JWTs securely (secrets manager, not in source control)
  - Consider rotating JWTs periodically
`);
}

main().catch(console.error);
