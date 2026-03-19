import init, {
  parse_dsl as _parseDsl,
  render_dsl as _renderDsl,
  render_svg as _renderSvg,
  initSync,
} from "../wasm/archflow_wasm.js";

let initialized = false;

async function ensureInit() {
  if (initialized) return;

  // Node.js: read wasm file from disk
  if (typeof globalThis.window === "undefined") {
    const { readFile } = await import("node:fs/promises");
    const { fileURLToPath } = await import("node:url");
    const { dirname, join } = await import("node:path");
    const dir = dirname(fileURLToPath(import.meta.url));
    const wasmPath = join(dir, "..", "wasm", "archflow_wasm_bg.wasm");
    const wasmBytes = await readFile(wasmPath);
    initSync({ module: wasmBytes });
  } else {
    // Browser: use default fetch-based init
    await init();
  }

  initialized = true;
}

/** Parse an Archflow DSL string and return JSON IR. */
export async function parseDsl(dsl: string): Promise<string> {
  await ensureInit();
  return _parseDsl(dsl);
}

/** Render an Archflow DSL string directly to SVG. */
export async function renderDsl(dsl: string): Promise<string> {
  await ensureInit();
  return _renderDsl(dsl);
}

/** Render a JSON IR string to SVG. */
export async function renderSvg(jsonIr: string): Promise<string> {
  await ensureInit();
  return _renderSvg(jsonIr);
}

/** Manually initialize the WASM module. Optional — called automatically on first use. */
export async function initialize(): Promise<void> {
  await ensureInit();
}
