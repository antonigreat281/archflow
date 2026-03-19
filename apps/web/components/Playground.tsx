"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import Editor, { type OnMount } from "@monaco-editor/react";
import { examples } from "./playground/examples";
import { resolveIcons } from "./playground/icons";

const THEMES = ["default", "dark", "ocean", "sunset"];

type WasmModule = {
  render_svg: (json: string) => string;
  parse_dsl: (dsl: string) => string;
};

export function Playground() {
  const [svg, setSvg] = useState("");
  const [error, setError] = useState("");
  const [ready, setReady] = useState(false);
  const [mode, setMode] = useState<"dsl" | "json">("dsl");
  const [theme, setTheme] = useState("default");
  const [status, setStatus] = useState("Loading WASM...");

  const wasmRef = useRef<WasmModule | null>(null);
  const editorRef = useRef<Parameters<OnMount>[0] | null>(null);
  const monacoRef = useRef<Parameters<OnMount>[1] | null>(null);
  const previewRef = useRef<HTMLDivElement>(null);

  // Pan & Zoom
  const [scale, setScale] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const panningRef = useRef(false);
  const startRef = useRef({ x: 0, y: 0 });

  // Load WASM
  useEffect(() => {
    (async () => {
      try {
        const wasm = await import(
          /* webpackIgnore: true */ "/wasm/archflow_wasm.js"
        );
        await wasm.default("/wasm/archflow_wasm_bg.wasm");
        wasmRef.current = wasm;
        setReady(true);
        setStatus("Ready");
      } catch (e) {
        setStatus(`WASM load failed: ${e instanceof Error ? e.message : e}`);
      }
    })();
  }, []);

  // Render
  const render = useCallback(async () => {
    const wasm = wasmRef.current;
    const editor = editorRef.current;
    if (!wasm || !editor) return;

    const content = editor.getValue();
    setError("");

    try {
      let irJson: string;
      if (mode === "dsl") {
        irJson = wasm.parse_dsl(content);
      } else {
        irJson = content;
      }

      const ir = JSON.parse(irJson);
      if (!ir.metadata) ir.metadata = {};
      ir.metadata.theme = theme;

      await resolveIcons(ir);

      const result = wasm.render_svg(JSON.stringify(ir));
      setSvg(result);
      setStatus("Ready");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setStatus("Error");
    }
  }, [mode, theme]);

  // Auto-render on ready
  useEffect(() => {
    if (ready) {
      const timer = setTimeout(render, 100);
      return () => clearTimeout(timer);
    }
  }, [ready, render]);

  // Monaco mount
  const handleEditorMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;

    // Register archflow language
    monaco.languages.register({ id: "archflow" });
    monaco.languages.setMonarchTokensProvider("archflow", {
      tokenizer: {
        root: [
          [/#.*$/, "comment"],
          [/\/\/.*$/, "comment"],
          [/^(title|direction|theme|icon_size|node_width|spacing)\s*:/, "keyword"],
          [/^use\b/, "keyword"],
          [/^cluster\b/, "keyword"],
          [/cluster:[a-z][a-z0-9-]*:[a-z][a-z0-9-]*/, "keyword"],
          [/>>/, "operator"],
          [/\[/, "delimiter.bracket", "@edgeLabel"],
          [/[a-z][a-z0-9-]*:[A-Za-z][A-Za-z0-9-]*/, "type"],
          [/[{}]/, "delimiter.bracket"],
        ],
        edgeLabel: [
          [/[^\]]+/, "string"],
          [/\]/, "delimiter.bracket", "@pop"],
        ],
      },
    });

    // Auto-render on change
    let timeout: ReturnType<typeof setTimeout>;
    editor.onDidChangeModelContent(() => {
      clearTimeout(timeout);
      timeout = setTimeout(render, 400);
    });
  };

  // Mode toggle
  const toggleMode = () => {
    const wasm = wasmRef.current;
    const editor = editorRef.current;
    const mn = monacoRef.current;
    if (!wasm || !editor || !mn) return;

    if (mode === "dsl") {
      try {
        const json = wasm.parse_dsl(editor.getValue());
        const ir = JSON.parse(json);
        const model = editor.getModel();
        if (model) mn.editor.setModelLanguage(model, "json");
        editor.setValue(JSON.stringify(ir, null, 2));
        setMode("json");
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    } else {
      const model = editor.getModel();
      if (model) mn.editor.setModelLanguage(model, "archflow");
      editor.setValue(examples[0].dsl);
      setMode("dsl");
    }
  };

  // Download
  const download = () => {
    if (!svg) return;
    const blob = new Blob([svg], { type: "image/svg+xml" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "archflow-diagram.svg";
    a.click();
    URL.revokeObjectURL(url);
  };

  // Share
  const share = () => {
    const editor = editorRef.current;
    if (!editor) return;
    const encoded = btoa(unescape(encodeURIComponent(editor.getValue())));
    const url = `${location.origin}/playground#${mode}/${encoded}`;
    navigator.clipboard.writeText(url).then(() => setStatus("Link copied!"));
  };

  // Zoom
  const zoomIn = () => setScale((s) => Math.min(s * 1.25, 5));
  const zoomOut = () => setScale((s) => Math.max(s * 0.8, 0.1));
  const zoomReset = () => {
    setScale(1);
    setPan({ x: 0, y: 0 });
  };

  // Wheel zoom
  const onWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    setScale((s) => {
      const factor = e.deltaY < 0 ? 1.1 : 0.9;
      return Math.min(Math.max(s * factor, 0.1), 5);
    });
  }, []);

  // Pan
  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      panningRef.current = true;
      startRef.current = { x: e.clientX - pan.x, y: e.clientY - pan.y };
    },
    [pan]
  );

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (!panningRef.current) return;
      setPan({
        x: e.clientX - startRef.current.x,
        y: e.clientY - startRef.current.y,
      });
    };
    const onUp = () => {
      panningRef.current = false;
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, []);

  const btnStyle: React.CSSProperties = {
    padding: "4px 10px",
    borderRadius: "4px",
    border: "1px solid #d1d5db",
    background: "#fff",
    cursor: "pointer",
    fontSize: "12px",
    fontWeight: 500,
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        overflow: "hidden",
        background: "#1e1e1e",
        color: "#d4d4d4",
      }}
    >
      {/* Toolbar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          padding: "6px 12px",
          borderBottom: "1px solid #333",
          background: "#252526",
          flexShrink: 0,
        }}
      >
        <a
          href="/"
          style={{
            fontWeight: 700,
            fontSize: "15px",
            textDecoration: "none",
            color: "#d4d4d4",
            marginRight: "12px",
          }}
        >
          arch<span style={{ color: "#569cd6" }}>flow</span>
        </a>

        <button
          type="button"
          onClick={toggleMode}
          style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}
          title={mode === "dsl" ? "Switch to JSON" : "Switch to DSL"}
        >
          {mode.toUpperCase()}
        </button>

        <select
          onChange={(e) => {
            const editor = editorRef.current;
            if (editor && mode === "dsl") {
              editor.setValue(examples[Number(e.target.value)].dsl);
            }
          }}
          style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}
        >
          {examples.map((ex, i) => (
            <option key={ex.name} value={i}>
              {ex.name}
            </option>
          ))}
        </select>

        <select
          value={theme}
          onChange={(e) => {
            setTheme(e.target.value);
            setTimeout(render, 50);
          }}
          style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}
        >
          {THEMES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>

        <button
          type="button"
          onClick={render}
          disabled={!ready}
          style={{
            ...btnStyle,
            background: ready ? "#569cd6" : "#555",
            color: "#fff",
            border: "none",
          }}
        >
          Render
        </button>

        <div style={{ flex: 1 }} />

        <span style={{ fontSize: "11px", color: "#888" }}>
          {Math.round(scale * 100)}%
        </span>
        <button type="button" onClick={zoomOut} style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}>−</button>
        <button type="button" onClick={zoomIn} style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}>+</button>
        <button type="button" onClick={zoomReset} style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}>Reset</button>

        <button type="button" onClick={download} style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}>
          Download
        </button>
        <button type="button" onClick={share} style={{ ...btnStyle, background: "#333", color: "#d4d4d4", border: "1px solid #555" }}>
          Share
        </button>
      </div>

      {/* Main */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          flex: 1,
          overflow: "hidden",
        }}
      >
        {/* Editor */}
        <div style={{ overflow: "hidden", position: "relative" }}>
          <Editor
            defaultLanguage="archflow"
            defaultValue={examples[0].dsl}
            theme="vs-dark"
            height="100%"
            onMount={handleEditorMount}
            options={{
              fontSize: 13,
              fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              tabSize: 2,
              wordWrap: "on",
              padding: { top: 8, bottom: 8 },
            }}
          />
        </div>

        {/* Preview */}
        <div
          ref={previewRef}
          onWheel={onWheel}
          onMouseDown={onMouseDown}
          style={{
            overflow: "hidden",
            background: "#fff",
            cursor: panningRef.current ? "grabbing" : "grab",
            position: "relative",
          }}
        >
          {error ? (
            <pre
              style={{
                color: "#f44",
                padding: "16px",
                fontSize: "13px",
                whiteSpace: "pre-wrap",
              }}
            >
              {error}
            </pre>
          ) : (
            <div
              style={{
                transform: `translate(${pan.x}px, ${pan.y}px) scale(${scale})`,
                transformOrigin: "0 0",
                padding: "24px",
              }}
              dangerouslySetInnerHTML={{ __html: svg }}
            />
          )}
        </div>
      </div>

      {/* Status bar */}
      <div
        style={{
          padding: "2px 12px",
          fontSize: "11px",
          background: "#007acc",
          color: "#fff",
          flexShrink: 0,
        }}
      >
        {status}
      </div>
    </div>
  );
}
