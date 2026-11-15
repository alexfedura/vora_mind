import React, { useState, useEffect, useCallback, useRef } from "react";
import { Link, useSearchParams } from "react-router-dom";
import {
  srcListSources,
  srcGetSourceBundle,
  SourceSummary,
  SourceBundle,
} from "@/source/services/sourceService";
import { SourceMasterList } from "./sources/SourceMasterList";
import { AddSourceModal } from "./sources/AddSourceModal";
import { AddSegmentModal } from "./sources/AddSegmentModal";
import { SourceDetailView } from "./sources/SourceDetailView";
import { ArrowLeft, Plus, RefreshCw, AlertCircle } from "lucide-react";

export const SourcesPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const requestedSourceId = searchParams.get("sourceId");
  const requestedVersionId = searchParams.get("sourceVersionId");
  const requestedLocationId = searchParams.get("sourceLocationId");
  const requestedClaimId = searchParams.get("sourceClaimId");
  const [sources, setSources] = useState<SourceSummary[]>([]);
  const [selectedSourceId, setSelectedSourceId] = useState<string | null>(null);
  const [selectedSegmentId, setSelectedSegmentId] = useState<string | null>(null);
  const [bundle, setBundle] = useState<SourceBundle | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const [showAddSource, setShowAddSource] = useState(false);
  const [showAddSegment, setShowAddSegment] = useState(false);
  const [targetParentSegmentId, setTargetParentSegmentId] = useState<string | null>(null);
  const selectedSourceIdRef = useRef(selectedSourceId);
  const appliedRouteSourceIdRef = useRef<string | null | undefined>(undefined);
  const sourceListRequestRef = useRef(0);
  const bundleRequestRef = useRef(0);

  const loadSources = useCallback(async (selectId?: string) => {
    const requestId = ++sourceListRequestRef.current;
    setLoading(true);
    setError(null);
    try {
      const list = await srcListSources();
      if (requestId !== sourceListRequestRef.current) return;
      setSources(list);
      if (selectId) {
        if (list.some((source) => source.id === selectId)) {
          setSelectedSourceId(selectId);
        } else {
          setError(`Source ${selectId} is not available in the shared Source Browser.`);
        }
      } else if (appliedRouteSourceIdRef.current !== requestedSourceId) {
        appliedRouteSourceIdRef.current = requestedSourceId;
        if (requestedSourceId) {
          if (list.some((source) => source.id === requestedSourceId)) {
            setSelectedSourceId(requestedSourceId);
          } else {
            setSelectedSourceId(null);
            setError(`Requested Source ${requestedSourceId} is not available in the shared Source Browser.`);
          }
        } else if (!selectedSourceIdRef.current && list.length > 0) {
          setSelectedSourceId(list[0].id);
        }
      } else if (
        selectedSourceIdRef.current
        && !list.some((source) => source.id === selectedSourceIdRef.current)
      ) {
        setSelectedSourceId(list[0]?.id ?? null);
      }
      if (
        requestedSourceId
        && (selectId ?? selectedSourceIdRef.current) === requestedSourceId
        && !requestedVersionId
        && (requestedLocationId || requestedClaimId)
      ) {
        setError("An exact SourceVersion ID is required to navigate to a SourceLocation or SourceClaim. Those requested targets were not selected.");
      }
    } catch (err) {
      if (requestId === sourceListRequestRef.current) setError(String(err));
    } finally {
      if (requestId === sourceListRequestRef.current) setLoading(false);
    }
  }, [requestedClaimId, requestedLocationId, requestedSourceId, requestedVersionId]);

  const loadBundle = useCallback(async (sourceId: string) => {
    const requestId = ++bundleRequestRef.current;
    try {
      const b = await srcGetSourceBundle(sourceId);
      if (requestId === bundleRequestRef.current) setBundle(b);
    } catch (err) {
      if (requestId === bundleRequestRef.current) setError(String(err));
    }
  }, []);

  useEffect(() => {
    selectedSourceIdRef.current = selectedSourceId;
  }, [selectedSourceId]);

  useEffect(() => () => {
    sourceListRequestRef.current += 1;
    bundleRequestRef.current += 1;
  }, []);

  useEffect(() => {
    loadSources();
  }, [loadSources]);

  useEffect(() => {
    if (selectedSourceId) {
      setBundle(null);
      loadBundle(selectedSourceId);
      setSelectedSegmentId(null);
    } else {
      setBundle(null);
      setSelectedSegmentId(null);
    }
    return () => {
      bundleRequestRef.current += 1;
    };
  }, [selectedSourceId, loadBundle]);

  const handleSourceCreated = (newId: string) => {
    loadSources(newId);
  };

  const handleSegmentCreated = () => {
    if (selectedSourceId) {
      loadBundle(selectedSourceId);
    }
  };

  const handleOpenAddSegment = (parentId?: string | null) => {
    setTargetParentSegmentId(parentId || null);
    setShowAddSegment(true);
  };

  const currentSource = bundle?.source;

  return (
    <div className="vora-mind-theme flex flex-col h-screen w-screen bg-background text-gray-200 overflow-hidden font-sans select-none">
      <header className="h-14 px-6 border-b border-[#2a2a2a] bg-background flex items-center justify-between shrink-0">
        <div className="flex items-center gap-4">
          <Link
            to="/"
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-[#181b23] hover:bg-[#202532] text-xs font-medium text-gray-300 hover:text-white transition border border-[#262b3a]"
          >
            <ArrowLeft size={14} />
            <span>Canvas</span>
          </Link>
          <div className="h-4 w-px bg-[#262b3a]" />
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 rounded-lg bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center font-bold text-white text-xs shadow-md">
              S
            </div>
            <span className="font-bold text-sm text-gray-100 tracking-wider">
              SOURCE BROWSER
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2.5">
          <button
            onClick={() => loadSources(selectedSourceId || undefined)}
            disabled={loading}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#181b23] hover:bg-[#202532] text-xs font-medium text-gray-300 hover:text-white transition border border-[#262b3a] disabled:opacity-50"
            title="Refresh Sources"
          >
            <RefreshCw size={13} className={loading ? "animate-spin" : ""} />
            <span>Refresh</span>
          </button>
          <button
            onClick={() => setShowAddSource(true)}
            className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-amber-500 hover:bg-amber-400 text-black font-semibold text-xs transition shadow-md shadow-amber-500/20"
          >
            <Plus size={14} />
            <span>Add Source</span>
          </button>
        </div>
      </header>

      {error && (
        <div className="mx-6 mt-3 p-3 rounded-lg bg-rose-950/40 border border-rose-800 text-rose-300 text-xs flex items-center justify-between shrink-0">
          <div className="flex items-center gap-2">
            <AlertCircle size={15} />
            <span>{error}</span>
          </div>
          <button onClick={() => setError(null)} className="text-xs hover:text-white underline ml-4">
            Dismiss
          </button>
        </div>
      )}

      <div className="flex-1 flex min-h-0 overflow-hidden">
        <SourceMasterList
          sources={sources}
          selectedSourceId={selectedSourceId}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          onSelectSource={(id) => setSelectedSourceId(id)}
        />

        <main className="flex-1 flex flex-col min-h-0 bg-background overflow-y-auto">
          <SourceDetailView
            bundle={bundle}
            requestedVersionId={selectedSourceId === requestedSourceId ? requestedVersionId : null}
            requestedLocationId={selectedSourceId === requestedSourceId && requestedVersionId ? requestedLocationId : null}
            requestedClaimId={selectedSourceId === requestedSourceId && requestedVersionId ? requestedClaimId : null}
            selectedSegmentId={selectedSegmentId}
            onSelectSegment={(id) => setSelectedSegmentId(id)}
            onOpenAddSegment={handleOpenAddSegment}
          />
        </main>
      </div>

      <AddSourceModal
        isOpen={showAddSource}
        onClose={() => setShowAddSource(false)}
        onSourceCreated={handleSourceCreated}
      />

      {currentSource && (
        <AddSegmentModal
          isOpen={showAddSegment}
          sourceId={currentSource.id}
          sourceKind={currentSource.kind}
          parentSegmentId={targetParentSegmentId}
          onClose={() => setShowAddSegment(false)}
          onSegmentCreated={handleSegmentCreated}
        />
      )}
    </div>
  );
};
