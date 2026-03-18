/**
 * Archflow DSL Parser
 *
 * Syntax:
 *   title: My Diagram
 *   direction: LR
 *   icon_sources: github:archflow/icons, https://example.com/icons
 *
 *   Node A >> Node B >> Node C
 *   Node A >> Node B : label text
 *   aws:EC2 Web Server >> aws:RDS Database
 *
 *   cluster My Group {
 *     Node A
 *     Node B
 *   }
 *
 *   cluster:aws:vpc My VPC {
 *     Node A
 *   }
 *
 * Node IDs are auto-generated from labels (lowercased, spaces to underscores).
 * Provider syntax: provider:type Label (e.g., aws:EC2 Web Server)
 */

// Built-in mini icon SVGs for playground (no external fetch needed)
const BUILTIN_ICONS = {
  aws: {
    ec2:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#FF9900"/><text x="16" y="20" text-anchor="middle" font-size="10" font-weight="bold" fill="#fff" font-family="sans-serif">EC2</text>',
    rds:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#3B48CC"/><text x="16" y="20" text-anchor="middle" font-size="10" font-weight="bold" fill="#fff" font-family="sans-serif">RDS</text>',
    s3:           '<rect x="2" y="2" width="28" height="28" rx="4" fill="#3F8624"/><text x="16" y="20" text-anchor="middle" font-size="11" font-weight="bold" fill="#fff" font-family="sans-serif">S3</text>',
    lambda:       '<rect x="2" y="2" width="28" height="28" rx="4" fill="#FF9900"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">λ</text>',
    elb:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#8C4FFF"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">ELB</text>',
    sqs:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#FF4F8B"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">SQS</text>',
    sns:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#FF4F8B"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">SNS</text>',
    dynamodb:     '<rect x="2" y="2" width="28" height="28" rx="4" fill="#3B48CC"/><text x="16" y="20" text-anchor="middle" font-size="7" font-weight="bold" fill="#fff" font-family="sans-serif">DDB</text>',
    elasticache:  '<rect x="2" y="2" width="28" height="28" rx="4" fill="#3B48CC"/><text x="16" y="20" text-anchor="middle" font-size="7" font-weight="bold" fill="#fff" font-family="sans-serif">Cache</text>',
    cloudfront:   '<rect x="2" y="2" width="28" height="28" rx="4" fill="#8C4FFF"/><text x="16" y="20" text-anchor="middle" font-size="7" font-weight="bold" fill="#fff" font-family="sans-serif">CDN</text>',
  },
  gcp: {
    gce:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#4285F4"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">GCE</text>',
    gcs:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#4285F4"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">GCS</text>',
    cloudsql:     '<rect x="2" y="2" width="28" height="28" rx="4" fill="#4285F4"/><text x="16" y="20" text-anchor="middle" font-size="8" font-weight="bold" fill="#fff" font-family="sans-serif">SQL</text>',
    gke:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#4285F4"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">GKE</text>',
  },
  k8s: {
    pod:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#326CE5"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">Pod</text>',
    service:      '<rect x="2" y="2" width="28" height="28" rx="14" fill="#326CE5"/><text x="16" y="20" text-anchor="middle" font-size="8" font-weight="bold" fill="#fff" font-family="sans-serif">Svc</text>',
    ingress:      '<rect x="2" y="2" width="28" height="28" rx="4" fill="#326CE5"/><text x="16" y="20" text-anchor="middle" font-size="8" font-weight="bold" fill="#fff" font-family="sans-serif">Ing</text>',
    deployment:   '<rect x="2" y="2" width="28" height="28" rx="4" fill="#326CE5"/><text x="16" y="20" text-anchor="middle" font-size="7" font-weight="bold" fill="#fff" font-family="sans-serif">Dep</text>',
  },
  generic: {
    server:       '<rect x="2" y="4" width="28" height="24" rx="3" fill="#6B7280" stroke="#4B5563" stroke-width="1"/><line x1="6" y1="10" x2="26" y2="10" stroke="#9CA3AF" stroke-width="1"/><line x1="6" y1="16" x2="26" y2="16" stroke="#9CA3AF" stroke-width="1"/><circle cx="24" cy="7" r="1.5" fill="#34D399"/>',
    database:     '<ellipse cx="16" cy="8" rx="12" ry="5" fill="#6B7280" stroke="#4B5563" stroke-width="1"/><rect x="4" y="8" width="24" height="16" fill="#6B7280" stroke="#4B5563" stroke-width="1"/><ellipse cx="16" cy="24" rx="12" ry="5" fill="#6B7280" stroke="#4B5563" stroke-width="1"/><ellipse cx="16" cy="8" rx="12" ry="5" fill="#9CA3AF"/>',
    cache:        '<rect x="2" y="2" width="28" height="28" rx="4" fill="#F59E0B"/><text x="16" y="20" text-anchor="middle" font-size="8" font-weight="bold" fill="#fff" font-family="sans-serif">Cache</text>',
    queue:        '<rect x="2" y="6" width="28" height="20" rx="3" fill="#8B5CF6"/><text x="16" y="20" text-anchor="middle" font-size="8" font-weight="bold" fill="#fff" font-family="sans-serif">Queue</text>',
    user:         '<circle cx="16" cy="10" r="6" fill="#6B7280"/><path d="M6 28 Q6 20 16 20 Q26 20 26 28" fill="#6B7280"/>',
    loadbalancer: '<rect x="2" y="2" width="28" height="28" rx="4" fill="#8C4FFF"/><text x="16" y="20" text-anchor="middle" font-size="8" font-weight="bold" fill="#fff" font-family="sans-serif">LB</text>',
    firewall:     '<rect x="2" y="2" width="28" height="28" rx="4" fill="#EF4444"/><text x="16" y="20" text-anchor="middle" font-size="8" font-weight="bold" fill="#fff" font-family="sans-serif">FW</text>',
    dns:          '<rect x="2" y="2" width="28" height="28" rx="4" fill="#06B6D4"/><text x="16" y="20" text-anchor="middle" font-size="9" font-weight="bold" fill="#fff" font-family="sans-serif">DNS</text>',
  },
};

function getBuiltinIcon(provider, icon) {
  const p = BUILTIN_ICONS[provider];
  return p ? (p[icon] || null) : null;
}

export function parseDSL(input) {
  const lines = input.split('\n');
  const ir = {
    version: '1.0.0',
    metadata: { title: '', direction: 'TB', theme: 'default', icon_sources: [] },
    nodes: [],
    clusters: [],
    edges: [],
  };

  const nodeMap = new Map(); // label -> id
  let i = 0;

  function toId(label) {
    return label.trim().toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '');
  }

  /**
   * Parse a node spec that may include provider:type prefix.
   * Returns { label, provider, icon } where provider/icon may be null.
   * Format: "provider:type Label" or just "Label"
   */
  function parseNodeSpec(raw) {
    raw = raw.trim();
    const providerMatch = raw.match(/^([a-z][a-z0-9]*):([a-zA-Z][a-zA-Z0-9]*)\s+(.+)$/);
    if (providerMatch) {
      return {
        label: providerMatch[3].trim(),
        provider: providerMatch[1],
        icon: providerMatch[2].toLowerCase(),
      };
    }
    return { label: raw, provider: null, icon: null };
  }

  function ensureNode(rawLabel) {
    const spec = parseNodeSpec(rawLabel);
    const label = spec.label;
    if (!label) return null;
    const id = toId(label);
    if (!nodeMap.has(label)) {
      nodeMap.set(label, id);
      const node = { id, label };
      if (spec.provider) node.provider = spec.provider;
      if (spec.icon) node.icon = spec.icon;
      // Auto-resolve built-in icons for playground
      if (spec.provider && spec.icon) {
        const svg = getBuiltinIcon(spec.provider, spec.icon);
        if (svg) node.icon_svg = svg;
      }
      ir.nodes.push(node);
    }
    return id;
  }

  function extractLabel(rawLabel) {
    const spec = parseNodeSpec(rawLabel);
    return spec.label;
  }

  function parseStyle(styleStr) {
    const style = {};
    const pairs = styleStr.split(',');
    for (const pair of pairs) {
      const [key, val] = pair.split(':').map(s => s.trim());
      if (key && val) {
        const k = key.replace(/-/g, '_');
        style[k] = isNaN(Number(val)) ? val : Number(val);
      }
    }
    return Object.keys(style).length > 0 ? style : undefined;
  }

  while (i < lines.length) {
    let line = lines[i].trimEnd();
    const trimmed = line.trim();
    i++;

    // Skip empty lines and comments
    if (!trimmed || trimmed.startsWith('#') || trimmed.startsWith('//')) continue;

    // Metadata: title: ...
    const titleMatch = trimmed.match(/^title\s*:\s*(.+)$/i);
    if (titleMatch) {
      ir.metadata.title = titleMatch[1].trim();
      continue;
    }

    // Metadata: direction: ...
    const dirMatch = trimmed.match(/^direction\s*:\s*(TB|LR|BT|RL)$/i);
    if (dirMatch) {
      ir.metadata.direction = dirMatch[1].toUpperCase();
      continue;
    }

    // Metadata: theme: ...
    const themeMatch = trimmed.match(/^theme\s*:\s*(.+)$/i);
    if (themeMatch) {
      ir.metadata.theme = themeMatch[1].trim();
      continue;
    }

    // Metadata: icon_sources: ...
    const iconSourcesMatch = trimmed.match(/^icon_sources\s*:\s*(.+)$/i);
    if (iconSourcesMatch) {
      ir.metadata.icon_sources = iconSourcesMatch[1].split(',').map(s => s.trim()).filter(Boolean);
      continue;
    }

    // Cluster with provider: cluster:provider:type Name { ... }
    const providerClusterMatch = trimmed.match(/^cluster:([a-z][a-z0-9]*):([a-z][a-z0-9]*)\s+(.+?)\s*\{$/i);
    if (providerClusterMatch) {
      const clusterProvider = providerClusterMatch[1];
      const clusterType = providerClusterMatch[2];
      const clusterLabel = providerClusterMatch[3].trim();
      const clusterId = toId(clusterLabel);
      const children = [];

      while (i < lines.length) {
        const cline = lines[i].trim();
        i++;
        if (cline === '}') break;
        if (!cline || cline.startsWith('#') || cline.startsWith('//')) continue;
        const nodeId = ensureNode(cline);
        if (nodeId) children.push(nodeId);
      }

      ir.clusters.push({
        id: clusterId,
        label: clusterLabel,
        children,
        provider: clusterProvider,
        cluster_type: clusterType,
      });
      continue;
    }

    // Cluster: cluster Name { ... }
    const clusterMatch = trimmed.match(/^cluster\s+(.+?)\s*\{$/i);
    if (clusterMatch) {
      const clusterLabel = clusterMatch[1].trim();
      const clusterId = toId(clusterLabel);
      const children = [];

      while (i < lines.length) {
        const cline = lines[i].trim();
        i++;
        if (cline === '}') break;
        if (!cline || cline.startsWith('#') || cline.startsWith('//')) continue;
        // Each line in cluster is a node name (possibly with provider prefix)
        const nodeLabel = cline;
        const nodeId = ensureNode(nodeLabel);
        if (nodeId) children.push(nodeId);
      }

      ir.clusters.push({ id: clusterId, label: clusterLabel, children });
      continue;
    }

    // Edge chain: A >> B >> C  or  A >> B : label
    if (trimmed.includes('>>')) {
      // Split by >> but handle labels with :
      const parts = trimmed.split('>>').map(s => s.trim());

      for (let j = 0; j < parts.length - 1; j++) {
        const fromRaw = parts[j].split(':').length > 2
          ? parts[j]  // provider:type label
          : parts[j].includes(':') && !parts[j].match(/^[a-z]+:[A-Z]/)
            ? parts[j].split(':')[0].trim()
            : parts[j];
        const fromId = ensureNode(fromRaw);

        let toRaw, edgeLabel;
        const toPart = parts[j + 1];

        // Last segment can have : for edge label (but not provider:type)
        if (j === parts.length - 2) {
          // Check if the colon is for a provider prefix or an edge label
          const provCheck = toPart.match(/^([a-z][a-z0-9]*):([a-zA-Z][a-zA-Z0-9]*)\s+/);
          if (provCheck) {
            // Has provider prefix — check if there's ANOTHER colon for edge label
            const afterProvider = toPart.substring(provCheck[0].length);
            const colonIdx = afterProvider.indexOf(':');
            if (colonIdx >= 0) {
              toRaw = toPart.substring(0, provCheck[0].length + colonIdx).trim();
              edgeLabel = afterProvider.substring(colonIdx + 1).trim();
            } else {
              toRaw = toPart;
            }
          } else if (toPart.includes(':')) {
            const colonIdx = toPart.indexOf(':');
            toRaw = toPart.substring(0, colonIdx).trim();
            edgeLabel = toPart.substring(colonIdx + 1).trim();
          } else {
            toRaw = toPart;
          }
        } else {
          toRaw = toPart;
        }

        const toId = ensureNode(toRaw);
        if (fromId && toId) {
          const edge = { from: fromId, to: toId };
          if (edgeLabel) edge.label = edgeLabel;
          ir.edges.push(edge);
        }
      }
      continue;
    }

    // Single edge: A -> B or A -> B : label (also support ->)
    if (trimmed.includes('->')) {
      const parts = trimmed.split('->').map(s => s.trim());
      for (let j = 0; j < parts.length - 1; j++) {
        const fromLabel = parts[j];
        const fromId = ensureNode(fromLabel);

        let toLabel, edgeLabel;
        const toPart = parts[j + 1];

        if (j === parts.length - 2 && toPart.includes(':')) {
          const colonIdx = toPart.indexOf(':');
          toLabel = toPart.substring(0, colonIdx).trim();
          edgeLabel = toPart.substring(colonIdx + 1).trim();
        } else {
          toLabel = toPart;
        }

        const toId = ensureNode(toLabel);
        if (fromId && toId) {
          const edge = { from: fromId, to: toId };
          if (edgeLabel) edge.label = edgeLabel;
          ir.edges.push(edge);
        }
      }
      continue;
    }

    // Standalone node (just a name on its own line, outside cluster)
    if (trimmed && !trimmed.includes('{') && !trimmed.includes('}')) {
      ensureNode(trimmed);
    }
  }

  return ir;
}
