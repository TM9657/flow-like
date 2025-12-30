import "dotenv/config";
import { defineConfig, env } from "prisma/config";

export default defineConfig({
	// the main entry for your schema
	schema: "prisma/schema/",
	// where migrations should be generated
	// what script to run for "prisma db seed"
	migrations: {
		path: "prisma/migrations",
		seed: "tsx prisma/seed.ts",
	},
	// The database URL
	datasource: {
		// Type Safe env() helper
		// Does not replace the need for dotenv
		url: env("DATABASE_URL"),
	},
});
