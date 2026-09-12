import { resolve } from "node:path";
import { fetchModelsWithCache } from "./fetch";
const ROOT = resolve(import.meta.dir, "../..");
const key = process.env.ARTIFICIAL_ANALYSIS;
if (!key) { console.error("missing ARTIFICIAL_ANALYSIS"); process.exit(1); }
const { models, maxes } = await fetchModelsWithCache(key, ROOT);
console.error(`enriched ${models.length} models; results.json written`);
