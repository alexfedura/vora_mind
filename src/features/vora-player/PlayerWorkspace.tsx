import { useEffect, useRef, useState } from "react";
import {
  Copy,
  Film,
  Maximize,
  Pause,
  Play,
  Plus,
  SkipBack,
  SkipForward,
  Trash2,
  Volume2,
  X,
} from "lucide-react";
import type { MediaPlayerRequest } from "@/hooks/useMediaPlayer";
import { useMediaPlayer } from "@/hooks/useMediaPlayer";
import type { MediaSlice } from "@/mind/types";
import { formatMediaTime } from "@/mind/utils/mediaLinks";

interface PlayerWorkspaceProps {
  active: boolean;
  request: MediaPlayerRequest | null;
  onReturnToBoard: () => void;
}

function mediaFileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function PlayerWorkspace({
  active,
  request,
  onReturnToBoard,
}: PlayerWorkspaceProps) {
  const player = useMediaPlayer(active, request);
  const [seekValue, setSeekValue] = useState(0);
  const seekingRef = useRef(false);
  const playerRef = useRef(player);
  playerRef.current = player;

  useEffect(() => {
    if (!seekingRef.current) setSeekValue(player.position);
  }, [player.position]);

  useEffect(() => {
    if (!active) return;
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      if (event.target instanceof HTMLElement) {
        const tagName = event.target.tagName.toLowerCase();
        if (
          tagName === "input" ||
          tagName === "textarea" ||
          tagName === "select" ||
          event.target.isContentEditable
        ) return;
      }

      switch (event.key.toLowerCase()) {
        case " ":
          event.preventDefault();
          void playerRef.current.togglePlayback();
          break;
        case "arrowleft":
          event.preventDefault();
          void playerRef.current.seekBy(-5);
          break;
        case "arrowright":
          event.preventDefault();
          void playerRef.current.seekBy(5);
          break;
        case "f":
          event.preventDefault();
          void playerRef.current.toggleFullscreen();
          break;
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [active]);

  const openNewReference = (): void => {
    player.startNewReference();
  };

  const handleDelete = (slice: MediaSlice): void => {
    if (!window.confirm(`Delete the reference "${slice.title}"? The media file will not be changed.`)) return;
    void player.deleteReference(slice.id);
  };

  const saveDraft = (): void => {
    void player.saveReference();
  };

  return (
    <div
      className={`absolute inset-0 z-30 overflow-hidden bg-transparent text-white ${
        active ? "block" : "hidden"
      }`}
      aria-label="Vora Player workspace"
    >
      <header className="absolute left-3 right-3 top-3 z-20 flex min-h-14 items-center gap-3 rounded-xl border border-white/10 bg-[#111319]/85 px-4 py-2 shadow-xl backdrop-blur-md">
        <Film size={19} className="shrink-0 text-violet-300" aria-hidden="true" />
        <div className="min-w-0 flex-1">
          <div className="font-semibold">Vora Player</div>
          <div className="truncate text-xs text-zinc-300" title={player.source?.path}>
            {player.source ? player.source.path : "Local audio and video references"}
          </div>
        </div>
        {player.lastSource && !player.source && (
          <button
            type="button"
            onClick={() => void player.resumeLastSource()}
            disabled={!player.isReady || player.isBusy}
            className="rounded-lg border border-white/10 px-3 py-2 text-sm transition hover:bg-white/10 disabled:opacity-50"
          >
            Resume last source
          </button>
        )}
        <button
          type="button"
          onClick={() => void player.openMediaFile()}
          disabled={!player.isReady || player.isBusy}
          className="rounded-lg bg-violet-600 px-3 py-2 text-sm font-medium transition hover:bg-violet-500 disabled:opacity-50"
        >
          Open media
        </button>
        <button
          type="button"
          onClick={onReturnToBoard}
          className="rounded-lg p-2 text-zinc-300 transition hover:bg-white/10 hover:text-white"
          aria-label="Return to Board"
          title="Return to Board"
        >
          <X size={18} />
        </button>
      </header>

      {!player.source && (
        <div className="absolute inset-0 z-10 flex items-center justify-center px-6 text-center">
          <div className="max-w-lg rounded-2xl border border-white/10 bg-[#101116]/75 p-7 shadow-2xl backdrop-blur-md">
            <Film size={34} className="mx-auto mb-4 text-violet-300" aria-hidden="true" />
            <h2 className="text-xl font-semibold">Open a local media source</h2>
            <p className="mt-2 text-sm text-zinc-300">
              Files remain at their original paths. Time references are stored in Vora Mind;
              the source audio or video is never cut or rewritten.
            </p>
            {player.lastSource && (
              <p className="mt-4 truncate text-xs text-zinc-400" title={player.lastSource.path}>
                Last source: {mediaFileName(player.lastSource.path)} at{" "}
                {formatMediaTime(player.lastSource.last_position_seconds)}
              </p>
            )}
          </div>
        </div>
      )}

      <aside className="absolute bottom-24 right-3 top-20 z-20 flex w-[min(22rem,40vw)] flex-col overflow-hidden rounded-xl border border-white/10 bg-[#101116]/85 shadow-xl backdrop-blur-md">
        <div className="flex items-center justify-between border-b border-white/10 px-4 py-3">
          <div>
            <h2 className="font-semibold">Time references</h2>
            <p className="text-xs text-zinc-400">{player.slices.length} saved</p>
          </div>
          <button
            type="button"
            onClick={openNewReference}
            disabled={!player.source || player.isBusy}
            className="flex items-center gap-1 rounded-lg bg-violet-600 px-2.5 py-2 text-xs font-medium transition hover:bg-violet-500 disabled:opacity-50"
          >
            <Plus size={14} aria-hidden="true" />
            New reference
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          {player.slices.length === 0 ? (
            <p className="px-3 py-6 text-center text-sm text-zinc-400">
              Save a time range to create a reusable link.
            </p>
          ) : player.slices.map((slice) => (
            <article
              key={slice.id}
              className="mb-2 rounded-lg border border-white/10 bg-white/[0.035] p-3 transition hover:border-violet-400/40"
            >
              <button
                type="button"
                onClick={() => void player.selectReference(slice)}
                className="block w-full text-left"
                title="Select and seek to this reference"
              >
                <span className="block truncate font-medium">{slice.title}</span>
                <span className="mt-1 block text-xs text-violet-200">
                  {formatMediaTime(slice.start_seconds)} - {formatMediaTime(slice.end_seconds)}
                </span>
              </button>
              {slice.note && <p className="mt-2 whitespace-pre-wrap text-xs text-zinc-300">{slice.note}</p>}
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={() => void player.copyReferenceLink(slice)}
                  className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-zinc-300 transition hover:bg-white/10 hover:text-white"
                >
                  <Copy size={13} aria-hidden="true" />
                  Copy Markdown link
                </button>
                <button
                  type="button"
                  onClick={() => player.editReference(slice)}
                  className="rounded-md px-2 py-1 text-xs text-zinc-300 transition hover:bg-white/10 hover:text-white"
                >
                  Edit
                </button>
                <button
                  type="button"
                  onClick={() => handleDelete(slice)}
                  aria-label={`Delete ${slice.title}`}
                  className="ml-auto rounded-md p-1 text-zinc-400 transition hover:bg-rose-500/15 hover:text-rose-300"
                >
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </div>
            </article>
          ))}
        </div>
      </aside>

      <footer className="absolute bottom-3 left-3 right-3 z-20 rounded-xl border border-white/10 bg-[#111319]/90 p-3 shadow-xl backdrop-blur-md">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => void player.seekBy(-5)}
            disabled={!player.source}
            aria-label="Back five seconds"
            title="Back 5 seconds (Left Arrow)"
            className="rounded-lg p-2 transition hover:bg-white/10 disabled:opacity-40"
          >
            <SkipBack size={17} />
          </button>
          <button
            type="button"
            onClick={() => void player.togglePlayback()}
            disabled={!player.source || player.isBusy}
            aria-label={player.isPlaying ? "Pause" : "Play"}
            title="Play or pause (Space)"
            className="rounded-lg bg-violet-600 p-2.5 transition hover:bg-violet-500 disabled:opacity-40"
          >
            {player.isPlaying ? <Pause size={18} /> : <Play size={18} />}
          </button>
          <button
            type="button"
            onClick={() => void player.seekBy(5)}
            disabled={!player.source}
            aria-label="Forward five seconds"
            title="Forward 5 seconds (Right Arrow)"
            className="rounded-lg p-2 transition hover:bg-white/10 disabled:opacity-40"
          >
            <SkipForward size={17} />
          </button>
          <span className="w-32 shrink-0 text-center font-mono text-xs tabular-nums text-zinc-200">
            {formatMediaTime(player.position)} / {formatMediaTime(player.duration)}
          </span>
          <input
            type="range"
            min={0}
            max={Math.max(player.duration, 1)}
            step={0.01}
            value={Math.min(seekValue, Math.max(player.duration, 1))}
            disabled={!player.source || player.duration <= 0}
            onPointerDown={() => { seekingRef.current = true; }}
            onChange={(event) => setSeekValue(Number(event.currentTarget.value))}
            onPointerUp={() => {
              seekingRef.current = false;
              void player.seekTo(seekValue);
            }}
            onKeyUp={() => {
              seekingRef.current = false;
              void player.seekTo(seekValue);
            }}
            onBlur={() => { seekingRef.current = false; }}
            aria-label="Seek through media"
            className="min-w-20 flex-1 accent-violet-500 disabled:opacity-40"
          />
          <span className="w-12 shrink-0 text-center text-xs tabular-nums text-zinc-300">
            {player.speed.toFixed(2)}x
          </span>
          <button
            type="button"
            onClick={() => void player.changeSpeed(player.speed >= 2 ? 0.5 : player.speed + 0.25)}
            disabled={!player.source}
            title="Change playback speed"
            className="rounded-lg px-2 py-2 text-xs transition hover:bg-white/10 disabled:opacity-40"
          >
            Speed
          </button>
          <Volume2 size={16} className="shrink-0 text-zinc-300" aria-hidden="true" />
          <input
            type="range"
            min={0}
            max={100}
            step={1}
            value={player.volume}
            disabled={!player.source}
            onChange={(event) => void player.changeVolume(Number(event.currentTarget.value))}
            aria-label="Volume"
            className="w-20 accent-violet-500 disabled:opacity-40"
          />
          <button
            type="button"
            onClick={() => void player.toggleFullscreen()}
            title={player.isFullscreen ? "Exit fullscreen" : "Fullscreen (F)"}
            aria-label={player.isFullscreen ? "Exit fullscreen" : "Fullscreen"}
            className="rounded-lg p-2 transition hover:bg-white/10"
          >
            <Maximize size={17} />
          </button>
        </div>
        {(player.error || player.status) && (
          <p
            role={player.error ? "alert" : "status"}
            className={`mt-2 text-xs ${player.error ? "text-rose-300" : "text-emerald-300"}`}
          >
            {player.error || player.status}
          </p>
        )}
        {player.linkPreview && (
          <div className="mt-2 flex gap-2">
            <input
              type="text"
              readOnly
              value={player.linkPreview}
              aria-label="Markdown media link"
              onFocus={(event) => event.currentTarget.select()}
              className="min-w-0 flex-1 rounded-md border border-white/10 bg-black/30 px-2 py-1 font-mono text-[11px] text-zinc-300"
            />
            <button
              type="button"
              onClick={() => void player.copyMarkdownLink(player.linkPreview ?? "")}
              className="rounded-md px-2 text-xs text-zinc-300 transition hover:bg-white/10"
            >
              Copy
            </button>
          </div>
        )}
      </footer>

      {player.draft && (
        <div className="absolute inset-0 z-40 flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <section
            role="dialog"
            aria-modal="true"
            aria-labelledby="media-reference-title"
            className="w-full max-w-lg rounded-xl border border-white/10 bg-[#17191f] p-5 shadow-2xl"
          >
            <div className="mb-4 flex items-center justify-between">
              <h2 id="media-reference-title" className="font-semibold">
                {player.draft.id ? "Edit time reference" : "New time reference"}
              </h2>
              <button
                type="button"
                onClick={() => player.setDraft(null)}
                aria-label="Close reference editor"
                className="rounded-md p-1 text-zinc-400 hover:bg-white/10 hover:text-white"
              >
                <X size={17} />
              </button>
            </div>
            <label className="mb-3 block text-sm">
              Title
              <input
                value={player.draft.title}
                onChange={(event) => player.setDraft((draft) => draft ? { ...draft, title: event.currentTarget.value } : draft)}
                className="mt-1 w-full rounded-md border border-white/10 bg-black/25 px-3 py-2"
                maxLength={200}
              />
            </label>
            <div className="mb-3 grid grid-cols-2 gap-3">
              <label className="text-sm">
                Start (seconds)
                <input
                  type="number"
                  min={0}
                  step="0.001"
                  value={player.draft.start_seconds}
                  onChange={(event) => player.setDraft((draft) => draft ? { ...draft, start_seconds: Number(event.currentTarget.value) } : draft)}
                  className="mt-1 w-full rounded-md border border-white/10 bg-black/25 px-3 py-2 font-mono"
                />
              </label>
              <label className="text-sm">
                End (seconds)
                <input
                  type="number"
                  min={0}
                  step="0.001"
                  value={player.draft.end_seconds}
                  onChange={(event) => player.setDraft((draft) => draft ? { ...draft, end_seconds: Number(event.currentTarget.value) } : draft)}
                  className="mt-1 w-full rounded-md border border-white/10 bg-black/25 px-3 py-2 font-mono"
                />
              </label>
            </div>
            <label className="block text-sm">
              Note
              <textarea
                value={player.draft.note}
                onChange={(event) => player.setDraft((draft) => draft ? { ...draft, note: event.currentTarget.value } : draft)}
                rows={4}
                maxLength={10000}
                className="mt-1 w-full resize-y rounded-md border border-white/10 bg-black/25 px-3 py-2"
              />
            </label>
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => player.setDraft(null)}
                className="rounded-md px-3 py-2 text-sm text-zinc-300 hover:bg-white/10"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={saveDraft}
                disabled={player.isBusy}
                className="rounded-md bg-violet-600 px-3 py-2 text-sm font-medium hover:bg-violet-500 disabled:opacity-50"
              >
                Save reference
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
