import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { dbService } from "@/mind/services/dbService";
import type {
  MediaLinkPayload,
  MediaSlice,
  MediaSliceInput,
  MediaSource,
} from "@/mind/types";
import { createMediaMarkdownLink, formatMediaTime } from "@/mind/utils/mediaLinks";

export interface MediaPlayerRequest {
  requestId: number;
  payload: MediaLinkPayload | null;
  error: string | null;
}

export interface MediaPlayerDraft {
  id?: string;
  title: string;
  start_seconds: number;
  end_seconds: number;
  note: string;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function sourceName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

function matchesMediaLink(
  payload: MediaLinkPayload,
  slice: MediaSlice,
  source: MediaSource,
): boolean {
  return (
    payload.sourceId === source.id &&
    payload.sliceId === slice.id &&
    payload.startSeconds === slice.start_seconds &&
    payload.endSeconds === slice.end_seconds &&
    payload.path === source.path &&
    payload.fingerprintSize === source.fingerprint_size &&
    payload.fingerprintModifiedMs === source.fingerprint_modified_ms &&
    slice.source_id === source.id
  );
}

export function useMediaPlayer(
  active: boolean,
  request: MediaPlayerRequest | null,
) {
  const [source, setSource] = useState<MediaSource | null>(null);
  const [lastSource, setLastSource] = useState<MediaSource | null>(null);
  const [slices, setSlices] = useState<MediaSlice[]>([]);
  const [position, setPosition] = useState(0);
  const [duration, setDuration] = useState(0);
  const [speed, setSpeed] = useState(1);
  const [volume, setVolume] = useState(100);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isReady, setIsReady] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [draft, setDraft] = useState<MediaPlayerDraft | null>(null);
  const [linkPreview, setLinkPreview] = useState<string | null>(null);
  const sourceRef = useRef(source);
  const positionRef = useRef(position);
  const latestRequestId = useRef(0);
  const processedRequestId = useRef(0);
  const openQueue = useRef<Promise<void>>(Promise.resolve());

  sourceRef.current = source;
  positionRef.current = position;

  useEffect(() => {
    let cancelled = false;
    void dbService.getLastMediaSource().then((storedSource) => {
      if (!cancelled) setLastSource(storedSource);
    }).catch((loadError: unknown) => {
      if (!cancelled) setError(`Could not load the last media source: ${errorMessage(loadError)}`);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!active) {
      setIsReady(false);
      setIsPlaying(false);
      return;
    }

    let cancelled = false;
    const unlisteners: Array<() => void> = [];
    void listen<number>("mind-player-position", (event) => {
      setPosition(event.payload);
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    }).catch((listenError: unknown) => {
      if (!cancelled) setError(`Could not receive player position updates: ${errorMessage(listenError)}`);
    });
    void listen<number>("mind-player-duration", (event) => {
      setDuration(event.payload);
      const currentSource = sourceRef.current;
      if (!currentSource || !Number.isFinite(event.payload) || event.payload <= 0) return;
      void dbService.updateMediaSourceDuration(currentSource.id, event.payload).catch((saveError: unknown) => {
        setError(`Could not save media duration: ${errorMessage(saveError)}`);
      });
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    }).catch((listenError: unknown) => {
      if (!cancelled) setError(`Could not receive player duration updates: ${errorMessage(listenError)}`);
    });
    void listen<boolean>("mind-player-paused", (event) => {
      setIsPlaying(!event.payload);
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    }).catch((listenError: unknown) => {
      if (!cancelled) setError(`Could not receive player playback updates: ${errorMessage(listenError)}`);
    });
    void listen<boolean>("mind-player-fullscreen", (event) => {
      setIsFullscreen(event.payload);
      void getCurrentWindow().setFullscreen(event.payload).catch((fullscreenError: unknown) => {
        setError(`Could not update the player window fullscreen state: ${errorMessage(fullscreenError)}`);
      });
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    }).catch((listenError: unknown) => {
      if (!cancelled) setError(`Could not receive player fullscreen updates: ${errorMessage(listenError)}`);
    });
    void dbService.playerSpawn().then(() => {
      if (!cancelled) setIsReady(true);
    }).catch((spawnError: unknown) => {
      if (!cancelled) setError(`Could not start the bundled MPV player: ${errorMessage(spawnError)}`);
    });

    return () => {
      cancelled = true;
      unlisteners.forEach((unlisten) => unlisten());
      const currentSource = sourceRef.current;
      if (currentSource) {
        void dbService.saveMediaSourcePosition(currentSource.id, positionRef.current).catch((saveError: unknown) => {
          setError(`Could not save playback position: ${errorMessage(saveError)}`);
        });
      }
      void dbService.playerPause().catch((pauseError: unknown) => {
        setError(`Could not pause the player: ${errorMessage(pauseError)}`);
      });
    };
  }, [active]);

  useEffect(() => {
    if (!active || !isReady || !request || request.requestId === processedRequestId.current) return;
    latestRequestId.current = request.requestId;
    processedRequestId.current = request.requestId;
    openQueue.current = openQueue.current.then(async () => {
      if (latestRequestId.current !== request.requestId) return;
      setError(request.error);
      setStatus(null);
      if (request.error || !request.payload) return;

      setIsBusy(true);
      try {
        const media = await dbService.getMediaSlice(request.payload.sliceId);
        if (!matchesMediaLink(request.payload, media.slice, media.source)) {
          throw new Error("The media link no longer matches its saved source fingerprint or time range.");
        }
        const verifiedSource = await dbService.validateMediaSource(request.payload.sourceId);
        if (latestRequestId.current !== request.requestId) return;
        await dbService.playerLoadAt(verifiedSource.path, media.slice.start_seconds);
        if (latestRequestId.current !== request.requestId) return;
        setSource(verifiedSource);
        setLastSource(verifiedSource);
        setSlices(await dbService.listMediaSlices(verifiedSource.id));
        setDuration(verifiedSource.duration_seconds ?? 0);
        setPosition(media.slice.start_seconds);
        setIsPlaying(false);
        setStatus(`Opened ${media.slice.title} at ${formatMediaTime(media.slice.start_seconds)}.`);
      } catch (openError: unknown) {
        if (latestRequestId.current === request.requestId) {
          setError(`Could not open this media reference: ${errorMessage(openError)}`);
        }
      } finally {
        if (latestRequestId.current === request.requestId) setIsBusy(false);
      }
    }).catch((queueError: unknown) => {
      if (latestRequestId.current === request.requestId) {
        setIsBusy(false);
        setError(`Could not process the media reference: ${errorMessage(queueError)}`);
      }
    });
  }, [active, isReady, request]);

  useEffect(() => {
    if (!active || !source) return;
    const timer = window.setTimeout(() => {
      void dbService.saveMediaSourcePosition(source.id, position).catch((saveError: unknown) => {
        setError(`Could not save playback position: ${errorMessage(saveError)}`);
      });
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [active, position, source]);

  const run = async (operation: () => Promise<void>): Promise<void> => {
    setError(null);
    setStatus(null);
    try {
      await operation();
    } catch (operationError: unknown) {
      setError(errorMessage(operationError));
    }
  };

  const loadSource = async (mediaSource: MediaSource, startSeconds: number): Promise<void> => {
    const verifiedSource = await dbService.validateMediaSource(mediaSource.id);
    await dbService.playerLoadAt(verifiedSource.path, startSeconds);
    setSource(verifiedSource);
    setLastSource(verifiedSource);
    setSlices(await dbService.listMediaSlices(verifiedSource.id));
    setPosition(startSeconds);
    setDuration(verifiedSource.duration_seconds ?? 0);
    setIsPlaying(false);
  };

  const openMediaFile = async (): Promise<void> => {
    if (!isReady) {
      setError("The player is still starting. Try opening the file again in a moment.");
      return;
    }
    const selected = await open({
      multiple: false,
      directory: false,
    });
    if (typeof selected !== "string") return;
    setIsBusy(true);
    setError(null);
    setStatus(null);
    try {
      const registered = await dbService.registerMediaSource(selected);
      await loadSource(registered, 0);
      setStatus(`Opened ${sourceName(registered.path)}.`);
    } catch (openError: unknown) {
      setError(`Could not open the selected media file: ${errorMessage(openError)}`);
    } finally {
      setIsBusy(false);
    }
  };

  const resumeLastSource = async (): Promise<void> => {
    if (!lastSource) return;
    setIsBusy(true);
    setError(null);
    setStatus(null);
    try {
      await loadSource(lastSource, lastSource.last_position_seconds);
      setStatus(`Resumed ${sourceName(lastSource.path)}.`);
    } catch (resumeError: unknown) {
      setError(`Could not resume the last source: ${errorMessage(resumeError)}`);
    } finally {
      setIsBusy(false);
    }
  };

  const togglePlayback = async (): Promise<void> => {
    if (!source) return;
    await run(async () => {
      if (isPlaying) await dbService.playerPause();
      else await dbService.playerPlay();
      setIsPlaying(!isPlaying);
    });
  };

  const seekTo = async (seconds: number): Promise<void> => {
    if (!source || !Number.isFinite(seconds)) return;
    const bounded = Math.max(0, duration > 0 ? Math.min(seconds, duration) : seconds);
    await run(async () => {
      await dbService.playerSeek(bounded);
      setPosition(bounded);
    });
  };

  const seekBy = async (delta: number): Promise<void> => {
    await seekTo(position + delta);
  };

  const changeSpeed = async (nextSpeed: number): Promise<void> => {
    const bounded = Math.min(4, Math.max(0.25, nextSpeed));
    await run(async () => {
      await dbService.playerSetSpeed(bounded);
      setSpeed(bounded);
    });
  };

  const changeVolume = async (nextVolume: number): Promise<void> => {
    const bounded = Math.min(100, Math.max(0, nextVolume));
    await run(async () => {
      await dbService.playerSetVolume(bounded);
      setVolume(bounded);
    });
  };

  const toggleFullscreen = async (): Promise<void> => {
    await run(async () => {
      const fullscreen = !isFullscreen;
      await dbService.playerSetFullscreen(fullscreen);
      await getCurrentWindow().setFullscreen(fullscreen);
      setIsFullscreen(fullscreen);
    });
  };

  const startNewReference = (): void => {
    if (!source) {
      setError("Open an audio or video source before creating a time reference.");
      return;
    }
    const end = duration > position ? Math.min(duration, position + 5) : position + 5;
    setDraft({
      title: `Reference at ${formatMediaTime(position)}`,
      start_seconds: position,
      end_seconds: end,
      note: "",
    });
    setError(null);
  };

  const editReference = (slice: MediaSlice): void => {
    setDraft({
      id: slice.id,
      title: slice.title,
      start_seconds: slice.start_seconds,
      end_seconds: slice.end_seconds,
      note: slice.note,
    });
    setError(null);
  };

  const saveReference = async (): Promise<void> => {
    if (!source || !draft) return;
    setIsBusy(true);
    setError(null);
    try {
      const saved = await dbService.saveMediaSlice({
        ...draft,
        id: draft.id,
        source_id: source.id,
      } satisfies MediaSliceInput);
      setSlices(await dbService.listMediaSlices(source.id));
      setDraft(null);
      setStatus(`Saved ${saved.title}. The original media file was not changed.`);
    } catch (saveError: unknown) {
      setError(`Could not save the time reference: ${errorMessage(saveError)}`);
    } finally {
      setIsBusy(false);
    }
  };

  const deleteReference = async (sliceId: string): Promise<void> => {
    setIsBusy(true);
    setError(null);
    try {
      await dbService.deleteMediaSlice(sliceId);
      if (source) setSlices(await dbService.listMediaSlices(source.id));
      setStatus("Time reference deleted. The media file was not changed.");
    } catch (deleteError: unknown) {
      setError(`Could not delete the time reference: ${errorMessage(deleteError)}`);
    } finally {
      setIsBusy(false);
    }
  };

  const copyReferenceLink = async (slice: MediaSlice): Promise<void> => {
    if (!source) return;
    const markdown = createMediaMarkdownLink(slice, source);
    setLinkPreview(markdown);
    await copyMarkdownLink(markdown);
  };

  const copyMarkdownLink = async (markdown: string): Promise<void> => {
    setError(null);
    try {
      await navigator.clipboard.writeText(markdown);
      setStatus("Markdown media link copied. Paste it into any Board card.");
    } catch (copyError: unknown) {
      setError(`Could not copy the media link: ${errorMessage(copyError)}. Copy it from the field below.`);
    }
  };

  return {
    source,
    lastSource,
    slices,
    position,
    duration,
    speed,
    volume,
    isPlaying,
    isReady,
    isFullscreen,
    isBusy,
    error,
    status,
    draft,
    linkPreview,
    openMediaFile,
    resumeLastSource,
    togglePlayback,
    seekTo,
    seekBy,
    changeSpeed,
    changeVolume,
    toggleFullscreen,
    startNewReference,
    editReference,
    saveReference,
    deleteReference,
    copyReferenceLink,
    copyMarkdownLink,
    setDraft,
  };
}
