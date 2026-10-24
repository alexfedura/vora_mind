/**
 * Lazy Mermaid loader.
 * Mermaid is imported and initialized only when a diagram is first rendered.
 * Rendering is serialized because Mermaid mutates shared DOM during render.
 */
type MermaidModule = typeof import('mermaid');
type MermaidApi = MermaidModule['default'];

let mermaidApi: MermaidApi | null = null;
let mermaidLoading: Promise<MermaidApi> | null = null;
let mermaidLoadError: unknown = null;
const readyListeners = new Set<() => void>();
const renderCache = new Map<string, string>();
const MAX_CACHE_SIZE = 200;
let renderQueue: Promise<unknown> = Promise.resolve();

function cacheKey(id: string, source: string): string {
  return `${id}::${source}`;
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

function placeholderHtml(source: string): string {
  return `<div class="vm-mermaid-placeholder"><pre>${escapeHtml(source)}</pre></div>`;
}

function errorHtml(source: string, message: string): string {
  return (
    `<div class="vm-mermaid-error">` +
    `<div class="vm-mermaid-error-title">Mermaid error</div>` +
    `<pre>${escapeHtml(source)}</pre>` +
    `<div class="vm-mermaid-error-msg">${escapeHtml(message)}</div>` +
    `</div>`
  );
}

function notifyReady(): void {
  const listeners = [...readyListeners];
  readyListeners.clear();
  for (const listener of listeners) {
    try {
      listener();
    } catch (error) {
      console.error('Mermaid ready listener failed:', error);
    }
  }
}

async function loadMermaid(): Promise<MermaidApi> {
  if (mermaidApi) return mermaidApi;
  if (mermaidLoading) return mermaidLoading;

  mermaidLoadError = null;
  mermaidLoading = (async () => {
    try {
      const module = await import('mermaid');
      const api = module.default;
      api.initialize({
        startOnLoad: false,
        theme: 'neutral',
        securityLevel: 'strict',
        fontFamily:
          'ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
        flowchart: { htmlLabels: false, curve: 'basis' },
        sequence: { useMaxWidth: true },
        gantt: { useMaxWidth: true },
      });
      mermaidApi = api;
      return api;
    } catch (error) {
      mermaidLoadError = error;
      throw error;
    } finally {
      mermaidLoading = null;
      notifyReady();
    }
  })();

  return mermaidLoading;
}

export function isMermaidReady(): boolean {
  return mermaidApi !== null;
}

export function onMermaidReady(callback: () => void): () => void {
  if (mermaidApi || mermaidLoadError) {
    callback();
    return () => {};
  }
  readyListeners.add(callback);
  return () => readyListeners.delete(callback);
}

export function renderMermaid(
  id: string,
  source: string,
): { svg: string; isPlaceholder: boolean } {
  const cached = renderCache.get(cacheKey(id, source));
  if (cached !== undefined) return { svg: cached, isPlaceholder: false };
  if (mermaidLoadError) {
    const message = mermaidLoadError instanceof Error ? mermaidLoadError.message : String(mermaidLoadError);
    return { svg: errorHtml(source, message), isPlaceholder: false };
  }

  void renderMermaidAsync(id, source).catch((error: unknown) => {
    console.error('Could not render Mermaid diagram:', error);
  });
  return { svg: placeholderHtml(source), isPlaceholder: true };
}

export async function renderMermaidAsync(id: string, source: string): Promise<string> {
  try {
    const api = await loadMermaid();
    const key = cacheKey(id, source);
    const cached = renderCache.get(key);
    if (cached !== undefined) return cached;

    const job = renderQueue.then(async () => {
      try {
        const result = await api.render(id, source);
        setCache(key, result.svg);
        return result.svg;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        const html = errorHtml(source, message);
        setCache(key, html);
        return html;
      }
    });
    renderQueue = job.catch((error: unknown) => {
      console.error('Mermaid render queue failed:', error);
    });
    return await job;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Could not load Mermaid:', error);
    return errorHtml(source, message);
  }
}

export function extractMermaidFences(source: string): Array<{ id: string; code: string }> {
  const fences: Array<{ id: string; code: string }> = [];
  const fencePattern = /```mermaid\b[^\n]*\n([\s\S]*?)```/g;
  let match: RegExpExecArray | null;
  let index = 0;
  while ((match = fencePattern.exec(source)) !== null) {
    fences.push({ id: `vm-mmd-${index++}`, code: match[1] ?? '' });
  }
  return fences;
}
