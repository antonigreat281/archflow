/**
 * Icon browser for the Icons section.
 * Loads provider manifests from registry and displays all available icons.
 * Click an icon card to copy its DSL syntax.
 */
(function () {
  const REGISTRY = (location.hostname === 'localhost' || location.hostname === '127.0.0.1')
    ? location.origin
    : 'https://raw.githubusercontent.com/soulee-dev/archflow-icons/main';

  const container = document.getElementById('icons-container');
  const searchInput = document.getElementById('icon-search');
  const statsEl = document.getElementById('icon-stats');
  const toast = document.getElementById('toast');

  if (!container) return;

  let allCards = [];
  let loaded = false;

  // Lazy load: only fetch icons when the section becomes visible
  const observer = new MutationObserver(() => {
    const section = document.getElementById('icons');
    if (section && section.classList.contains('active') && !loaded) {
      loaded = true;
      loadIcons();
    }
  });
  observer.observe(document.body, { subtree: true, attributes: true, attributeFilter: ['class'] });

  async function loadIcons() {
    try {
      const rootResp = await fetch(`${REGISTRY}/manifest.json`);
      const root = await rootResp.json();
      const providers = Object.keys(root.providers || {}).sort();

      container.innerHTML = '';
      let totalIcons = 0;

      for (const provider of providers) {
        const mfResp = await fetch(`${REGISTRY}/${provider}/manifest.json`);
        const mf = await mfResp.json();

        const section = document.createElement('div');
        section.className = 'provider-section';

        const nodeCount = (mf.nodes || []).length;
        const clusterCount = (mf.clusters || []).length;
        const catCount = (mf.categories || []).length;

        let badges = `${nodeCount} nodes`;
        if (clusterCount) badges += `, ${clusterCount} clusters`;
        if (catCount) badges += `, ${catCount} categories`;

        section.innerHTML = `
          <div class="provider-header">
            <span class="provider-name">${provider.toUpperCase()}</span>
            <span class="provider-badge">${badges}</span>
            <span class="provider-meta">${mf.source_version || ''}</span>
          </div>
        `;

        const subdirs = [
          ['Nodes', 'nodes', mf.nodes],
          ['Clusters', 'clusters', mf.clusters],
          ['Categories', 'categories', mf.categories],
        ];

        for (const [title, subdir, items] of subdirs) {
          if (!items || !items.length) continue;

          const titleEl = document.createElement('div');
          titleEl.className = 'category-title';
          titleEl.textContent = title;
          section.appendChild(titleEl);

          const grid = document.createElement('div');
          grid.className = 'icon-grid';

          for (const name of items) {
            const card = createCard(provider, subdir, name);
            grid.appendChild(card);
            totalIcons++;
          }
          section.appendChild(grid);
        }

        container.appendChild(section);
      }

      if (statsEl) statsEl.textContent = `${totalIcons} icons across ${providers.length} providers`;
    } catch (e) {
      container.innerHTML = `<div class="icons-loading" style="color:#e74c3c;">Failed to load: ${e.message}</div>`;
    }
  }

  function createCard(provider, subdir, name) {
    const card = document.createElement('div');
    card.className = 'icon-card';
    card.dataset.provider = provider;
    card.dataset.name = name;
    card.dataset.subdir = subdir;

    const dsl = subdir === 'nodes' ? `${provider}:${name}` : (subdir === 'clusters' ? `cluster:${provider}:${name}` : `${provider}/${name}`);

    card.innerHTML = `
      <div class="icon-preview"></div>
      <div class="icon-name">${name}</div>
      <div class="dsl-hint">${dsl}</div>
    `;

    card.addEventListener('click', () => {
      navigator.clipboard.writeText(dsl).then(() => {
        if (toast) {
          toast.textContent = `Copied: ${dsl}`;
          toast.classList.add('show');
          setTimeout(() => toast.classList.remove('show'), 1500);
        }
      });
    });

    // Lazy load SVG
    const preview = card.querySelector('.icon-preview');
    const url = `${REGISTRY}/${provider}/${subdir}/${name}.svg`;

    fetch(url).then(r => {
      if (!r.ok) { preview.textContent = '?'; return r.text(); }
      return r.text();
    }).then(svg => {
      if (svg && !svg.startsWith('?')) {
        preview.innerHTML = svg;
        const svgEl = preview.querySelector('svg');
        if (svgEl) {
          svgEl.removeAttribute('width');
          svgEl.removeAttribute('height');
          svgEl.style.width = '36px';
          svgEl.style.height = '36px';
        }
      }
    }).catch(() => { preview.textContent = '?'; });

    allCards.push(card);
    return card;
  }

  // Search
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      const q = searchInput.value.toLowerCase().trim();
      let visible = 0;
      for (const card of allCards) {
        const match = !q || card.dataset.name.includes(q) || card.dataset.provider.includes(q);
        card.classList.toggle('hidden', !match);
        if (match) visible++;
      }
      if (statsEl) {
        statsEl.textContent = q ? `${visible} matching` : `${allCards.length} icons`;
      }
    });
  }
})();
