import dotenv from "dotenv";
dotenv.config();

import { defineConfig } from "drizzle-kit";

const dbUrl = process.env.DATABASE_URL ?? "";

export default defineConfig({
	dialect: "postgresql",
	dbCredentials: {
		url: dbUrl,
	},
});
