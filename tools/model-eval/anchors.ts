import { resolve } from "node:path";
import { readFile } from "node:fs/promises";
import { computeClassification } from "./normalize";
import type { AAModel, GlobalMaxes } from "./types";
const ROOT = "/Users/felix/Git/flow-like";
const r = JSON.parse(await readFile(resolve(ROOT, "tmp/results.json"), "utf-8"));
const models: AAModel[] = r.data, maxes: GlobalMaxes = r.globalMaxes;
const prices = models.map(m => m.pricing.price_1m_blended_3_to_1).filter((p): p is number => p != null && p > 0);
const want = ["claude-opus-5","gpt-6-astra-high","glm-5-3-max","qwen3-8-27b","gemma-4-31b","qwen3-5-9b",
  "granite-4-2-8b","minicpm5-2b","ling-3-0-tiny","qwen3-5-4b","llama-3-1-instruct-8b","mistral-small-4"];
const F = ["coding","reasoning","factuality","function_calling","multilinguality","safety","creativity"] as const;
console.log(["slug".padEnd(26), ...F.map(f=>f.slice(0,7).padStart(8))].join(""));
for (const s of want) {
  const m = models.find(x => x.slug === s);
  if (!m) { console.log(s.padEnd(26) + "  (no row)"); continue; }
  const { classification: c } = computeClassification(m, maxes, prices, false);
  console.log([s.padEnd(26), ...F.map(f => String((c as any)[f]).padStart(8))].join(""));
}
