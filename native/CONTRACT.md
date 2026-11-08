# Native app implementation contract

Desktop: Tauri 2, Rust commands, static HTML/CSS/ES module UI in `native/ui`. Android: Kotlin/Compose in `native/android`. Shared assets in `native/shared/assets`. No Python or VPS runtime.

Frontend calls `window.__TAURI__.core.invoke`. Commands return JSON objects or reject with a short error code/message. All Rust types use camelCase JSON fields.

- `app_info()` -> `{version, outputDir, hasEmbeddedKey, codexAvailable}`
- `account()` -> provider account JSON
- `generate({request})` -> Job (submit + return; frontend polls separately). Request: `{provider:'elyum'|'codex', mode:'text'|'photo', description, photos:[dataURL], customReference:dataURL|null, character:'own'|'chiikawa'|'hachiware'|'usagi'|'trio', background:'scene'|'white'|'meadow'|'room'|'sticker', emotion:'scene'|'happy'|'shy'|'tears'|'surprised', strength:50, fullCreature:false, aspectRatio:'1:1', resolution:'2K', creditMode:false, cleanCodex:true}`.
- `get_job({jobId})` -> Job; polls server once, downloads preview/original when available. Never keeps/charges automatically.
- `keep_original({jobId, confirmedCost})` -> Job; only user click calls this, passing the displayed price. Refresh status before keep, unknown/changed price guarded, never retry a write automatically. Completed job can retry download without repaying.
- `restore_job({jobId})` -> Job; imports an Elyum g_* task ID after an interrupted request. Never purchases a preview.
- `list_jobs()` -> Job[] from local persisted state.
- `export_image({jobId})` -> saved path or null; native save dialog for original or clearly named preview.
- `codex_account()` -> JSON `{authenticated, plan?}`
- `codex_login()` -> `{authUrl?, message?}` starts official sign-in; open URL using native browser. Must preserve login state until callback completes.

Job: `{id, provider, status:'queued'|'running'|'preview'|'completed'|'failed', imageUrl?:dataURL, imagePath?:string, cost?:number, error?:string, createdAt:string, description?:string}`. Persistence stores provider IDs needed for keep and output paths. Data URLs at boundary so WebView file access not broadly granted. Image content not persisted in job JSON unnecessarily.

Frontend language switch ru/en/ja stored in localStorage; all controls and status messages localized. Backend error codes localized by frontend. Code/prompt semantics port from root prompts.py, official references bundled. Multiple photos (1–8) combine in one scene. Classic transformation default; full-creature separate. Credit charge only after preview visible and explicit confirmation of price.

Token supplied by root from local .env at build as `CHIIKAWA_EMBEDDED_MCP_KEY`. Never hardcode credentials in source, logs or workflow YAML. User authorized embedding in delivered binaries. MCP endpoint fixed https://elyum.ai/mcp. Official Codex desktop provider must remain; Android has Elyum only because official Codex app-server is desktop-only.

Ownership: backend agent `native/desktop` and Rust tests; design agent `native/ui`, shared assets, `.hallmark`, icon assets; Android agent `native/android`; root build scripts/workflows/documentation/legacy Gradio restore. Communicate contract changes before editing other ownership.
