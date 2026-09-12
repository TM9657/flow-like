# flow-bits

A command line tool for bits, covering both halves of the system: the local
store the desktop app downloads into, and the bit catalog a hub serves.

```bash
cargo run -p bit-cli -- <command>     # or: cargo bits <command>
```

Global options: `--store <DIR>` for the local store, `--hub <DOMAIN>` for the
catalog, `--token <PAT>` for catalog writes. Without them the tool reads the
desktop app's `global-settings.json`, falling back to the platform defaults and
`api.flow-like.com`.

## Local store

| Command | What it does |
| --- | --- |
| `status` | Store path, artifact count, size, problem count |
| `list [--resolve] [--json]` | Artifacts on disk, named against the catalog with `--resolve` |
| `search <query> [--type Llm]` | Search the catalog, marking what is installed |
| `info <bit-id>` | A bit, its dependency pack and which artifacts are present |
| `install <bit-id>…` | Download a bit and its dependencies |
| `remove <bit-id\|hash>… [--yes]` | Delete store directories, printing the plan without `--yes` |
| `doctor [--fix]` | Empty directories, superseded partials and zero byte artifacts |

Downloads run through the runtime's own path, so resume, size validation and
blake3 verification behave exactly as they do in the desktop app.

## Hub catalog

| Command | What it does |
| --- | --- |
| `hub whoami` | The user the token acts as, and whether it may write bits |
| `hub pull <bit-id> [--with-dependencies] [--out FILE]` | Turn catalog entries into a spec |
| `hub push <spec> [--dry-run]` | Write a spec: dependencies first, then metadata |
| `hub delete <bit-id>… [--yes]` | Remove entries from the catalog |

Writes need a personal access token whose user holds `WriteBits` (or `Admin`),
passed as `--token` or `FLOW_LIKE_PAT`. Mint one under settings, or through
`PUT /user/pat`.

### Spec format

One file describes a bit and everything it depends on. An entry carries the
fields of a `Bit` plus two authoring conveniences: `ref`, a local alias other
entries point at as `@alias`, and `meta`, the per-language metadata the hub
stores through its own endpoint. Omit `id` and one is minted.

```json
{
  "bits": [
    {
      "ref": "tokenizer",
      "type": "Tokenizer",
      "download_link": "https://huggingface.co/org/model/resolve/main/tokenizer.json",
      "file_name": "tokenizer.json",
      "license": "apache-2.0",
      "meta": { "en": { "name": "Tokenizer", "description": "…" } }
    },
    {
      "ref": "model",
      "type": "Llm",
      "download_link": "https://huggingface.co/org/model/resolve/main/model.gguf",
      "file_name": "model.gguf",
      "dependencies": ["@tokenizer"],
      "parameters": { "context_length": 131072, "provider": { "provider_name": "Local" } },
      "meta": { "en": { "name": "The model", "description": "…" } }
    }
  ]
}
```

A hosted bit is the same shape without `download_link`, naming its profile
under `parameters.provider` (for example `hosted:openrouter` with a
`@preset/…` model id). `hub pull` on an existing entry is the quickest way to
get a correct starting point for either kind.

The hub fetches, hashes and mirrors every artifact itself, so a spec only ever
names source URLs. Dependencies are stored as `hub:id`; the tool writes each
dependency first and builds the reference from what the hub stored.
