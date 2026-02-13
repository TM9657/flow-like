#!/usr/bin/env python3
"""
Model Rater
- Fuzzy-search a model name across multiple registries (OpenRouter, Hugging Face search, local Ollama, plus DB)
- Let user select an exact model
- Fetch raw metrics (benchmarks/pricing/metadata)
- Normalize to 0..1 in categories and store everything in a local SQLite DB
- Re-score all models whenever the "standard" (config) or cohort changes

Dependencies (pip):
  requests
  huggingface_hub
  pandas
  pyarrow

Optional:
  rapidfuzz  (better fuzzy matching)

Notes:
- This script relies on *real benchmark sources* (Arena, BigCodeBench, Open LLM Leaderboard, COMPL-AI)
  plus pricing/feature metadata (OpenRouter) and basic HF metadata (languages).
- If a model has no matching entries in those sources, the score will remain 0.5 with confidence 0.0
  (and the report will tell you which metrics were missing).
"""

from __future__ import annotations

import argparse
import contextlib
import csv
import dataclasses
import datetime as dt
import hashlib
import io
import json
import math
import os
import re
import sqlite3
import time
import urllib.parse
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import requests

try:
    import pandas as pd  # type: ignore
except Exception:  # pragma: no cover
    pd = None  # type: ignore


# -----------------------------
# Scoring "standard" (versioned)
# -----------------------------
DEFAULT_STANDARD: Dict[str, Any] = {
    "name": "default-v5",
    # Per-category: weighted mix of raw metrics (normalized across DB cohort unless a fixed scale is given).
    # If metrics are missing, we fall back to other metrics.
    "categories": {
        "coding": {
            "metrics": [
                {"key": "bigcodebench_instruct", "better": "higher", "weight": 0.75, "transform": None, "scale": {"min": 0, "max": 100}},
                {"key": "bigcodebench_complete", "better": "higher", "weight": 0.25, "transform": None, "scale": {"min": 0, "max": 100}},
            ],
            "fallbacks": [
                {"key": "arena_score", "better": "higher", "weight": 1.0, "transform": None},
            ],
        },
        "context_length": {
            # Context window size (tokens). Larger = higher score. Log scale because 8k vs 32k matters more than 128k vs 200k.
            "metrics": [
                {"key": "context_length_tokens", "better": "higher", "weight": 1.0, "transform": "log1p", "scale": {"min": 2048, "max": 2000000}},
            ],
            "fallbacks": [],
        },
        "cost": {
            # Lower $/1M tokens should score higher. Fixed scale based on market range.
            "metrics": [
                {"key": "cost_usd_per_1m_mixed", "better": "lower", "weight": 1.0, "transform": "log1p", "scale": {"min": 0.01, "max": 100}},
            ],
            "fallbacks": [
                # local models: treat as "cheap" vs API
                {"key": "cost_is_local_proxy", "better": "higher", "weight": 1.0, "transform": None, "scale": "binary"},
            ],
        },
        "creativity": {
            "metrics": [
                {"key": "arena_score", "better": "higher", "weight": 1.0, "transform": None},
            ],
            "fallbacks": [],
        },
        "factuality": {
            "metrics": [
                {"key": "openllm_gpqa_acc_norm", "better": "higher", "weight": 1.0, "transform": None, "scale": "unit"},
            ],
            "fallbacks": [
                {"key": "openllm_truthfulqa_mc2", "better": "higher", "weight": 1.0, "transform": None, "scale": "unit"},
                {"key": "complai_truthful_qa_mc2", "better": "higher", "weight": 1.0, "transform": None, "scale": "unit"},
                {"key": "arena_score", "better": "higher", "weight": 1.0, "transform": None},
            ],
        },
        "function_calling": {
            "metrics": [
                # Real benchmark: BFCL (Berkeley Function Calling Leaderboard). Populate via `ingest-bfcl`.
                {"key": "bfcl_v3_score", "better": "higher", "weight": 1.0, "transform": None, "scale": "unit"},
            ],
            "fallbacks": [
                # Proxy: tool-calling support flag from OpenRouter
                {"key": "openrouter_tools_supported", "better": "higher", "weight": 1.0, "transform": None, "scale": "binary"},
            ],
        },
        "multilinguality": {
            "metrics": [
                # MMMLU: professionally translated MMLU in 14 languages (OpenAI benchmark)
                {"key": "mmmlu_avg", "better": "higher", "weight": 1.0, "transform": None, "scale": "unit"},
            ],
            "fallbacks": [
                {"key": "openllm_mgsm_exact_match", "better": "higher", "weight": 0.6, "transform": None, "scale": "unit"},
                {"key": "openllm_xnli_acc", "better": "higher", "weight": 0.4, "transform": None, "scale": "unit"},
                {"key": "hf_language_count", "better": "higher", "weight": 1.0, "transform": "log1p"},
            ],
        },
        "openness": {
            "metrics": [
                {"key": "complai_openness_mean", "better": "higher", "weight": 1.0, "transform": None, "scale": "unit"},
            ],
            "fallbacks": [
                {"key": "openrouter_is_moderated", "better": "lower", "weight": 1.0, "transform": None, "scale": "binary"},
            ],
        },
        "reasoning": {
            "metrics": [
                {"key": "openllm_bbh_acc_norm", "better": "higher", "weight": 0.34, "transform": None, "scale": "unit"},
                {"key": "openllm_math_hard_exact_match", "better": "higher", "weight": 0.33, "transform": None, "scale": "unit"},
                {"key": "openllm_gpqa_acc_norm", "better": "higher", "weight": 0.33, "transform": None, "scale": "unit"},
            ],
            "fallbacks": [
                {"key": "arena_score", "better": "higher", "weight": 1.0, "transform": None},
            ],
        },
        "safety": {
            "metrics": [
                {"key": "complai_safety_enterprise_mean", "better": "higher", "weight": 0.5, "transform": None, "scale": "unit"},
                {"key": "complai_eu_ai_act_mean", "better": "higher", "weight": 0.3, "transform": None, "scale": "unit"},
                {"key": "complai_overall_mean", "better": "higher", "weight": 0.2, "transform": None, "scale": "unit"},
            ],
            "fallbacks": [
                {"key": "openrouter_is_moderated", "better": "higher", "weight": 1.0, "transform": None, "scale": "binary"},
            ],
        },
        "speed": {
            # Real measured TPS when available; otherwise model size proxy (smaller = faster)
            "metrics": [
                {"key": "measured_tokens_per_sec", "better": "higher", "weight": 1.0, "transform": "log1p", "scale": {"min": 1, "max": 1000}},
                {"key": "ollama_measured_tokens_per_sec", "better": "higher", "weight": 1.0, "transform": "log1p", "scale": {"min": 1, "max": 200}},
            ],
            "fallbacks": [
                # Proxy: cheaper models correlate with faster inference (not perfect but better than nothing)
                {"key": "cost_usd_per_1m_mixed", "better": "lower", "weight": 1.0, "transform": "log1p", "scale": {"min": 0.01, "max": 100}},
            ],
        },
        "structured_output": {
            # Ability to produce structured/JSON output reliably
            "metrics": [
                {"key": "openrouter_structured_outputs_supported", "better": "higher", "weight": 1.0, "transform": None, "scale": "binary"},
            ],
            "fallbacks": [],
        },
    },
}

# -----------------------------
# Data sources (IDs / locations)
# -----------------------------
OPENROUTER_MODELS_URL = "https://openrouter.ai/api/v1/models"

HF_ARENA_DATASET = "mathewhe/chatbot-arena-elo"
HF_ARENA_FILE = "elo.csv"

HF_BIGCODEBENCH_RESULTS = "bigcode/bigcodebench-results"
HF_OPENLLM_RESULTS = "open-llm-leaderboard/results"

COMPLAI_BOARD_SPACE = "latticeflow/compl-ai-board"
COMPLAI_CACHE_FILE = "complai_index_cache.json"
COMPLAI_CACHE_TTL_SECS = 7 * 24 * 3600

HF_MODELS_SEARCH_URL = "https://huggingface.co/api/models"

# MMMLU (Multilingual MMLU) benchmark results from OpenAI simple-evals
MMMLU_RESULTS_URL = "https://raw.githubusercontent.com/openai/simple-evals/main/multilingual_mmlu_benchmark_results.md"
MMMLU_CACHE_FILE = "mmmlu_results_cache.json"
MMMLU_CACHE_TTL_SECS = 7 * 24 * 3600

# BFCL (Berkeley Function Calling Leaderboard) results
BFCL_RESULTS_URL = "https://raw.githubusercontent.com/HuanzhiMao/BFCL-Result/main/2025-11-03/score/data_overall.csv"
BFCL_CACHE_FILE = "bfcl_results_cache.json"
BFCL_CACHE_TTL_SECS = 7 * 24 * 3600

OLLAMA_HOST = os.getenv("OLLAMA_HOST", "http://localhost:11434").rstrip("/")

DEFAULT_CACHE_TTL_SECS = 24 * 3600

SCRIPT_CACHE_DIR = Path(__file__).resolve().parent / ".cache"


# -----------------------------
# Utilities
# -----------------------------
def now_iso() -> str:
    # timezone-aware UTC (avoids utcnow deprecation warning)
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def sha256_json(obj: Any) -> str:
    raw = json.dumps(obj, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(raw).hexdigest()


def try_import_rapidfuzz():
    try:
        from rapidfuzz import fuzz  # type: ignore
        return fuzz
    except Exception:
        return None


def fuzzy_score(a: str, b: str) -> float:
    """Returns similarity 0..100"""
    fuzz = try_import_rapidfuzz()
    if fuzz is not None:
        return float(fuzz.WRatio(a, b))
    import difflib

    return difflib.SequenceMatcher(None, a.lower(), b.lower()).ratio() * 100.0


def _http_get_json(url: str, *, headers: Optional[dict] = None, timeout: int = 30) -> Any:
    r = requests.get(url, headers=headers or {}, timeout=timeout)
    r.raise_for_status()
    return r.json()


def _http_get_bytes(url: str, *, headers: Optional[dict] = None, timeout: int = 60) -> bytes:
    r = requests.get(url, headers=headers or {}, timeout=timeout)
    r.raise_for_status()
    return r.content


def hf_api_tree(repo_type: str, repo_id: str, *, recursive: bool = True) -> List[dict]:
    # repo_type: "datasets" | "spaces"
    rec = "true" if recursive else "false"
    url = f"https://huggingface.co/api/{repo_type}/{repo_id}/tree/main?recursive={rec}"
    data = _http_get_json(url, timeout=60)
    return list(data or [])


def hf_resolve_url(repo_type: str, repo_id: str, path: str) -> str:
    # Use /resolve/main/<path> (HF handles redirect + caching).
    qpath = urllib.parse.quote(path)
    return f"https://huggingface.co/{repo_type}/{repo_id}/resolve/main/{qpath}"


def hf_download_cached(cache_dir: Path, repo_type: str, repo_id: str, path: str, *, ttl_secs: int) -> Path:
    safe = re.sub(r"[^a-zA-Z0-9._/-]+", "_", f"{repo_type}/{repo_id}/{path}")
    local = cache_dir / safe
    local.parent.mkdir(parents=True, exist_ok=True)
    if local.exists():
        age = time.time() - local.stat().st_mtime
        if age <= ttl_secs:
            return local
    local.write_bytes(_http_get_bytes(hf_resolve_url(repo_type, repo_id, path), timeout=120))
    return local


def clamp01(x: float) -> float:
    return max(0.0, min(1.0, x))


def transform_value(x: float, transform: Optional[str]) -> float:
    if transform is None:
        return x
    if transform == "log1p":
        return math.log1p(max(0.0, x))
    if transform == "cap_10":
        return min(10.0, max(0.0, x))
    raise ValueError(f"Unknown transform: {transform}")


def norm_name(s: str) -> str:
    """
    Normalize names for cross-source matching:
    - strip provider prefixes like "OpenAI:" / "Anthropic:"
    - lower
    - remove punctuation
    - collapse whitespace
    """
    s = (s or "").strip()
    s = re.sub(r"^\s*[\w .-]+\s*:\s*", "", s)  # "OpenAI: GPT-5.1" -> "GPT-5.1"
    s = s.lower()
    s = re.sub(r"[^a-z0-9]+", " ", s)
    s = re.sub(r"\s+", " ", s).strip()
    return s


def name_variants(display_name: str, openrouter_id: Optional[str], hf_repo_id: Optional[str]) -> List[str]:
    vs = []
    for x in [display_name, openrouter_id or "", hf_repo_id or ""]:
        x = x.strip()
        if x:
            vs.append(x)
            vs.append(norm_name(x))
    # also add short-id variants (e.g. "openai/gpt-5.1" -> "gpt-5.1")
    for x in [openrouter_id, hf_repo_id]:
        if x and "/" in x:
            vs.append(x.split("/", 1)[1])
            vs.append(norm_name(x.split("/", 1)[1]))
    # dedup keep order
    out: List[str] = []
    seen = set()
    for v in vs:
        if v and v not in seen:
            seen.add(v)
            out.append(v)
    return out


# -----------------------------
# COMPL-AI (index + metrics)
# -----------------------------
@dataclass
class ComplAIEntry:
    model_name: str
    model_report: Optional[str]
    results: Dict[str, Optional[float]]


def load_complai_index(cache_path: Path, force_refresh: bool = False) -> Dict[str, ComplAIEntry]:
    if cache_path.exists() and not force_refresh:
        age = time.time() - cache_path.stat().st_mtime
        if age < COMPLAI_CACHE_TTL_SECS:
            raw = json.loads(cache_path.read_text("utf-8"))
            return {
                k: ComplAIEntry(
                    model_name=v["model_name"],
                    model_report=v.get("model_report"),
                    results=v["results"],
                )
                for k, v in raw.items()
            }

    files = hf_api_tree("spaces", COMPLAI_BOARD_SPACE, recursive=True)
    json_files = [d.get("path", "") for d in files if isinstance(d, dict) and d.get("type") == "file"]
    json_files = [p for p in json_files if p.startswith("results/") and p.endswith(".json")]

    idx: Dict[str, ComplAIEntry] = {}
    for f in json_files:
        try:
            data = json.loads(_http_get_bytes(hf_resolve_url("spaces", COMPLAI_BOARD_SPACE, f), timeout=120).decode("utf-8"))
            model_name = (data.get("config") or {}).get("model_name")
            model_report = (data.get("config") or {}).get("model_report")
            results = {
                k: (v.get("aggregate_score") if isinstance(v, dict) else None)
                for k, v in (data.get("results") or {}).items()
            }
            if model_name:
                idx[str(model_name)] = ComplAIEntry(model_name=str(model_name), model_report=model_report, results=results)
        except Exception:
            continue

    cache_path.write_text(
        json.dumps(
            {k: {"model_name": v.model_name, "model_report": v.model_report, "results": v.results} for k, v in idx.items()},
            ensure_ascii=False,
            indent=2,
        ),
        "utf-8",
    )
    return idx


def complai_best_match(idx: Dict[str, ComplAIEntry], variants: List[str]) -> Tuple[Optional[ComplAIEntry], float]:
    best_e: Optional[ComplAIEntry] = None
    best_s = 0.0
    for q in variants:
        for k, e in idx.items():
            s = max(fuzzy_score(q, k), fuzzy_score(norm_name(q), norm_name(k)))
            if s > best_s:
                best_s = s
                best_e = e
    return best_e, best_s


COMPLAI_SAFETY_KEY_HINTS = [
    # these are "hints" that work across common naming schemes (we match by substring)
    "tox", "toxicity",
    "privacy",
    "memor",
    "injection", "hijacking", "goal",
    "bias", "fair", "bbq", "bold",
    "deception",
]

COMPLAI_OPENNESS_KEY_HINTS = [
    # Heuristics: openness ~ low over-refusal/overblocking.
    "overrefusal",
    "over_refusal",
    "over-refusal",
    "overblock",
    "over_block",
    "over-block",
    "xstest",
    "xs_test",
    "refusal_benign",
    "excessive_refusal",
]

COMPLAI_EU_AI_ACT_KEY_HINTS = [
    # Heuristics: compliance/enterprise readiness signals.
    "gdpr",
    "privacy",
    "pii",
    "personal_data",
    "data_protection",
    "transparen",
    "govern",
    "audit",
    "logging",
    "risk",
    "bias",
    "fair",
]


def complai_metrics_for_any(script_dir: Path, display_name: str, openrouter_id: Optional[str], hf_repo_id: Optional[str]) -> Tuple[Dict[str, float], List[str], Optional[str], float]:
    idx = load_complai_index(script_dir / COMPLAI_CACHE_FILE)
    variants = name_variants(display_name, openrouter_id, hf_repo_id)
    entry, score = complai_best_match(idx, variants)
    if entry is None or score < 90.0:
        return {}, [], None, score

    metrics: Dict[str, float] = {}
    vals_all: List[float] = []
    vals_safety: List[float] = []
    vals_openness: List[float] = []
    vals_eu: List[float] = []

    for k, v in entry.results.items():
        if not isinstance(v, (int, float)):
            continue
        fv = float(v)
        metrics[f"complai_{k}"] = fv
        vals_all.append(fv)
        lk = k.lower()
        if any(h in lk for h in COMPLAI_SAFETY_KEY_HINTS):
            vals_safety.append(fv)
        if any(h in lk for h in COMPLAI_OPENNESS_KEY_HINTS):
            vals_openness.append(fv)
        if any(h in lk for h in COMPLAI_EU_AI_ACT_KEY_HINTS):
            vals_eu.append(fv)

    if vals_all:
        metrics["complai_overall_mean"] = float(sum(vals_all) / len(vals_all))
    if vals_safety:
        metrics["complai_safety_enterprise_mean"] = float(sum(vals_safety) / len(vals_safety))
    if vals_openness:
        metrics["complai_openness_mean"] = float(sum(vals_openness) / len(vals_openness))
    if vals_eu:
        metrics["complai_eu_ai_act_mean"] = float(sum(vals_eu) / len(vals_eu))

    links: List[str] = []
    if entry.model_report:
        links.append(entry.model_report)
    return metrics, links, entry.model_name, score


# -----------------------------
# Candidate model representation
# -----------------------------
@dataclasses.dataclass
class Candidate:
    source: str  # "openrouter" | "ollama" | "hf" | "arena" | "db"
    name: str
    provider: str
    provider_id: str
    openrouter_id: Optional[str] = None
    hf_repo_id: Optional[str] = None
    extra: Dict[str, Any] = dataclasses.field(default_factory=dict)


# -----------------------------
# SQLite
# -----------------------------
def db_path_for_script(script_path: Path) -> Path:
    return script_path.with_name("model_ratings.sqlite3")


def connect_db(db_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    return conn


def ensure_column(conn: sqlite3.Connection, table: str, col: str, coldef: str) -> None:
    cols = [r["name"] for r in conn.execute(f"PRAGMA table_info({table})").fetchall()]
    if col not in cols:
        conn.execute(f"ALTER TABLE {table} ADD COLUMN {col} {coldef}")


def init_db(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS models (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            display_name TEXT NOT NULL,
            provider TEXT,
            provider_id TEXT,
            openrouter_id TEXT,
            hf_repo_id TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            retrieved_at TEXT NOT NULL,
            blob_json TEXT
        );

        CREATE TABLE IF NOT EXISTS raw_metrics (
            model_id INTEGER NOT NULL,
            key TEXT NOT NULL,
            value REAL NOT NULL,
            unit TEXT,
            source_id INTEGER,
            retrieved_at TEXT NOT NULL,
            PRIMARY KEY(model_id, key),
            FOREIGN KEY(model_id) REFERENCES models(id),
            FOREIGN KEY(source_id) REFERENCES sources(id)
        );

        CREATE TABLE IF NOT EXISTS links (
            model_id INTEGER NOT NULL,
            kind TEXT NOT NULL,
            url TEXT NOT NULL,
            title TEXT,
            source_id INTEGER,
            created_at TEXT NOT NULL,
            PRIMARY KEY(model_id, kind, url),
            FOREIGN KEY(model_id) REFERENCES models(id),
            FOREIGN KEY(source_id) REFERENCES sources(id)
        );

        CREATE TABLE IF NOT EXISTS standards (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            config_hash TEXT NOT NULL UNIQUE,
            config_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scores (
            model_id INTEGER NOT NULL,
            standard_id INTEGER NOT NULL,
            category TEXT NOT NULL,
            score REAL NOT NULL,
            confidence REAL NOT NULL,
            details_json TEXT NOT NULL,
            computed_at TEXT NOT NULL,
            PRIMARY KEY(model_id, standard_id, category),
            FOREIGN KEY(model_id) REFERENCES models(id),
            FOREIGN KEY(standard_id) REFERENCES standards(id)
        );
        """
    )

    ensure_column(conn, "models", "provider", "TEXT")
    ensure_column(conn, "models", "provider_id", "TEXT")
    ensure_column(conn, "models", "openrouter_id", "TEXT")
    ensure_column(conn, "models", "hf_repo_id", "TEXT")

    with contextlib.suppress(Exception):
        conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_models_provider ON models(provider, provider_id)")

    conn.commit()


def upsert_source(conn: sqlite3.Connection, name: str, url: str, blob: Optional[dict]) -> int:
    retrieved_at = now_iso()
    blob_json = json.dumps(blob) if blob is not None else None
    conn.execute(
        "INSERT INTO sources(name, url, retrieved_at, blob_json) VALUES(?,?,?,?)",
        (name, url, retrieved_at, blob_json),
    )
    conn.commit()
    return int(conn.execute("SELECT last_insert_rowid()").fetchone()[0])


def upsert_model(conn: sqlite3.Connection, display_name: str, provider: str, provider_id: str, openrouter_id: Optional[str], hf_repo_id: Optional[str]) -> int:
    created_at = now_iso()

    row = conn.execute("SELECT id FROM models WHERE provider=? AND provider_id=?", (provider, provider_id)).fetchone()
    if row is not None:
        mid = int(row["id"])
        conn.execute(
            "UPDATE models SET display_name=?, openrouter_id=COALESCE(?, openrouter_id), hf_repo_id=COALESCE(?, hf_repo_id) WHERE id=?",
            (display_name, openrouter_id, hf_repo_id, mid),
        )
        conn.commit()
        return mid

    conn.execute(
        """
        INSERT INTO models(display_name, provider, provider_id, openrouter_id, hf_repo_id, created_at)
        VALUES(?,?,?,?,?,?)
        """,
        (display_name, provider, provider_id, openrouter_id, hf_repo_id, created_at),
    )
    conn.commit()
    return int(conn.execute("SELECT last_insert_rowid()").fetchone()[0])


def set_model_hf_repo_id(conn: sqlite3.Connection, model_id: int, hf_repo_id: Optional[str]) -> None:
    conn.execute("UPDATE models SET hf_repo_id=? WHERE id=?", (hf_repo_id, int(model_id)))
    conn.commit()


def upsert_link(conn: sqlite3.Connection, model_id: int, kind: str, url: str, title: Optional[str], source_id: Optional[int]) -> None:
    conn.execute(
        """
        INSERT OR IGNORE INTO links(model_id, kind, url, title, source_id, created_at)
        VALUES(?,?,?,?,?,?)
        """,
        (model_id, kind, url, title, source_id, now_iso()),
    )


def upsert_metric(conn: sqlite3.Connection, model_id: int, key: str, value: float, unit: Optional[str], source_id: Optional[int]) -> None:
    conn.execute(
        """
        INSERT INTO raw_metrics(model_id, key, value, unit, source_id, retrieved_at)
        VALUES(?,?,?,?,?,?)
        ON CONFLICT(model_id, key) DO UPDATE SET
          value=excluded.value,
          unit=excluded.unit,
          source_id=excluded.source_id,
          retrieved_at=excluded.retrieved_at
        """,
        (model_id, key, float(value), unit, source_id, now_iso()),
    )


def get_or_create_standard(conn: sqlite3.Connection, standard: dict) -> int:
    h = sha256_json(standard)
    row = conn.execute("SELECT id FROM standards WHERE config_hash=?", (h,)).fetchone()
    if row is not None:
        return int(row["id"])
    conn.execute(
        "INSERT INTO standards(name, config_hash, config_json, created_at) VALUES(?,?,?,?)",
        (standard.get("name", "unnamed"), h, json.dumps(standard), now_iso()),
    )
    conn.commit()
    return int(conn.execute("SELECT last_insert_rowid()").fetchone()[0])


# -----------------------------
# Fetch: OpenRouter models
# -----------------------------
def fetch_openrouter_models() -> List[dict]:
    headers = {}
    api_key = os.getenv("OPENROUTER_API_KEY")
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    r = requests.get(OPENROUTER_MODELS_URL, headers=headers, timeout=30)
    r.raise_for_status()
    data = r.json()
    return list(data.get("data", []))


def fetch_openrouter_models_cached(cache_dir: Path, *, ttl_secs: int = 3600, force_refresh: bool = False) -> List[dict]:
    cache_file = cache_dir / "openrouter_models.json"
    if not force_refresh:
        cached = None
        try:
            if cache_file.exists() and (time.time() - cache_file.stat().st_mtime) <= ttl_secs:
                cached = json.loads(cache_file.read_text("utf-8"))
        except Exception:
            cached = None
        if isinstance(cached, list):
            return cached

    models = fetch_openrouter_models()
    try:
        cache_file.parent.mkdir(parents=True, exist_ok=True)
        cache_file.write_text(json.dumps(models, ensure_ascii=False, indent=2), "utf-8")
    except Exception:
        pass
    return models


# -----------------------------
# Fetch: Hugging Face search
# -----------------------------
def fetch_hf_model_search(query: str, limit: int = 25) -> List[dict]:
    params = {"search": query, "limit": str(limit)}
    r = requests.get(HF_MODELS_SEARCH_URL, params=params, timeout=30)
    r.raise_for_status()
    return list(r.json() or [])


def _best_hf_repo_for_openllm(display_name: str, openrouter_id: str, limit: int = 25) -> Tuple[Optional[str], float]:
    """Return (hf_repo_id, score) using HF search + fuzzy matching.

    Only returns a repo if it has Open LLM Leaderboard results available.
    """
    queries: List[str] = []
    if display_name.strip():
        queries.append(display_name.strip())
    if openrouter_id.strip():
        queries.append(openrouter_id.strip())
        if "/" in openrouter_id:
            queries.append(openrouter_id.split("/", 1)[1].strip())

    seen: set[str] = set()
    hf_models: List[dict] = []
    for q in queries:
        if not q or q in seen:
            continue
        seen.add(q)
        try:
            hf_models.extend(fetch_hf_model_search(q, limit=limit))
        except Exception:
            continue

    best_id: Optional[str] = None
    best_score = -1.0
    variants = name_variants(display_name, openrouter_id, None)
    for m in hf_models:
        mid = str(m.get("modelId") or "").strip()
        if not mid:
            continue
        s = max(
            max(fuzzy_score(v, mid) for v in variants),
            max(fuzzy_score(norm_name(v), norm_name(mid)) for v in variants),
        )
        if s > best_score:
            best_score = s
            best_id = mid

    if not best_id or best_score < 88.0:
        return (None, 0.0)

    openllm_json, _ = fetch_openllm_results_json(best_id)
    if openllm_json is None:
        return (None, 0.0)

    return (best_id, float(best_score))


def _best_hf_repo_for_metadata(display_name: str, openrouter_id: str, limit: int = 25) -> Tuple[Optional[str], float]:
    """Return (hf_repo_id, score) for HF metadata linkage (languages/tags).

    Unlike `_best_hf_repo_for_openllm`, this does not require Open LLM results.
    """
    queries: List[str] = []
    if display_name.strip():
        queries.append(display_name.strip())
    if openrouter_id.strip():
        queries.append(openrouter_id.strip())
        if "/" in openrouter_id:
            queries.append(openrouter_id.split("/", 1)[1].strip())

    seen: set[str] = set()
    hf_models: List[dict] = []
    for q in queries:
        if not q or q in seen:
            continue
        seen.add(q)
        try:
            hf_models.extend(fetch_hf_model_search(q, limit=limit))
        except Exception:
            continue

    best_id: Optional[str] = None
    best_score = -1.0
    variants = name_variants(display_name, openrouter_id, None)
    for m in hf_models:
        mid = str(m.get("modelId") or "").strip()
        if not mid:
            continue
        s = max(
            max(fuzzy_score(v, mid) for v in variants),
            max(fuzzy_score(norm_name(v), norm_name(mid)) for v in variants),
        )
        if s > best_score:
            best_score = s
            best_id = mid

    if not best_id or best_score < 92.0:
        return (None, 0.0)

    if fetch_hf_model_metadata(best_id) is None:
        return (None, 0.0)
    return (best_id, float(best_score))


# -----------------------------
# Fetch: Ollama local models
# -----------------------------
def fetch_ollama_tags() -> Optional[dict]:
    url = f"{OLLAMA_HOST}/api/tags"
    try:
        r = requests.get(url, timeout=10)
        r.raise_for_status()
        return r.json()
    except Exception:
        return None


def measure_speed_ollama(model_name: str, max_tokens: int = 128) -> Optional[Tuple[float, dict]]:
    """
    Measure tokens/sec for Ollama:
      tokens/sec = eval_count / eval_duration * 1e9   (eval_duration in ns)
    """
    url = f"{OLLAMA_HOST}/api/generate"
    payload = {
        "model": model_name,
        "prompt": "Return a short numbered list of 5 animals.",
        "stream": False,
        "options": {"temperature": 0.2, "num_predict": max_tokens},
    }
    try:
        r = requests.post(url, json=payload, timeout=120)
        r.raise_for_status()
        data = r.json()
        eval_count = float(data.get("eval_count") or 0.0)
        eval_duration = float(data.get("eval_duration") or 0.0)  # ns
        if eval_count <= 0 or eval_duration <= 0:
            return None
        tps = eval_count / eval_duration * 1e9
        return (float(tps), data)
    except Exception:
        return None


# -----------------------------
# Fetch: HF datasets files
# -----------------------------
def hf_download_dataset_file(repo_id: str, filename: str) -> Path:
    cache_dir = SCRIPT_CACHE_DIR / "hf"
    return hf_download_cached(cache_dir, "datasets", repo_id, filename, ttl_secs=DEFAULT_CACHE_TTL_SECS)


def hf_list_dataset_files(repo_id: str) -> List[str]:
    data = hf_api_tree("datasets", repo_id, recursive=True)
    return [d.get("path", "") for d in data if isinstance(d, dict) and d.get("type") == "file"]


def hf_list_dataset_tree(repo_id: str, path_in_repo: str) -> List[str]:
    data = hf_api_tree("datasets", repo_id, recursive=True)
    out: List[str] = []
    prefix = (path_in_repo.rstrip("/") + "/") if path_in_repo else ""
    for d in data:
        if not isinstance(d, dict) or d.get("type") != "file":
            continue
        p = str(d.get("path") or "")
        if prefix and not p.startswith(prefix):
            continue
        out.append(p)
    return out


# -----------------------------
# Load: Chatbot Arena Elo CSV
# -----------------------------
def load_arena_csv() -> List[dict]:
    p = hf_download_dataset_file(HF_ARENA_DATASET, HF_ARENA_FILE)
    rows: List[dict] = []
    with p.open("r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)
    return rows


# -----------------------------
# Load: BigCodeBench results (Parquet)
# -----------------------------
def load_bigcodebench_results() -> Any:
    if pd is None:
        raise RuntimeError("pandas/pyarrow are required to load BigCodeBench parquet")
    files = hf_list_dataset_files(HF_BIGCODEBENCH_RESULTS)
    parquet_files = [f for f in files if f.lower().endswith(".parquet")]
    if not parquet_files:
        raise RuntimeError(f"No parquet files found in dataset {HF_BIGCODEBENCH_RESULTS}")
    parquet_files.sort(key=lambda s: (("train" not in s.lower()), len(s)))
    chosen = parquet_files[0]
    p = hf_download_dataset_file(HF_BIGCODEBENCH_RESULTS, chosen)
    df = pd.read_parquet(p)  # type: ignore[union-attr]
    return df


# -----------------------------
# Load: MMMLU (Multilingual MMLU) results from OpenAI simple-evals
# -----------------------------
_MMMLU_RESULTS_CACHE: Optional[Dict[str, Dict[str, float]]] = None


def load_mmmlu_results() -> Dict[str, Dict[str, float]]:
    """Load MMMLU benchmark results from OpenAI simple-evals.

    Returns dict: model_name -> {"AR_XY": score, "BN_BD": score, ..., "Average": score}
    """
    global _MMMLU_RESULTS_CACHE
    if _MMMLU_RESULTS_CACHE is not None:
        return _MMMLU_RESULTS_CACHE

    cache_dir = SCRIPT_CACHE_DIR / "mmmlu"
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache_file = cache_dir / MMMLU_CACHE_FILE

    if cache_file.exists():
        age = time.time() - cache_file.stat().st_mtime
        if age <= MMMLU_CACHE_TTL_SECS:
            try:
                with cache_file.open("r", encoding="utf-8") as f:
                    cached = json.load(f)
                if isinstance(cached, dict):
                    _MMMLU_RESULTS_CACHE = cached
                    return _MMMLU_RESULTS_CACHE
            except Exception:
                pass

    results: Dict[str, Dict[str, float]] = {}
    try:
        md_text = _http_get_bytes(MMMLU_RESULTS_URL, timeout=30).decode("utf-8")
        lines = md_text.splitlines()
        header_models: List[str] = []
        in_table = False
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("| Language"):
                parts = [p.strip() for p in stripped.split("|")]
                parts = [p for p in parts if p and p != "Language"]
                header_models = parts
                for m in header_models:
                    results[m] = {}
                in_table = True
                continue
            if in_table and stripped.startswith("|:") and "---" in stripped:
                continue
            if in_table and stripped.startswith("|") and "|" in stripped[1:]:
                parts = [p.strip() for p in stripped.split("|")]
                parts = [p for p in parts if p]
                if len(parts) < 2:
                    continue
                lang = parts[0]
                values = parts[1:]
                for i, val in enumerate(values):
                    if i >= len(header_models):
                        break
                    model = header_models[i]
                    try:
                        val_clean = val.replace("**", "").strip()
                        results[model][lang] = float(val_clean)
                    except Exception:
                        pass
            elif in_table and not stripped.startswith("|"):
                break
    except Exception:
        pass

    try:
        with cache_file.open("w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
    except Exception:
        pass

    _MMMLU_RESULTS_CACHE = results
    return results


def extract_mmmlu_metrics(model_name: str, openrouter_id: Optional[str] = None) -> Dict[str, Tuple[float, str]]:
    """Extract MMMLU metrics for a model.

    Uses fuzzy match on model name or OpenRouter ID.
    Returns mmmlu_avg (average score 0..1) and optionally per-language scores.
    """
    out: Dict[str, Tuple[float, str]] = {}
    data = load_mmmlu_results()
    if not data:
        return out

    variants: List[str] = []
    if model_name:
        variants.append(model_name)
        variants.append(norm_name(model_name))
    if openrouter_id:
        variants.append(openrouter_id)
        if "/" in openrouter_id:
            variants.append(openrouter_id.split("/", 1)[1])
        variants.append(norm_name(openrouter_id))

    best_model: Optional[str] = None
    best_score = -1.0
    for model_key in data:
        for v in variants:
            s = max(fuzzy_score(v, model_key), fuzzy_score(norm_name(v), norm_name(model_key)))
            if s > best_score:
                best_score = s
                best_model = model_key

    if not best_model or best_score < 80.0:
        return out

    model_data = data[best_model]
    avg = model_data.get("Average")
    if avg is not None:
        out["mmmlu_avg"] = (float(avg), "0..1 avg across 14 languages")

    return out


# -----------------------------
# Load: BFCL (Berkeley Function Calling Leaderboard) results
# -----------------------------
_BFCL_RESULTS_CACHE: Optional[Dict[str, Dict[str, Any]]] = None


def load_bfcl_results() -> Dict[str, Dict[str, Any]]:
    """Load BFCL benchmark results from GitHub.

    Returns dict: model_name -> {"overall_acc": float, "rank": int, "cost": float, "latency": float, ...}
    """
    global _BFCL_RESULTS_CACHE
    if _BFCL_RESULTS_CACHE is not None:
        return _BFCL_RESULTS_CACHE

    cache_dir = SCRIPT_CACHE_DIR / "bfcl"
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache_file = cache_dir / BFCL_CACHE_FILE

    if cache_file.exists():
        age = time.time() - cache_file.stat().st_mtime
        if age <= BFCL_CACHE_TTL_SECS:
            try:
                with cache_file.open("r", encoding="utf-8") as f:
                    cached = json.load(f)
                if isinstance(cached, dict):
                    _BFCL_RESULTS_CACHE = cached
                    return _BFCL_RESULTS_CACHE
            except Exception:
                pass

    results: Dict[str, Dict[str, Any]] = {}
    try:
        csv_bytes = _http_get_bytes(BFCL_RESULTS_URL, timeout=30)
        csv_text = csv_bytes.decode("utf-8")
        reader = csv.DictReader(io.StringIO(csv_text))
        for row in reader:
            model = (row.get("Model") or "").strip()
            if not model:
                continue
            overall_acc_str = (row.get("Overall Acc") or "").replace("%", "").strip()
            try:
                overall_acc = float(overall_acc_str) / 100.0
            except Exception:
                continue
            rank_str = row.get("Rank") or ""
            try:
                rank = int(rank_str)
            except Exception:
                rank = 0
            cost_str = (row.get("Total Cost ($)") or "").strip()
            try:
                cost = float(cost_str)
            except Exception:
                cost = None
            latency_str = (row.get("Latency Mean (s)") or "").strip()
            try:
                latency = float(latency_str)
            except Exception:
                latency = None
            results[model] = {
                "overall_acc": overall_acc,
                "rank": rank,
                "cost": cost,
                "latency": latency,
            }
    except Exception:
        pass

    try:
        with cache_file.open("w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
    except Exception:
        pass

    _BFCL_RESULTS_CACHE = results
    return results


def extract_bfcl_metrics(model_name: str, openrouter_id: Optional[str] = None) -> Dict[str, Tuple[float, str]]:
    """Extract BFCL function calling benchmark metrics for a model.

    Uses fuzzy match on model name or OpenRouter ID.
    Returns bfcl_v3_score (overall accuracy 0..1).
    """
    out: Dict[str, Tuple[float, str]] = {}
    data = load_bfcl_results()
    if not data:
        return out

    variants: List[str] = []
    if model_name:
        variants.append(model_name)
        variants.append(norm_name(model_name))
        base = model_name.replace("OpenAI: ", "").replace("Anthropic: ", "").replace("Google: ", "")
        base = base.replace("(FC)", "").replace("(Prompt)", "").strip()
        variants.append(base)
        variants.append(norm_name(base))
    if openrouter_id:
        variants.append(openrouter_id)
        if "/" in openrouter_id:
            variants.append(openrouter_id.split("/", 1)[1])
        variants.append(norm_name(openrouter_id))

    best_model: Optional[str] = None
    best_score = -1.0
    best_is_fc = False
    for model_key in data:
        is_fc = "(FC)" in model_key
        for v in variants:
            model_key_norm = model_key.replace("(FC)", "").replace("(Prompt)", "").strip()
            s = max(fuzzy_score(v, model_key_norm), fuzzy_score(norm_name(v), norm_name(model_key_norm)))
            if s > best_score or (s == best_score and is_fc and not best_is_fc):
                best_score = s
                best_model = model_key
                best_is_fc = is_fc

    if not best_model or best_score < 70.0:
        return out

    model_data = data[best_model]
    overall_acc = model_data.get("overall_acc")
    if overall_acc is not None:
        out["bfcl_v3_score"] = (float(overall_acc), "0..1 overall accuracy")

    return out


# -----------------------------
# Fetch: Open LLM Leaderboard JSON for a HF model repo
# -----------------------------
def fetch_openllm_results_json(hf_repo_id: str) -> Tuple[Optional[dict], Optional[str]]:
    if "/" not in hf_repo_id:
        return (None, None)
    org, name = hf_repo_id.split("/", 1)
    folder = f"{org}/{name}"
    try:
        entries = hf_list_dataset_tree(HF_OPENLLM_RESULTS, folder)
    except Exception:
        return (None, None)

    json_files = [p for p in entries if p.startswith(folder + "/results_") and p.lower().endswith(".json")]
    if not json_files:
        return (None, None)

    json_files.sort(reverse=True)
    chosen = json_files[0]
    local = hf_download_dataset_file(HF_OPENLLM_RESULTS, chosen)
    with local.open("r", encoding="utf-8") as f:
        data = json.load(f)
    url = f"https://huggingface.co/datasets/{HF_OPENLLM_RESULTS}/blob/main/{chosen}"
    return (data, url)


# -----------------------------
# HF model metadata (languages)
# -----------------------------
def fetch_hf_model_metadata(hf_repo_id: str) -> Optional[dict]:
    try:
        data = _http_get_json(f"https://huggingface.co/api/models/{hf_repo_id}", timeout=30)
        return {
            "modelId": data.get("modelId") or hf_repo_id,
            "sha": data.get("sha"),
            "tags": list(data.get("tags") or []),
            "pipeline_tag": data.get("pipeline_tag"),
            "license": data.get("license"),
            "gated": bool(data.get("gated") or False),
            "private": bool(data.get("private") or False),
            "languages": list(data.get("languages") or []),
        }
    except Exception:
        return None


# -----------------------------
# Optional: speed measurement (OpenRouter)
# -----------------------------
def measure_speed_openrouter(openrouter_id: str, max_tokens: int = 128) -> Optional[Tuple[float, dict]]:
    api_key = os.getenv("OPENROUTER_API_KEY")
    if not api_key:
        return None
    url = "https://openrouter.ai/api/v1/chat/completions"
    headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}
    payload = {
        "model": openrouter_id,
        "messages": [{"role": "user", "content": "Return a short numbered list of 5 animals."}],
        "max_tokens": max_tokens,
        "temperature": 0.2,
    }
    t0 = time.time()
    r = requests.post(url, headers=headers, json=payload, timeout=60)
    dt_s = max(1e-6, time.time() - t0)
    if r.status_code != 200:
        return None
    data = r.json()
    usage = data.get("usage") or {}
    completion_tokens = float(usage.get("completion_tokens") or 0.0)
    if completion_tokens <= 0:
        return None
    tps = completion_tokens / dt_s
    return (tps, {"duration_sec": dt_s, "usage": usage})


# -----------------------------
# Candidate building & selection
# -----------------------------
def build_candidates(query: str, conn: sqlite3.Connection, limit: int = 25) -> List[Tuple[Candidate, float]]:
    scored: List[Tuple[Candidate, float]] = []

    # DB existing
    for row in conn.execute("SELECT display_name, provider, provider_id, openrouter_id, hf_repo_id FROM models").fetchall():
        prov = row["provider"] or "unknown"
        pid = row["provider_id"] or row["display_name"]
        c = Candidate(
            source="db",
            name=row["display_name"],
            provider=prov,
            provider_id=pid,
            openrouter_id=row["openrouter_id"],
            hf_repo_id=row["hf_repo_id"],
        )
        scored.append((c, max(fuzzy_score(query, c.name), fuzzy_score(norm_name(query), norm_name(c.name)))))

    # OpenRouter
    try:
        models = fetch_openrouter_models_cached(SCRIPT_CACHE_DIR)
        for m in models:
            name = m.get("name") or m.get("id") or ""
            mid = m.get("id") or ""
            if not mid:
                continue
            c = Candidate(
                source="openrouter",
                name=name,
                provider="openrouter",
                provider_id=mid,
                openrouter_id=mid,
                hf_repo_id=None,
                extra=m,
            )
            s = max(
                fuzzy_score(query, name),
                fuzzy_score(norm_name(query), norm_name(name)),
                0.98 * fuzzy_score(query, mid),
                0.98 * fuzzy_score(norm_name(query), norm_name(mid)),
            )
            scored.append((c, s))
    except Exception:
        pass

    # Ollama (local)
    tags = fetch_ollama_tags()
    if tags and isinstance(tags.get("models"), list):
        for m in tags["models"]:
            mn = str(m.get("name") or "").strip()
            if not mn:
                continue
            disp = mn
            details = m.get("details") or {}
            if isinstance(details, dict):
                ps = details.get("parameter_size")
                ql = details.get("quantization_level")
                if ps or ql:
                    disp = f"{mn} ({ps or ''} {ql or ''})".strip()
            c = Candidate(source="ollama", name=disp, provider="ollama", provider_id=mn, extra=m)
            s = max(fuzzy_score(query, disp), fuzzy_score(norm_name(query), norm_name(disp)), 0.98 * fuzzy_score(query, mn))
            scored.append((c, s))

    # Hugging Face search
    try:
        hf_models = fetch_hf_model_search(query, limit=limit)
        for m in hf_models:
            mid = str(m.get("modelId") or "").strip()
            if not mid:
                continue
            c = Candidate(source="hf", name=mid, provider="hf", provider_id=mid, hf_repo_id=mid, extra=m)
            s = max(fuzzy_score(query, mid), fuzzy_score(norm_name(query), norm_name(mid)))
            scored.append((c, s))
    except Exception:
        pass

    # Deduplicate by (provider, provider_id)
    best: Dict[Tuple[str, str], float] = {}
    obj: Dict[Tuple[str, str], Candidate] = {}
    for c, s in scored:
        k = (c.provider, c.provider_id)
        if k not in best or s > best[k]:
            best[k] = s
            obj[k] = c

    out = [(obj[k], best[k]) for k in best.keys()]
    out.sort(key=lambda t: t[1], reverse=True)
    return out[:limit]


def prompt_select(scored: List[Tuple[Candidate, float]]) -> Candidate:
    if not scored:
        raise SystemExit("No candidates found.")
    print("\nCandidates:")
    for i, (c, s) in enumerate(scored, start=1):
        meta_bits = [f"provider={c.provider}", f"id={c.provider_id}"]
        if c.openrouter_id:
            meta_bits.append(f"openrouter_id={c.openrouter_id}")
        if c.hf_repo_id:
            meta_bits.append(f"hf_repo_id={c.hf_repo_id}")
        meta = ", ".join(meta_bits)
        print(f"  [{i:02d}] {c.name}  (match={s:.1f}; {meta})")

    sel = input("\nSelect [1..N] (default 1): ").strip()
    idx = 1
    if sel:
        with contextlib.suppress(Exception):
            idx = int(sel)
    idx = max(1, min(idx, len(scored)))
    return scored[idx - 1][0]


# -----------------------------
# Metric extraction
# -----------------------------
def extract_openrouter_metrics(openrouter_obj: dict) -> Dict[str, Tuple[float, str]]:
    out: Dict[str, Tuple[float, str]] = {}
    pricing = openrouter_obj.get("pricing") or {}

    def ffloat(x: Any) -> Optional[float]:
        try:
            return float(x)
        except Exception:
            return None

    prompt = ffloat(pricing.get("prompt"))
    completion = ffloat(pricing.get("completion"))
    request_cost = ffloat(pricing.get("request"))

    if prompt is not None and completion is not None:
        mixed_per_token = 0.5 * prompt + 0.5 * completion
        out["cost_usd_per_1m_mixed"] = (mixed_per_token * 1_000_000.0, "USD/1M tokens (50/50 prompt+completion)")
        out["openrouter_prompt_usd_per_token"] = (prompt, "USD/token")
        out["openrouter_completion_usd_per_token"] = (completion, "USD/token")
    if request_cost is not None:
        out["openrouter_request_usd"] = (request_cost, "USD/request")

    supported = set(openrouter_obj.get("supported_parameters") or [])
    out["openrouter_tools_supported"] = (1.0 if ("tools" in supported) else 0.0, "bool")
    out["openrouter_structured_outputs_supported"] = (1.0 if ("structured_outputs" in supported) else 0.0, "bool")

    top_provider = openrouter_obj.get("top_provider") or {}
    is_moderated = bool(top_provider.get("is_moderated")) if isinstance(top_provider, dict) else False
    out["openrouter_is_moderated"] = (1.0 if is_moderated else 0.0, "bool")

    ctx = openrouter_obj.get("context_length")
    if isinstance(ctx, (int, float)):
        out["context_length_tokens"] = (float(ctx), "tokens")

    return out


def extract_arena_metrics(arena_row: dict) -> Dict[str, Tuple[float, str]]:
    out: Dict[str, Tuple[float, str]] = {}

    def to_float(x: Any) -> Optional[float]:
        try:
            if isinstance(x, str):
                x = x.replace(",", "")
            return float(x)
        except Exception:
            return None

    s = to_float(arena_row.get("Arena Score"))
    if s is not None:
        out["arena_score"] = (s, "Elo-ish")
    v = to_float(arena_row.get("Votes"))
    if v is not None:
        out["arena_votes"] = (v, "count")
    return out


def extract_bigcodebench_metrics(df: Any, model_name: str) -> Dict[str, Tuple[float, str]]:
    out: Dict[str, Tuple[float, str]] = {}
    if "model" not in df.columns:
        return out

    candidates = df["model"].dropna().astype(str).unique().tolist()
    best_name = None
    best_score = -1.0
    for n in candidates:
        s = max(fuzzy_score(model_name, n), fuzzy_score(norm_name(model_name), norm_name(n)))
        if s > best_score:
            best_score = s
            best_name = n

    if best_name is None or best_score < 70.0:
        return out

    row = df[df["model"].astype(str) == best_name].head(1)
    if row.empty:
        return out

    r0 = row.iloc[0].to_dict()
    for k, key_out in [("complete", "bigcodebench_complete"), ("instruct", "bigcodebench_instruct")]:
        if k in r0 and r0[k] is not None and not (isinstance(r0[k], float) and math.isnan(r0[k])):
            with contextlib.suppress(Exception):
                out[key_out] = (float(r0[k]), "score (0..100)")
    return out


def extract_openllm_metrics(openllm_json: dict) -> Dict[str, Tuple[float, str]]:
    out: Dict[str, Tuple[float, str]] = {}
    results = (openllm_json or {}).get("results") or {}

    def first_metric(task: str, keys: List[str]) -> Optional[float]:
        t = results.get(task) or {}
        if not isinstance(t, dict):
            return None
        for k in keys:
            v = t.get(k)
            if v is None:
                continue
            with contextlib.suppress(Exception):
                return float(v)
        return None

    bbh = first_metric("leaderboard_bbh", ["acc_norm,none", "acc_norm"])
    if bbh is not None:
        out["openllm_bbh_acc_norm"] = (bbh, "0..1 acc_norm")

    gpqa = first_metric("leaderboard_gpqa", ["acc_norm,none", "acc_norm"])
    if gpqa is not None:
        out["openllm_gpqa_acc_norm"] = (gpqa, "0..1 acc_norm")

    mh = first_metric("leaderboard_math_hard", ["exact_match,none", "exact_match"])
    if mh is not None:
        out["openllm_math_hard_exact_match"] = (mh, "0..1 exact_match")

    # Multilinguality candidates (task naming differs across OLL revisions).
    for task in ["leaderboard_mgsm", "leaderboard_mgsm_en", "leaderboard_mgsm_multilingual"]:
        mgsm = first_metric(task, ["exact_match,none", "exact_match", "acc,none", "acc"])
        if mgsm is not None:
            out["openllm_mgsm_exact_match"] = (mgsm, "0..1")
            break

    for task in ["leaderboard_xnli", "leaderboard_xnli_en"]:
        xnli = first_metric(task, ["acc,none", "acc", "accuracy,none", "accuracy"])
        if xnli is not None:
            out["openllm_xnli_acc"] = (xnli, "0..1")
            break

    # Factuality candidates.
    for task in ["leaderboard_truthfulqa", "leaderboard_truthfulqa_mc2", "leaderboard_truthfulqa_generation"]:
        truthful = first_metric(task, ["mc2,none", "mc2", "acc,none", "acc"])
        if truthful is not None:
            out["openllm_truthfulqa_mc2"] = (truthful, "0..1")
            break

    return out


# -----------------------------
# Normalization & scoring
# -----------------------------
def get_metric_values(conn: sqlite3.Connection, key: str) -> List[float]:
    rows = conn.execute("SELECT value FROM raw_metrics WHERE key=?", (key,)).fetchall()
    return [float(r["value"]) for r in rows]


def fixed_scale_params(metric_spec: dict) -> Optional[Tuple[float, float]]:
    """
    If a metric has an explicit scale, normalize against that instead of cohort min/max.
    - "binary": 0..1
    - "unit":   0..1
    - {"min": x, "max": y}: fixed numeric range
    """
    scale = metric_spec.get("scale")
    transform = metric_spec.get("transform")
    if scale in ("binary", "unit"):
        mn, mx = (0.0, 1.0)
        if transform:
            return (transform_value(mn, transform), transform_value(mx, transform))
        return (mn, mx)
    if isinstance(scale, dict) and "min" in scale and "max" in scale:
        mn, mx = (float(scale["min"]), float(scale["max"]))
        if transform:
            return (transform_value(mn, transform), transform_value(mx, transform))
        return (mn, mx)
    return None


_ARENA_ELO_MIN_MAX: Optional[Tuple[float, float]] = None


def arena_elo_min_max() -> Optional[Tuple[float, float]]:
    """Compute min/max Elo from the Arena dataset.

    Used as a benchmark-derived fixed scale for `arena_score`.
    """
    global _ARENA_ELO_MIN_MAX
    if _ARENA_ELO_MIN_MAX is not None:
        return _ARENA_ELO_MIN_MAX
    try:
        rows = load_arena_csv()
        vals: List[float] = []
        for r in rows:
            try:
                v = float(str(r.get("Arena Score") or "").replace(",", ""))
                if math.isfinite(v):
                    vals.append(v)
            except Exception:
                continue
        if len(vals) < 2:
            _ARENA_ELO_MIN_MAX = None
            return None
        _ARENA_ELO_MIN_MAX = (min(vals), max(vals))
        return _ARENA_ELO_MIN_MAX
    except Exception:
        _ARENA_ELO_MIN_MAX = None
        return None


_BIGCODEBENCH_MIN_MAX: Optional[Dict[str, Tuple[float, float]]] = None


def bigcodebench_score_min_max() -> Optional[Dict[str, Tuple[float, float]]]:
    """Compute min/max for BigCodeBench scores from the results dataset.

    Returns a mapping for keys: bigcodebench_instruct, bigcodebench_complete.
    """
    global _BIGCODEBENCH_MIN_MAX
    if _BIGCODEBENCH_MIN_MAX is not None:
        return _BIGCODEBENCH_MIN_MAX
    if pd is None:
        _BIGCODEBENCH_MIN_MAX = None
        return None
    try:
        df = load_bigcodebench_results()
        out: Dict[str, Tuple[float, float]] = {}
        for col, key_out in [("instruct", "bigcodebench_instruct"), ("complete", "bigcodebench_complete")]:
            if col not in df.columns:
                continue
            s = df[col]
            # numeric conversion + drop NaN/inf
            s = pd.to_numeric(s, errors="coerce")  # type: ignore[union-attr]
            s = s.dropna()  # type: ignore[union-attr]
            if len(s) < 2:  # type: ignore[arg-type]
                continue
            mn = float(s.min())  # type: ignore[union-attr]
            mx = float(s.max())  # type: ignore[union-attr]
            if math.isfinite(mn) and math.isfinite(mx) and abs(mx - mn) > 1e-12:
                out[key_out] = (mn, mx)
        _BIGCODEBENCH_MIN_MAX = out or None
        return _BIGCODEBENCH_MIN_MAX
    except Exception:
        _BIGCODEBENCH_MIN_MAX = None
        return None


def benchmark_scale_override(metric_spec: dict) -> Optional[Tuple[float, float]]:
    """Provide fixed scales derived from benchmark definitions/datasets.

    This is intentionally *not* cohort-based.
    """
    key = str(metric_spec.get("key") or "")
    transform = metric_spec.get("transform")

    if key == "arena_score":
        mm = arena_elo_min_max()
        if mm is None:
            return None
        mn, mx = mm
        if transform:
            return (transform_value(mn, transform), transform_value(mx, transform))
        return (mn, mx)

    if key in ("bigcodebench_instruct", "bigcodebench_complete"):
        mm = bigcodebench_score_min_max() or {}
        if key not in mm:
            return None
        mn, mx = mm[key]
        if transform:
            return (transform_value(mn, transform), transform_value(mx, transform))
        return (mn, mx)

    return None


def compute_norm_params(conn: sqlite3.Connection, metric_spec: dict) -> Optional[Tuple[float, float]]:
    key = str(metric_spec.get("key") or "")

    # Only cost is cohort-relative by default.
    if key != "cost_usd_per_1m_mixed":
        override = benchmark_scale_override(metric_spec)
        if override is not None:
            return override
        fixed = fixed_scale_params(metric_spec)
        if fixed is not None:
            return fixed

    transform = metric_spec.get("transform")
    vals = get_metric_values(conn, key)
    if len(vals) < 2:
        return None
    tv = [transform_value(v, transform) for v in vals]
    mn, mx = min(tv), max(tv)
    if abs(mx - mn) < 1e-12:
        return None
    return (mn, mx)


def normalize_value(x: float, metric_spec: dict, params: Tuple[float, float]) -> float:
    transform = metric_spec.get("transform")
    better = metric_spec.get("better", "higher")
    mn, mx = params
    tx = transform_value(x, transform)

    if abs(mx - mn) < 1e-12:
        return 0.5  # no ranking signal

    n = (tx - mn) / (mx - mn)
    n = clamp01(n)
    if better == "lower":
        n = 1.0 - n
    return clamp01(n)


def model_metric(conn: sqlite3.Connection, model_id: int, key: str) -> Optional[float]:
    row = conn.execute("SELECT value FROM raw_metrics WHERE model_id=? AND key=?", (model_id, key)).fetchone()
    if row is None:
        return None
    return float(row["value"])


def score_model_category(conn: sqlite3.Connection, model_id: int, standard: dict, norm_params: Dict[str, Tuple[float, float]]) -> Dict[str, Tuple[float, float, dict]]:
    """
    Returns: {category: (score, confidence, details_json_obj)}
    """
    out: Dict[str, Tuple[float, float, dict]] = {}
    fallback_confidence_multiplier = float(standard.get("fallback_confidence_multiplier", 0.33))
    for cat, cfg in standard["categories"].items():
        metrics = cfg.get("metrics", [])
        fallbacks = cfg.get("fallbacks", [])

        def compute_from_specs(specs: List[dict]) -> Tuple[Optional[float], float, dict]:
            used = []
            total_w = sum(float(s.get("weight", 1.0)) for s in specs) or 1.0
            accum = 0.0
            used_w = 0.0
            for spec in specs:
                key = spec["key"]
                w = float(spec.get("weight", 1.0))
                raw = model_metric(conn, model_id, key)
                if raw is None:
                    continue
                if key not in norm_params:
                    norm = 0.5
                    params_used = None
                else:
                    norm = normalize_value(raw, spec, norm_params[key])
                    params_used = {
                        "min": norm_params[key][0],
                        "max": norm_params[key][1],
                        "transform": spec.get("transform"),
                        "better": spec.get("better"),
                        "scale": spec.get("scale"),
                    }
                accum += norm * w
                used_w += w
                used.append({"key": key, "raw": raw, "norm": norm, "weight": w, "norm_params": params_used})
            if used_w <= 0:
                return (None, 0.0, {"used": [], "note": "no metrics available"})
            score = accum / used_w
            confidence = used_w / total_w
            return (score, clamp01(confidence), {"used": used})

        score, conf, details = compute_from_specs(metrics)
        used_fallback = False
        if score is None:
            score, conf, details = compute_from_specs(fallbacks)
            used_fallback = True

        if used_fallback and conf > 0:
            conf = clamp01(conf * fallback_confidence_multiplier)

        if score is None:
            score = 0.5
            conf = 0.0
            details = {"used": [], "note": "no metrics available, defaulted to 0.5"}
        details["used_fallback"] = used_fallback
        out[cat] = (clamp01(float(score)), clamp01(float(conf)), details)
    return out


def rescore_all(conn: sqlite3.Connection, standard: dict) -> int:
    standard_id = get_or_create_standard(conn, standard)

    needed_keys: set[str] = set()
    metric_specs_by_key: Dict[str, dict] = {}
    for cfg in standard["categories"].values():
        for spec in (cfg.get("metrics", []) + cfg.get("fallbacks", [])):
            needed_keys.add(spec["key"])
            metric_specs_by_key[spec["key"]] = spec

    norm_params: Dict[str, Tuple[float, float]] = {}
    for key in needed_keys:
        spec = metric_specs_by_key[key]
        params = compute_norm_params(conn, spec)
        if params is not None:
            norm_params[key] = params

    models = conn.execute("SELECT id FROM models").fetchall()
    updated = 0
    for r in models:
        mid = int(r["id"])
        cat_scores = score_model_category(conn, mid, standard, norm_params)
        for cat, (score, conf, details) in cat_scores.items():
            conn.execute(
                """
                INSERT INTO scores(model_id, standard_id, category, score, confidence, details_json, computed_at)
                VALUES(?,?,?,?,?,?,?)
                ON CONFLICT(model_id, standard_id, category) DO UPDATE SET
                  score=excluded.score,
                  confidence=excluded.confidence,
                  details_json=excluded.details_json,
                  computed_at=excluded.computed_at
                """,
                (mid, standard_id, cat, float(score), float(conf), json.dumps(details), now_iso()),
            )
            updated += 1
    conn.commit()
    return updated


# -----------------------------
# Main: evaluate a selected model
# -----------------------------
def evaluate_and_store_metrics(conn: sqlite3.Connection, cand: Candidate, measure_speed: bool) -> int:
    display_name = cand.name.strip()
    openrouter_id = cand.openrouter_id
    hf_repo_id = cand.hf_repo_id

    # IMPORTANT: do NOT guess HF repo IDs from OpenRouter IDs (often wrong).
    # If you want HF metrics, select a HF candidate (provider=hf).

    model_id = upsert_model(
        conn,
        display_name=display_name,
        provider=cand.provider,
        provider_id=cand.provider_id,
        openrouter_id=openrouter_id,
        hf_repo_id=hf_repo_id,
    )

    # If we have a clearly-invalid HF repo ID (common when it was copied from OpenRouter IDs), clear it.
    if hf_repo_id and openrouter_id and hf_repo_id == openrouter_id:
        if fetch_hf_model_metadata(hf_repo_id) is None:
            hf_repo_id = None
            set_model_hf_repo_id(conn, model_id, None)

    # Auto-link a HF repo for OpenRouter models (only when confident + OpenLLM results exist).
    if (not hf_repo_id) and openrouter_id and cand.provider == "openrouter":
        best_id, best_s = _best_hf_repo_for_openllm(display_name, openrouter_id)
        if best_id:
            hf_repo_id = best_id
            set_model_hf_repo_id(conn, model_id, best_id)
            sid = upsert_source(
                conn,
                "hf_autolink_for_openllm",
                HF_MODELS_SEARCH_URL,
                {"openrouter_id": openrouter_id, "display_name": display_name, "hf_repo_id": best_id, "match": best_s},
            )
            upsert_link(conn, model_id, "hf_model_autolink", f"https://huggingface.co/{best_id}", best_id, sid)
        else:
            # Still attempt metadata linkage (languages), even if OpenLLM results aren't available.
            meta_id, meta_s = _best_hf_repo_for_metadata(display_name, openrouter_id)
            if meta_id:
                hf_repo_id = meta_id
                set_model_hf_repo_id(conn, model_id, meta_id)
                sid = upsert_source(
                    conn,
                    "hf_autolink_metadata",
                    HF_MODELS_SEARCH_URL,
                    {"openrouter_id": openrouter_id, "display_name": display_name, "hf_repo_id": meta_id, "match": meta_s},
                )
                upsert_link(conn, model_id, "hf_model_autolink", f"https://huggingface.co/{meta_id}", meta_id, sid)

    # ---- OpenRouter metrics
    openrouter_obj = cand.extra if cand.source == "openrouter" else None
    if openrouter_id and openrouter_obj is None:
        try:
            all_models = fetch_openrouter_models_cached(SCRIPT_CACHE_DIR)
            for m in all_models:
                if m.get("id") == openrouter_id:
                    openrouter_obj = m
                    break
        except Exception:
            openrouter_obj = None

    if openrouter_obj is not None:
        sid = upsert_source(conn, "openrouter_models_api", OPENROUTER_MODELS_URL, openrouter_obj)
        upsert_link(conn, model_id, "openrouter_model", f"https://openrouter.ai/models/{openrouter_id}", display_name, sid)
        for key, (val, unit) in extract_openrouter_metrics(openrouter_obj).items():
            upsert_metric(conn, model_id, key, float(val), unit, sid)

    # ---- Ollama (local)
    if cand.provider == "ollama":
        sid = upsert_source(conn, "ollama_tags", f"{OLLAMA_HOST}/api/tags", {"host": OLLAMA_HOST, "model": cand.provider_id})
        upsert_link(conn, model_id, "ollama_model", f"ollama:{cand.provider_id}", cand.provider_id, sid)
        upsert_metric(conn, model_id, "cost_is_local_proxy", 1.0, "bool", sid)

    # ---- Arena metrics (match with normalized variants)
    try:
        arena_rows = load_arena_csv()
        best_row = None
        best_s = -1.0
        variants = name_variants(display_name, openrouter_id, hf_repo_id)
        for r in arena_rows:
            n = r.get("Model") or ""
            if not n:
                continue
            sn = max(fuzzy_score(v, n) for v in variants)
            sn = max(sn, max(fuzzy_score(norm_name(v), norm_name(n)) for v in variants))
            if sn > best_s:
                best_s = sn
                best_row = r
        if best_row is not None and best_s >= 75.0:
            sid = upsert_source(
                conn,
                "hf_chatbot_arena_elo",
                f"https://huggingface.co/datasets/{HF_ARENA_DATASET}",
                {"matched_model": best_row.get("Model"), "match": best_s},
            )
            upsert_link(conn, model_id, "arena_dataset", f"https://huggingface.co/datasets/{HF_ARENA_DATASET}", "Chatbot Arena Elo dataset", sid)
            for key, (val, unit) in extract_arena_metrics(best_row).items():
                upsert_metric(conn, model_id, key, float(val), unit, sid)
    except Exception:
        pass

    # ---- BigCodeBench metrics
    try:
        df = load_bigcodebench_results()
        metrics = extract_bigcodebench_metrics(df, display_name)
        if metrics:
            sid = upsert_source(conn, "hf_bigcodebench_results", f"https://huggingface.co/datasets/{HF_BIGCODEBENCH_RESULTS}", {"matched": True})
            upsert_link(conn, model_id, "bigcodebench_dataset", f"https://huggingface.co/datasets/{HF_BIGCODEBENCH_RESULTS}", "BigCodeBench results dataset", sid)
            for key, (val, unit) in metrics.items():
                upsert_metric(conn, model_id, key, float(val), unit, sid)
    except Exception:
        pass

    # ---- MMMLU (Multilingual MMLU) metrics from OpenAI simple-evals
    try:
        mmmlu_metrics = extract_mmmlu_metrics(display_name, openrouter_id)
        if mmmlu_metrics:
            sid = upsert_source(conn, "openai_mmmlu", MMMLU_RESULTS_URL, {"matched": True})
            upsert_link(conn, model_id, "mmmlu_results", MMMLU_RESULTS_URL, "OpenAI MMMLU benchmark results", sid)
            for key, (val, unit) in mmmlu_metrics.items():
                upsert_metric(conn, model_id, key, float(val), unit, sid)
    except Exception:
        pass

    # ---- BFCL (Berkeley Function Calling Leaderboard) metrics
    try:
        bfcl_metrics = extract_bfcl_metrics(display_name, openrouter_id)
        if bfcl_metrics:
            sid = upsert_source(conn, "bfcl_results", BFCL_RESULTS_URL, {"matched": True})
            upsert_link(conn, model_id, "bfcl_results", "https://gorilla.cs.berkeley.edu/leaderboard.html", "Berkeley Function Calling Leaderboard", sid)
            for key, (val, unit) in bfcl_metrics.items():
                upsert_metric(conn, model_id, key, float(val), unit, sid)
    except Exception:
        pass

    # ---- HF metadata + Open LLM Leaderboard (only if a real HF repo is selected)
    if hf_repo_id:
        meta = fetch_hf_model_metadata(hf_repo_id)
        if meta is not None:
            sid = upsert_source(conn, "hf_model_metadata", f"https://huggingface.co/{hf_repo_id}", meta)
            upsert_link(conn, model_id, "hf_model", f"https://huggingface.co/{hf_repo_id}", hf_repo_id, sid)
            langs = meta.get("languages") or []
            lang_count = float(len(langs)) if isinstance(langs, list) else 0.0
            upsert_metric(conn, model_id, "hf_language_count", lang_count, "count", sid)

        openllm_json, openllm_url = fetch_openllm_results_json(hf_repo_id)
        if openllm_json is not None:
            sid = upsert_source(conn, "hf_openllm_results", openllm_url or f"https://huggingface.co/datasets/{HF_OPENLLM_RESULTS}", {"hf_repo_id": hf_repo_id})
            upsert_link(conn, model_id, "openllm_results", openllm_url or f"https://huggingface.co/datasets/{HF_OPENLLM_RESULTS}", "Open LLM Leaderboard results", sid)
            for key, (val, unit) in extract_openllm_metrics(openllm_json).items():
                upsert_metric(conn, model_id, key, float(val), unit, sid)

    # ---- COMPL-AI metrics (match against name variants; store if strong match)
    script_dir = Path(__file__).resolve().parent
    complai_metrics, complai_links, matched_name, match_score = complai_metrics_for_any(script_dir, display_name, openrouter_id, hf_repo_id)
    if complai_metrics:
        sid = upsert_source(
            conn,
            "complai_space_results",
            f"https://huggingface.co/spaces/{COMPLAI_BOARD_SPACE}",
            {"matched_model_name": matched_name, "match": match_score, "variants": name_variants(display_name, openrouter_id, hf_repo_id)},
        )
        for url in complai_links:
            upsert_link(conn, model_id, "complai_report", url, "COMPL-AI evaluation", sid)
        for k, v in complai_metrics.items():
            upsert_metric(conn, model_id, k, float(v), "0..1", sid)

    # ---- Optional speed measurement
    if measure_speed:
        if openrouter_id:
            measured = measure_speed_openrouter(openrouter_id)
            if measured is not None:
                tps, blob = measured
                sid = upsert_source(conn, "measured_speed_openrouter", "https://openrouter.ai/api/v1/chat/completions", blob)
                upsert_metric(conn, model_id, "measured_tokens_per_sec", float(tps), "tokens/sec", sid)

        if cand.provider == "ollama":
            measured = measure_speed_ollama(cand.provider_id)
            if measured is not None:
                tps, blob = measured
                sid = upsert_source(conn, "measured_speed_ollama", f"{OLLAMA_HOST}/api/generate", blob)
                upsert_metric(conn, model_id, "ollama_measured_tokens_per_sec", float(tps), "tokens/sec", sid)

    conn.commit()

    return model_id


def evaluate_and_store(conn: sqlite3.Connection, cand: Candidate, standard: dict, measure_speed: bool) -> int:
    model_id = evaluate_and_store_metrics(conn, cand, measure_speed=measure_speed)
    changed = rescore_all(conn, standard)
    print(f"\nStored model_id={model_id}. Re-scored {changed} category-scores across the DB.\n")
    return model_id


def fetch_model_scores(conn: sqlite3.Connection, model_id: int, standard_id: int) -> Dict[str, dict]:
    out: Dict[str, dict] = {}
    rows = conn.execute(
        "SELECT category, score, confidence, details_json, computed_at FROM scores WHERE model_id=? AND standard_id=? ORDER BY category",
        (int(model_id), int(standard_id)),
    ).fetchall()
    for r in rows:
        try:
            details = json.loads(r["details_json"]) if r["details_json"] else {}
        except Exception:
            details = {}
        out[str(r["category"])] = {
            "score": float(r["score"]),
            "confidence": float(r["confidence"]),
            "used_fallback": bool(details.get("used_fallback")),
            "computed_at": r["computed_at"],
            "details": details,
        }
    return out


def print_model_report(conn: sqlite3.Connection, model_id: int, show_details: bool = True) -> None:
    row = conn.execute("SELECT * FROM models WHERE id=?", (model_id,)).fetchone()
    if row is None:
        print("Model not found.")
        return

    print(f"\nModel #{row['id']}: {row['display_name']}")
    if row["provider"] and row["provider_id"]:
        print(f"  Provider:  {row['provider']} ({row['provider_id']})")
    if row["openrouter_id"]:
        print(f"  OpenRouter: {row['openrouter_id']}")
    if row["hf_repo_id"]:
        print(f"  HF repo:    {row['hf_repo_id']}")

    print("\nRaw metrics:")
    metrics = conn.execute("SELECT key, value, unit, retrieved_at FROM raw_metrics WHERE model_id=? ORDER BY key", (model_id,)).fetchall()
    for m in metrics:
        unit = m["unit"] or ""
        print(f"  - {m['key']}: {m['value']:.6g} {unit} (as of {m['retrieved_at']})")

    std = conn.execute("SELECT id, name FROM standards ORDER BY id DESC LIMIT 1").fetchone()
    if std is None:
        print("\nNo scores yet.")
        return

    print(f"\nScores (standard={std['name']}):")
    scores = conn.execute(
        "SELECT category, score, confidence, details_json, computed_at FROM scores WHERE model_id=? AND standard_id=? ORDER BY category",
        (model_id, std["id"]),
    ).fetchall()
    for s in scores:
        print(f"  - {s['category']}: {s['score']:.3f} (confidence={s['confidence']:.2f}, computed_at={s['computed_at']})")
        if show_details:
            try:
                details = json.loads(s["details_json"])
                used = details.get("used") or []
                if not used:
                    print("      used: (none)")
                else:
                    keys = ", ".join(f"{u['key']}={u['raw']:.4g}" for u in used)
                    print(f"      used: {keys}{'  [fallback]' if details.get('used_fallback') else ''}")
            except Exception:
                pass

    print("\nLinks:")
    links: list[dict] = conn.execute("SELECT kind, url, title FROM links WHERE model_id=? ORDER BY kind", (model_id,)).fetchall()
    for link in links:
        title = link.get("title") or ""
        print(f"  - {link['kind']}: {link['url']} {('— ' + title) if title else ''}")
    print("")


def resolve_model_id(conn: sqlite3.Connection, name_or_id: str) -> Optional[int]:
    with contextlib.suppress(Exception):
        mid = int(name_or_id)
        row = conn.execute("SELECT id FROM models WHERE id=?", (mid,)).fetchone()
        if row:
            return mid
    rows = conn.execute("SELECT id, display_name FROM models").fetchall()
    best = None
    best_s = -1.0
    for r in rows:
        s = max(fuzzy_score(name_or_id, r["display_name"]), fuzzy_score(norm_name(name_or_id), norm_name(r["display_name"])))
        if s > best_s:
            best_s = s
            best = int(r["id"])
    if best is None:
        return None
    return best if best_s >= 70.0 else None


def ingest_bfcl_results(conn: sqlite3.Connection, results_file: Path) -> int:
    """
    Ingest BFCL scores for models.

    Supported formats:
    - CSV with headers: model, score
    - JSON list of objects containing {model, score} (or {name, score})

    Stores metric: bfcl_v3_score (0..1).
    """
    if not results_file.exists():
        raise SystemExit(f"BFCL results file not found: {results_file}")

    rows: List[Tuple[str, float]] = []
    if results_file.suffix.lower() == ".csv":
        with results_file.open("r", encoding="utf-8") as f:
            reader = csv.DictReader(f)
            for r in reader:
                m = (r.get("model") or r.get("name") or "").strip()
                s = (r.get("score") or r.get("bfcl_v3_score") or r.get("bfcl_score") or "").strip()
                if not m or not s:
                    continue
                with contextlib.suppress(Exception):
                    rows.append((m, float(s)))
    elif results_file.suffix.lower() == ".json":
        data = json.loads(results_file.read_text("utf-8"))
        items = data
        if isinstance(data, dict):
            items = data.get("models") or data.get("results") or []
        if isinstance(items, list):
            for it in items:
                if not isinstance(it, dict):
                    continue
                m = (it.get("model") or it.get("name") or it.get("model_name") or "").strip()
                v = it.get("score")
                if v is None:
                    v = it.get("bfcl_v3_score") or it.get("bfcl_score")
                if not m or v is None:
                    continue
                with contextlib.suppress(Exception):
                    rows.append((m, float(v)))
    else:
        raise SystemExit("Unsupported BFCL results format. Use .csv or .json")

    if not rows:
        return 0

    sid = upsert_source(
        conn,
        "bfcl_results_ingest",
        str(results_file.resolve()),
        {"benchmark": "BFCL", "version": "v3", "count": len(rows)},
    )

    updated = 0
    for model_name, score in rows:
        s = clamp01(float(score))

        mid = resolve_model_id(conn, model_name)
        if mid is None:
            mid = upsert_model(
                conn,
                display_name=model_name,
                provider="unknown",
                provider_id=model_name,
                openrouter_id=None,
                hf_repo_id=None,
            )

        upsert_metric(conn, mid, "bfcl_v3_score", s, "0..1", sid)
        updated += 1

    conn.commit()
    return updated


# -----------------------------
# CLI
# -----------------------------
def main() -> None:
    ap = argparse.ArgumentParser(description="Fuzzy model rater -> SQLite -> normalized 0..1 scores")
    ap.add_argument("--db", default=None, help="Path to sqlite db (default: next to script).")
    ap.add_argument("--limit", type=int, default=25, help="Number of fuzzy candidates to show.")
    ap.add_argument("--measure-speed", action="store_true", help="Measure tokens/sec (OpenRouter and/or Ollama).")

    sub = ap.add_subparsers(dest="cmd", required=True)

    s_search = sub.add_parser("search", help="Search candidates only")
    s_search.add_argument("query", help="Model query")

    s_eval = sub.add_parser("eval", help="Search -> select -> evaluate -> store -> rescore all")
    s_eval.add_argument("query", help="Model query")
    s_eval.add_argument("--measure-speed", action="store_true", help="Measure tokens/sec (OpenRouter and/or Ollama).")

    s_batch = sub.add_parser("batch-eval", help="Evaluate multiple models (non-interactive) and export JSON")
    s_batch.add_argument("models", nargs="*", help="Model queries (space-separated).")
    s_batch.add_argument("--file", default=None, help="Path to a newline-delimited file of model queries.")
    s_batch.add_argument("--output", default="model_scores.json", help="Where to write JSON output.")
    s_batch.add_argument("--min-match", type=float, default=70.0, help="Warn if top match is below this score.")
    s_batch.add_argument("--skip-low-match", action="store_true", help="Skip evaluation when match < --min-match.")
    s_batch.add_argument("--measure-speed", action="store_true", help="Measure tokens/sec (OpenRouter and/or Ollama).")

    sub.add_parser("rescore", help="Recompute normalized scores for all models")

    s_bfcl = sub.add_parser("ingest-bfcl", help="Ingest BFCL (function calling) benchmark scores from a file")
    s_bfcl.add_argument("file", help="Path to BFCL results (.csv or .json with model+score)")

    s_show = sub.add_parser("show", help="Show stored metrics/scores/links for a model")
    s_show.add_argument("model", help="Model id or name (fuzzy)")
    s_show.add_argument("--no-details", action="store_true", help="Hide per-category metric usage details")

    args = ap.parse_args()

    script_path = Path(__file__).resolve()
    db_path = Path(args.db).resolve() if args.db else db_path_for_script(script_path)

    conn = connect_db(db_path)
    init_db(conn)

    standard = DEFAULT_STANDARD

    if args.cmd == "search":
        scored = build_candidates(args.query, conn, limit=args.limit)
        if not scored:
            print("No matches.")
            return
        for c, s in scored:
            print(f"{s:6.1f}  {c.name}   (provider={c.provider}, id={c.provider_id}, source={c.source})")
        return

    if args.cmd == "eval":
        scored = build_candidates(args.query, conn, limit=args.limit)
        cand = prompt_select(scored)
        mid = evaluate_and_store(conn, cand, standard, measure_speed=bool(args.measure_speed))
        mid = resolve_model_id(conn, cand.name)
        if mid is not None:
            print_model_report(conn, mid, show_details=True)
        return

    if args.cmd == "batch-eval":
        queries: List[str] = []
        if args.file:
            p = Path(args.file).expanduser().resolve()
            if not p.exists():
                raise SystemExit(f"Model list file not found: {p}")
            for line in p.read_text("utf-8").splitlines():
                q = line.strip()
                if q and not q.startswith("#"):
                    queries.append(q)
        for q in (args.models or []):
            qq = str(q).strip()
            if qq:
                queries.append(qq)

        # de-dupe preserving order
        seen_q: set[str] = set()
        uniq: List[str] = []
        for q in queries:
            if q not in seen_q:
                seen_q.add(q)
                uniq.append(q)
        queries = uniq

        if not queries:
            raise SystemExit("No models provided. Use positional args or --file.")

        results: List[dict] = []
        evaluated_model_ids: List[int] = []
        for q in queries:
            scored = build_candidates(q, conn, limit=args.limit)
            if not scored:
                results.append({"query": q, "status": "no_match"})
                continue
            cand, match = scored[0]
            item: dict = {
                "query": q,
                "status": "ok",
                "selected": {
                    "name": cand.name,
                    "provider": cand.provider,
                    "provider_id": cand.provider_id,
                    "openrouter_id": cand.openrouter_id,
                    "hf_repo_id": cand.hf_repo_id,
                    "source": cand.source,
                    "match": float(match),
                },
                "warnings": [],
            }
            if float(match) < float(args.min_match):
                item["warnings"].append(f"low_match<{args.min_match}: {match:.1f}")
                if bool(args.skip_low_match):
                    item["status"] = "skipped_low_match"
                    results.append(item)
                    continue

            try:
                model_id = evaluate_and_store_metrics(conn, cand, measure_speed=bool(args.measure_speed))
                evaluated_model_ids.append(int(model_id))
                item["model_id"] = int(model_id)
            except Exception as e:
                item["status"] = "error"
                item["error"] = str(e)
            results.append(item)

        # Single rescore pass for the cohort.
        standard_id = get_or_create_standard(conn, standard)
        changed = rescore_all(conn, standard)

        # Attach scores
        for item in results:
            mid = item.get("model_id")
            if not isinstance(mid, int):
                continue
            item["scores"] = fetch_model_scores(conn, mid, standard_id)

        out_path = Path(str(args.output)).expanduser().resolve()
        payload = {
            "generated_at": now_iso(),
            "standard": standard,
            "db_path": str(db_path),
            "rescored_count": int(changed),
            "models": results,
        }
        out_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), "utf-8")
        print(f"Wrote {out_path}")
        return

    if args.cmd == "rescore":
        n = rescore_all(conn, standard)
        print(f"Re-scored {n} category-scores across DB.")
        return

    if args.cmd == "ingest-bfcl":
        n = ingest_bfcl_results(conn, Path(args.file).expanduser().resolve())
        rescored = rescore_all(conn, standard)
        print(f"Ingested {n} BFCL scores. Re-scored {rescored} category-scores across DB.")
        return

    if args.cmd == "show":
        mid = resolve_model_id(conn, args.model)
        if mid is None:
            print("No model found.")
            return
        print_model_report(conn, mid, show_details=not bool(args.no_details))
        return


if __name__ == "__main__":
    main()
