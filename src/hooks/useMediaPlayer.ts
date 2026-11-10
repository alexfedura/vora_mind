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
  VoraMediaData,
} from "@/mind/types";
import { createMediaMarkdownLink, formatMediaTime } from "@/mind/utils/mediaLinks";

export interface MediaPlayerRequest {
  requestId: number;
  payload: MediaLinkPayload | null;
  fileId: string | null;
  filePath: string | null;
  mediaType: VoraMediaData["type"] | null;
  startSeconds: number | null;
  error: string | null;
}

interface MediaPlaybackRange {
  startSeconds: number;
  endSeconds: number;
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
    slice.file_id === source.id
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
  const [playbackRange, setPlaybackRange] = useState<MediaPlaybackRange | null>(null);
  const [rangeCheckVersion, setRangeCheckVersion] = useState(0);
  const sourceRef = useRef(source);
  const positionRef = useRef(position);
  const playbackRangeRef = useRef(playbackRange);
  const isPlayingRef = useRef(isPlaying);
  const rangeCheckIntervalRef = useRef<number | null>(null);
  const latestRequestId = useRef(0);
  const processedRequestId = useRef(0);
  const openQueue = useRef<Promise<void>>(Promise.resolve());

  sourceRef.current = source;
  positionRef.current = position;
  playbackRangeRef.current = playbackRange;
  isPlayingRef.current = isPlaying;

  useEffect(() => {
    if (rangeCheckIntervalRef.current !== null) {
      window.clearInterval(rangeCheckIntervalRef.current);
      rangeCheckIntervalRef.current = null;
    }
    if (!active || !isPlaying || !playbackRange) return;

    let cancelled = false;
    let checking = false;
    const endSeconds = playbackRange.endSeconds;
    const intervalId = window.setInterval(async () => {
      if (checking) return;
      checking = true;
      try {
        const currentTime = await dbService.playerGetPosition();
        if (
          cancelled ||
          rangeCheckIntervalRef.current !== intervalId ||
          !isPlayingRef.current ||
          playbackRangeRef.current?.endSeconds !== endSeconds
        ) return;
        if (currentTime < endSeconds) return;

        window.clearInterval(intervalId);
        rangeCheckIntervalRef.current = null;
        await dbService.playerPause();
        await dbService.playerSeek(endSeconds);
        if (playbackRangeRef.current?.endSeconds === endSeconds) {
          setPosition(endSeconds);
          setIsPlaying(false);
        }
      } catch (rangeError: unknown) {
        if (!cancelled) setError(`Could not enforce the media reference end time: ${errorMessage(rangeError)}`);
        window.clearInterval(intervalId);
        if (rangeCheckIntervalRef.current === intervalId) rangeCheckIntervalRef.current = null;
      } finally {
        checking = false;
      }
    }, 500);
    rangeCheckIntervalRef.current = intervalId;

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
      if (rangeCheckIntervalRef.current === intervalId) rangeCheckIntervalRef.current = null;
    };

  }, [active, isPlaying, playbackRange, rangeCheckVersion]);

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
      void (async () => {
        try {
          await getCurrentWindow().setFullscreen(event.payload);
          setIsFullscreen(event.payload);
        } catch (fullscreenError: unknown) {
          console.error("Could not update the player window fullscreen state:", fullscreenError);
        }
      })();
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    }).catch((listenError: unknown) => {
      if (!cancelled) setError(`Could not receive player fullscreen updates: ${errorMessage(listenError)}`);
    });
    void dbService.playerSpawn().then(() => {
      if (!cancelled) setIsReady(true);
    }).catch((spawnError: unknown) => {
      console.error("Could not start the bundled MPV player:", spawnError);
      if (!cancelled) {
        setError("Video player failed to start. Please check the logs.");
      }
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
      if (request.error) return;

      setIsBusy(true);
      try {
        if (request.fileId) {
          if (request.mediaType !== "audio" && request.mediaType !== "video") {
            throw new Error("Vora Player supports audio and video sources, not image media.");
          }
          const storedSource = await dbService.getMediaSource(request.fileId);
          if (latestRequestId.current !== request.requestId) return;
          setSource(storedSource);
          const verifiedSource = await dbService.validateMediaSource(storedSource.id);
          if (latestRequestId.current !== request.requestId) return;
          const startSeconds = request.startSeconds ?? 0;
          await dbService.playerLoadAt(verifiedSource.path, startSeconds);
          if (latestRequestId.current !== request.requestId) return;
          await dbService.saveMediaSourcePosition(verifiedSource.id, startSeconds);
          setSource(verifiedSource);
          setLastSource(verifiedSource);
          setSlices(await dbService.listMediaSlices(verifiedSource.id));
          setDuration(verifiedSource.duration_seconds ?? 0);
          setPosition(startSeconds);
          setPlaybackRange(null);
          setIsPlaying(false);
          setStatus(`Opened ${sourceName(verifiedSource.path)}.`);
          return;
        }
        if (request.filePath) {
          if (request.mediaType !== "audio" && request.mediaType !== "video") {
            throw new Error("Vora Player supports audio and video sources, not image media.");
          }
          const registeredSource = await dbService.registerMediaSource(request.filePath);
          if (latestRequestId.current !== request.requestId) return;
          const verifiedSource = await dbService.validateMediaSource(registeredSource.id);
          if (latestRequestId.current !== request.requestId) return;
          await dbService.playerLoadAt(verifiedSource.path, 0);
          if (latestRequestId.current !== request.requestId) return;
          setSource(verifiedSource);
          setLastSource(verifiedSource);
          setSlices(await dbService.listMediaSlices(verifiedSource.id));
          setDuration(verifiedSource.duration_seconds ?? 0);
          setPosition(0);
          setPlaybackRange(null);
          setIsPlaying(false);
          setStatus(`Opened ${sourceName(verifiedSource.path)}.`);
          return;
        }
        if (!request.payload) return;
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
        setPlaybackRange({
          startSeconds: media.slice.start_seconds,
          endSeconds: media.slice.end_seconds,
        });
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
    setPlaybackRange(null);
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

  const locateMissingFile = async (): Promise<void> => {
    if (!source) return;
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected !== "string") return;
      setIsBusy(true);
      setError(null);
      setStatus(null);
      const resolvedPath = await dbService.resolveMissingFile(source.id, selected);
      const resolvedSource = await dbService.validateMediaSource(source.id);
      await dbService.playerLoadAt(resolvedPath, positionRef.current);
      setSource(resolvedSource);
      setLastSource(resolvedSource);
      setSlices(await dbService.listMediaSlices(resolvedSource.id));
      setStatus(`Located ${sourceName(resolvedPath)}.`);
      window.dispatchEvent(new CustomEvent("vora:file-reference-updated", {
        detail: { fileId: resolvedSource.id },
      }));
    } catch (locateError: unknown) {
      setError(`Could not locate the media file: ${errorMessage(locateError)}`);
    } finally {
      setIsBusy(false);
    }
  };

  const togglePlayback = async (): Promise<void> => {
    if (!source) return;
    await run(async () => {
      if (isPlaying) {
        if (rangeCheckIntervalRef.current !== null) {
          window.clearInterval(rangeCheckIntervalRef.current);
          rangeCheckIntervalRef.current = null;
        }
        try {
          await dbService.playerPause();
          setIsPlaying(false);
        } finally {
          setRangeCheckVersion((version) => version + 1);
        }
      } else {
        await dbService.playerPlay();
        setIsPlaying(true);
      }
    });
  };

  const seekTo = async (seconds: number, rangeOverride?: MediaPlaybackRange): Promise<void> => {
    if (!source || !Number.isFinite(seconds)) return;
    const range = rangeOverride ?? playbackRange;
    const sourceBounded = Math.max(0, duration > 0 ? Math.min(seconds, duration) : seconds);
    const bounded = range
      ? Math.min(range.endSeconds, Math.max(range.startSeconds, sourceBounded))
      : sourceBounded;
    await run(async () => {
      if (rangeCheckIntervalRef.current !== null) {
        window.clearInterval(rangeCheckIntervalRef.current);
        rangeCheckIntervalRef.current = null;
      }
      try {
        await dbService.playerSeek(bounded);
        setPosition(bounded);
      } finally {
        setRangeCheckVersion((version) => version + 1);
      }
    });
  };

  const selectReference = async (slice: MediaSlice): Promise<void> => {
    const range = {
      startSeconds: slice.start_seconds,
      endSeconds: slice.end_seconds,
    };
    setPlaybackRange(range);
    await seekTo(slice.start_seconds, range);
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
      try {
        await getCurrentWindow().setFullscreen(fullscreen);
      } catch (fullscreenError: unknown) {
        console.error("Could not update the player window fullscreen state:", fullscreenError);
        return;
      }
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
        file_id: source.id,
      } satisfies MediaSliceInput);
      setSlices(await dbService.listMediaSlices(source.id));
      window.dispatchEvent(new CustomEvent("vora:media-slices-changed", {
        detail: { fileId: source.id },
      }));
      const savedRange = {
        startSeconds: saved.start_seconds,
        endSeconds: saved.end_seconds,
      };
      setPlaybackRange(savedRange);
      await seekTo(saved.start_seconds, savedRange);
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
      if (source) {
        setSlices(await dbService.listMediaSlices(source.id));
        window.dispatchEvent(new CustomEvent("vora:media-slices-changed", {
          detail: { fileId: source.id },
        }));
      }
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
    locateMissingFile,
    togglePlayback,
    seekTo,
    selectReference,
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
