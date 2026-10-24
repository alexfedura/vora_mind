/**
 * Lazy KaTeX loader.
 * KaTeX and its stylesheet are imported only when math is first rendered.
 */
type KatexModule = typeof import('katex');

let katexModule: KatexModule | null = null;
let katexLoading: Promise<KatexModule> | null = null;
let katexLoadError: unknown = null;
const readyListeners = new Set<() => void>();
const renderCache = new Map<string, string>();
const MAX_CACHE_SIZE = 500;

function cacheKey(source: string, displayMode: boolean): string {
  return `${displayMode ? 'D' : 'I'}:${source}`;
}

function setCache(key: string, html: string): void {
  if (renderCache.size >= MAX_CACHE_SIZE) {
    const oldestKey = renderCache.keys().next().value;
    if (oldestKey !== undefined) renderCache.delete(oldestKey);
  }
  renderCache.set(key, html);
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function placeholderHtml(source: string, displayMode: boolean): string {
  const className = displayMode
    ? 'vm-math-placeholder vm-math-block'
    : 'vm-math-placeholder';
  return `<span class="${className}">${escapeHtml(source)}</span>`;
}

function errorHtml(source: string): string {
  return `<span class="vm-math-error" title="KaTeX parse error">${escapeHtml(source)}</span>`;
}

function notifyReady(): void {
  const listeners = [...readyListeners];
  readyListeners.clear();
  for (const listener of listeners) {
    try {
      listener();
    } catch (error) {
      console.error('KaTeX ready listener failed:', error);
    }
  }
}

async function loadKatex(): Promise<KatexModule> {
  if (katexModule) return katexModule;
  if (katexLoading) return katexLoading;

  katexLoadError = null;
  katexLoading = (async () => {
    try {
      const [module] = await Promise.all([
        import('katex'),
        import('katex/dist/katex.min.css'),
      ]);
      katexModule = module;
      return module;
    } catch (error) {
      katexLoadError = error;
      throw error;
    } finally {
      katexLoading = null;
      notifyReady();
    }
  })();

  return katexLoading;
}

export function isKatexReady(): boolean {
  return katexModule !== null;
}

export function onKatexReady(callback: () => void): () => void {
  if (katexModule || katexLoadError) {
    callback();
    return () => {};
  }
  readyListeners.add(callback);
  return () => readyListeners.delete(callback);
}

export function renderMath(
  source: string,
  displayMode: boolean,
): { html: string; isPlaceholder: boolean } {
  const key = cacheKey(source, displayMode);
  const cached = renderCache.get(key);
  if (cached !== undefined) return { html: cached, isPlaceholder: false };

  if (katexModule) {
    try {
      const html = katexModule.renderToString(source, {
        displayMode,
        throwOnError: false,
        strict: 'ignore',
        trust: false,
        output: 'html',
      });
      setCache(key, html);
      return { html, isPlaceholder: false };
    } catch (error) {
      console.error('Could not render math with KaTeX:', error);
      return { html: errorHtml(source), isPlaceholder: false };
    }
  }

  if (katexLoadError) return { html: errorHtml(source), isPlaceholder: false };
  void loadKatex().catch((error: unknown) => {
    console.error('Could not load KaTeX:', error);
  });
  return { html: placeholderHtml(source, displayMode), isPlaceholder: true };
}

export async function renderMathAsync(source: string, displayMode: boolean): Promise<string> {
  try {
    const katex = await loadKatex();
    const key = cacheKey(source, displayMode);
    const cached = renderCache.get(key);
    if (cached !== undefined) return cached;

    const html = katex.renderToString(source, {
      displayMode,
      throwOnError: false,
      strict: 'ignore',
      trust: false,
      output: 'html',
    });
    setCache(key, html);
    return html;
  } catch (error) {
    console.error('Could not render math with KaTeX:', error);
    return errorHtml(source);
  }
}
