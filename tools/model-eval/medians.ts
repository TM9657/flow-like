import { resolve } from "node:path";
import { readFile } from "node:fs/promises";
import { computeClassification } from "./normalize";
import type { AAModel, GlobalMaxes } from "./types";
const ROOT = resolve(import.meta.dir, "../..");
const r = JSON.parse(await readFile(resolve(ROOT, "tmp/results.json"), "utf-8"));
const models: AAModel[] = r.data;
const maxes: GlobalMaxes = r.globalMaxes;
const prices = models.map(m => m.pricing.price_1m_blended_3_to_1).filter((p): p is number => p != null && p > 0);
const FIELDS = ["coding","cost","creativity","factuality","function_calling","multilinguality","openness","reasoning","safety","speed"] as const;
const buckets: Record<string, number[]> = Object.fromEntries(FIELDS.map(f => [f, []]));
for (const m of models) {
  const { classification } = computeClassification(m, maxes, prices, true);
  for (const f of FIELDS) { const v = (classification as any)[f]; if (v > 0) buckets[f].push(v); }
}
const median = (a: number[]) => { const s=[...a].sort((x,y)=>x-y); return s.length ? Math.round(s[Math.floor(s.length/2)]*100)/100 : 0; };
console.log(JSON.stringify(Object.fromEntries(FIELDS.map(f => [f, {median: median(buckets[f]), n: buckets[f].length}])), null, 1));
