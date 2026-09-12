#!/usr/bin/env bun
// Computes classifications for a set of model slugs using the same normalization
// as the pipeline, without touching the database.
import { resolve } from "node:path";
import { readFile, writeFile } from "node:fs/promises";
import { computeClassification } from "./normalize";
import type { AAModel, GlobalMaxes } from "./types";

const ROOT = resolve(import.meta.dir, "../..");
const results = JSON.parse(await readFile(resolve(ROOT, "tmp/results.json"), "utf-8"));
const models: AAModel[] = results.data;
const maxes: GlobalMaxes = results.globalMaxes;
const allPrices = models
  .map((m) => m.pricing.price_1m_blended_3_to_1)
  .filter((p): p is number => p != null && p > 0);

const slugs: string[] = JSON.parse(process.argv[2]);
const bySlug = new Map(models.map((m) => [m.slug, m]));
const out: Record<string, unknown> = {};
for (const slug of slugs) {
  const model = bySlug.get(slug);
  if (!model) { console.error(`no AA model for ${slug}`); continue; }
  const { classification, missingFields } = computeClassification(model, maxes, allPrices, false);
  out[slug] = classification;
  console.error(`${slug}: cost ${classification.cost} reasoning ${classification.reasoning}${missingFields.length ? ` (${missingFields.length} partial)` : ""}`);
}
await writeFile(process.argv[3], JSON.stringify(out, null, 1));
