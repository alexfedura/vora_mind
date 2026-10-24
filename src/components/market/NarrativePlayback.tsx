import React, { useEffect, useMemo, useRef, useState } from "react";
import { Pause, Play, SkipBack, SkipForward, X } from "lucide-react";
import type { DrawingAny } from "@/lib/market/lineDrawings";
import type { ActiveScale, CompactStep, NarrativeEvent, NarrativeMode, VoraSnapshot } from "@/lib/snapshot/types";
import { deriveCompactSteps } from "@/lib/snapshot/narrative";

interface Props {
  snapshot: VoraSnapshot;
  baseDrawings: DrawingAny[];
  speedFontSize: number;
  countFontSize: number;
  onApplyDrawings: (drawings: DrawingAny[]) => void;
  onApplyScale: (scale: ActiveScale) => void;
  onClose: () => void;
}

export function NarrativePlayback({ snapshot, baseDrawings, speedFontSize, countFontSize, onApplyDrawings, onApplyScale, onClose }: Props): React.ReactElement | null {
  const fullEvents = useMemo(() => snapshot.narrativeMode === "FULL_VIDEO"
    ? snapshot.narrativeLog?.filter((event): event is NarrativeEvent => "timestamp" in event) ?? []
    : [], [snapshot.narrativeLog, snapshot.narrativeMode]);
  const savedSteps = useMemo(() => snapshot.narrativeMode === "COMPACT_STEPS"
    ? snapshot.narrativeLog?.filter((step): step is CompactStep => "committedDrawing" in step) ?? []
    : [], [snapshot.narrativeLog, snapshot.narrativeMode]);
  const derivedSteps = useMemo(() => snapshot.narrativeMode === "FULL_VIDEO"
    ? deriveCompactSteps(fullEvents)
    : [], [fullEvents, snapshot.narrativeMode]);
  const steps = snapshot.narrativeMode === "COMPACT_STEPS" ? savedSteps : derivedSteps;
  const [displayMode, setDisplayMode] = useState<NarrativeMode | "STANDARD">(snapshot.narrativeMode ?? "STANDARD");
  const [isPlaying, setIsPlaying] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [eventIndex, setEventIndex] = useState(-1);
  const eventIndexRef = useRef(-1);
  const fullEventsRef = useRef(fullEvents);
  const onApplyDrawingsRef = useRef(onApplyDrawings);
  const onApplyScaleRef = useRef(onApplyScale);
  const [currentStepIndex, setCurrentStepIndex] = useState(0);
  const currentStepIndexRef = useRef(0);
  const isFullVideo = displayMode === "FULL_VIDEO";
  const isCompactSteps = displayMode === "COMPACT_STEPS";

  fullEventsRef.current = fullEvents;
  onApplyDrawingsRef.current = onApplyDrawings;
  onApplyScaleRef.current = onApplyScale;

  useEffect(() => {
    if (!isFullVideo || !isPlaying) return;
    const timer = window.setInterval(() => {
      const nextIndex = eventIndexRef.current + 1;
      const event = fullEventsRef.current[nextIndex];
      if (!event) {
        setIsPlaying(false);
        return;
      }
      eventIndexRef.current = nextIndex;
      setEventIndex(nextIndex);
      onApplyDrawingsRef.current(event.drawings);
      onApplyScaleRef.current(event.activeScale);
    }, 50 / speed);
    return () => window.clearInterval(timer);
  }, [isFullVideo, isPlaying, speed]);

  useEffect(() => {
    setIsPlaying(false);
    eventIndexRef.current = -1;
    setEventIndex(-1);
    currentStepIndexRef.current = 0;
    setCurrentStepIndex(0);
    onApplyDrawingsRef.current(baseDrawings);
  }, [baseDrawings, displayMode]);

  const seekFull = (index: number): void => {
    const event = fullEvents[index];
    if (!event) return;
    setIsPlaying(false);
    eventIndexRef.current = index;
    setEventIndex(index);
    onApplyDrawings(event.drawings);
    onApplyScale(event.activeScale);
  };

  const seekCompact = (count: number): void => {
    const target = Math.max(0, Math.min(steps.length, count));
    const next = baseDrawings.map((drawing) => ({ ...drawing }));
    for (const step of steps.slice(0, target)) {
      const index = next.findIndex((drawing) => drawing.id === step.committedDrawing.id);
      if (index >= 0) next[index] = step.committedDrawing;
      else next.push(step.committedDrawing);
    }
    setIsPlaying(false);
    currentStepIndexRef.current = target;
    setCurrentStepIndex(target);
    onApplyDrawings(next);
    const scale = target > 0 ? steps[target - 1]?.activeScale : fullEvents[0]?.activeScale;
    if (scale) onApplyScale(scale);
  };

  const seekCompactRef = useRef(seekCompact);
  seekCompactRef.current = seekCompact;

  useEffect(() => {
    if (!isCompactSteps || !isPlaying) return;
    const timer = window.setInterval(() => {
      const nextIndex = currentStepIndexRef.current + 1;
      if (nextIndex > steps.length) {
        setIsPlaying(false);
        return;
      }
      seekCompactRef.current(nextIndex);
      setIsPlaying(true);
    }, 600 / speed);
    return () => window.clearInterval(timer);
  }, [isCompactSteps, isPlaying, speed, steps.length]);

  const previous = (): void => {
    if (isFullVideo) {
      seekFull(Math.max(0, (eventIndex < 0 ? 0 : eventIndex) - 1));
    } else if (isCompactSteps) {
      seekCompact(currentStepIndex - 1);
    }
  };

  const next = (): void => {
    if (isFullVideo) {
      seekFull(Math.min(fullEvents.length - 1, eventIndex + 1));
    } else if (isCompactSteps) {
      seekCompact(currentStepIndex + 1);
    }
  };

  const togglePlayback = (): void => {
    if (isPlaying) {
      setIsPlaying(false);
      return;
    }
    if (displayMode === "STANDARD") return;
    if (eventIndex >= fullEvents.length - 1) {
      eventIndexRef.current = -1;
      setEventIndex(-1);
    }
    if (isCompactSteps && currentStepIndex >= steps.length) {
      seekCompact(0);
    }
    setIsPlaying(true);
  };

  if (snapshot.narrativeMode !== "FULL_VIDEO" && snapshot.narrativeMode !== "COMPACT_STEPS") {
    return null;
  }

  const total = isFullVideo ? fullEvents.length : isCompactSteps ? steps.length : 0;
  const position = isFullVideo ? Math.max(0, eventIndex) : isCompactSteps ? currentStepIndex : 0;
  const positionLabel = displayMode === "STANDARD"
    ? "Standard"
    : isFullVideo
    ? `${eventIndex < 0 ? 0 : eventIndex + 1} / ${total}`
    : `${currentStepIndex} / ${total}`;

  return (
    <section className="chart-narrative-player" role="group" aria-label="Narrative playback controls">
      <input
        className="chart-narrative-player__seek"
        type="range"
        min={0}
        max={Math.max(0, isFullVideo ? total - 1 : total)}
        value={position}
        disabled={displayMode === "STANDARD" || total === 0}
        aria-label="Narrative position"
        onChange={(event) => isFullVideo
          ? seekFull(Number(event.target.value))
          : isCompactSteps ? seekCompact(Number(event.target.value)) : undefined}
      />
      <div className="chart-narrative-player__controls">
        <div className="chart-narrative-player__selectors">
          <label className="chart-narrative-player__speed">
            <span className="sr-only">Playback speed</span>
            <select
              aria-label="Playback speed"
              value={speed}
              style={{ fontSize: `${speedFontSize}px` }}
              disabled={displayMode === "STANDARD"}
              onChange={(event) => setSpeed(Number(event.target.value))}
            >
              {Array.from({ length: 9 }, (_, index) => (index + 1) / 10).map((value) => (
                <option key={value} value={value}>{value.toFixed(1)}x</option>
              ))}
              <option value={1}>1x</option>
              <option value={2}>2x</option>
            </select>
          </label>
          <label className="chart-narrative-player__mode">
            <span className="sr-only">Display mode</span>
            <select
              aria-label="Display mode"
              value={displayMode}
              onChange={(event) => {
                const value = event.target.value;
                if (value === "STANDARD" || value === "FULL_VIDEO" || value === "COMPACT_STEPS") {
                  setDisplayMode(value);
                }
              }}
            >
              <option value="STANDARD">Standard</option>
              <option value="COMPACT_STEPS">Compact</option>
              <option value="FULL_VIDEO" disabled={snapshot.narrativeMode !== "FULL_VIDEO"}>Full</option>
            </select>
          </label>
        </div>
        <div className="chart-narrative-player__transport">
          <button type="button" onClick={previous} disabled={displayMode === "STANDARD" || total === 0 || (isFullVideo ? eventIndex <= 0 : currentStepIndex === 0)} title="Previous movement" aria-label="Previous movement">
            <SkipBack size={20} />
          </button>
          <button type="button" className="chart-narrative-player__play" onClick={togglePlayback} disabled={displayMode === "STANDARD" || total === 0} title={isPlaying ? "Pause" : "Play"} aria-label={isPlaying ? "Pause" : "Play"}>
            {isPlaying ? <Pause size={22} fill="currentColor" /> : <Play size={22} fill="currentColor" />}
          </button>
          <button type="button" onClick={next} disabled={displayMode === "STANDARD" || total === 0 || (isFullVideo ? eventIndex >= total - 1 : currentStepIndex >= total)} title="Next movement" aria-label="Next movement">
            <SkipForward size={20} />
          </button>
        </div>
        <div className="chart-narrative-player__trailing">
          <div className="chart-narrative-player__meta" style={{ fontSize: `${countFontSize}px` }} aria-live="polite">
            {positionLabel}
          </div>
          <button type="button" className="chart-narrative-player__close" onClick={onClose} title="Close playback" aria-label="Close playback">
            <X size={17} />
          </button>
        </div>
      </div>
    </section>
  );
}