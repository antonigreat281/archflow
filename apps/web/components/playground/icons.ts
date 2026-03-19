const DEFAULT_REGISTRY =
  typeof window !== "undefined" &&
  (location.hostname === "localhost" || location.hostname === "127.0.0.1")
    ? location.origin
    : "https://raw.githubusercontent.com/soulee-dev/archflow-icons/main";

const iconCache = new Map<string, string | null>();
const manifestCache = new Map<string, Record<string, unknown> | null>();

function resolveSourceBase(source: string | null | undefined): string {
  if (!source) return DEFAULT_REGISTRY;
  const ghMatch = source.match(/^github:(.+\/.+)$/);
  if (ghMatch) {
    return `https://raw.githubusercontent.com/${ghMatch[1]}/main`;
  }
  if (source.startsWith("https://") || source.startsWith("http://")) {
    return source.replace(/\/$/, "");
  }
  return DEFAULT_REGISTRY;
}

async function fetchManifest(
  provider: string,
  baseUrl: string
): Promise<Record<string, unknown> | null> {
  const key = `${baseUrl}/${provider}/manifest`;
  if (manifestCache.has(key)) return manifestCache.get(key) ?? null;

  try {
    const resp = await fetch(`${baseUrl}/${provider}/manifest.json`);
    if (!resp.ok) {
      manifestCache.set(key, null);
      return null;
    }
    const manifest = await resp.json();
    manifestCache.set(key, manifest);
    return manifest;
  } catch {
    manifestCache.set(key, null);
    return null;
  }
}

async function fetchIcon(url: string): Promise<string | null> {
  if (iconCache.has(url)) return iconCache.get(url) ?? null;
  try {
    const resp = await fetch(url);
    if (!resp.ok) {
      iconCache.set(url, null);
      return null;
    }
    let svg = await resp.text();
    svg = svg.replace(/<script[\s\S]*?<\/script>/gi, "");
    svg = svg.replace(/\bon\w+\s*=\s*["'][^"']*["']/gi, "");
    iconCache.set(url, svg);
    return svg;
  } catch {
    iconCache.set(url, null);
    return null;
  }
}

// biome-ignore lint: using any for IR flexibility
export async function resolveIcons(ir: any): Promise<any> {
  const providerSources =
    (ir.metadata && ir.metadata.provider_sources) || {};
  const declaredProviders = new Set(Object.keys(providerSources));
  if (declaredProviders.size === 0) return ir;

  const providerBaseUrls: Record<string, string> = {};
  const manifests: Record<string, Record<string, unknown> | null> = {};

  await Promise.all(
    [...declaredProviders].map(async (p) => {
      const base = resolveSourceBase(providerSources[p]);
      providerBaseUrls[p] = base;
      manifests[p] = await fetchManifest(p, base);
    })
  );

  // Apply cluster_styles
  for (const cluster of ir.clusters || []) {
    if (!cluster.provider || !cluster.cluster_type) continue;
    if (!declaredProviders.has(cluster.provider)) continue;
    if (cluster.style) continue;
    const mf = manifests[cluster.provider] as Record<string, unknown> | null;
    const styles = mf?.cluster_styles as
      | Record<string, unknown>
      | undefined;
    const preset = styles?.[cluster.cluster_type];
    if (preset) cluster.style = { ...preset as object };
  }

  // Apply node_render_modes
  const renderModes: Record<string, string> = {};
  for (const [p, mf] of Object.entries(manifests)) {
    if (mf && typeof mf.node_render_mode === "string")
      renderModes[p] = mf.node_render_mode;
  }
  if (Object.keys(renderModes).length > 0) {
    if (!ir.metadata) ir.metadata = {};
    ir.metadata.node_render_modes = renderModes;
  }

  // Resolve node + cluster icons
  const tasks: Promise<void>[] = [];

  for (const node of ir.nodes || []) {
    if (!node.provider || !node.icon) continue;
    if (!declaredProviders.has(node.provider)) continue;
    const base = providerBaseUrls[node.provider];
    tasks.push(
      fetchIcon(`${base}/${node.provider}/nodes/${node.icon}.svg`).then(
        (svg) => {
          if (svg) node.icon_svg = svg;
        }
      )
    );
  }

  for (const cluster of ir.clusters || []) {
    if (!cluster.provider || !cluster.cluster_type) continue;
    if (!declaredProviders.has(cluster.provider)) continue;
    const base = providerBaseUrls[cluster.provider];
    tasks.push(
      fetchIcon(
        `${base}/${cluster.provider}/clusters/${cluster.cluster_type}.svg`
      ).then((svg) => {
        if (svg) cluster.icon_svg = svg;
      })
    );
  }

  await Promise.allSettled(tasks);
  return ir;
}
