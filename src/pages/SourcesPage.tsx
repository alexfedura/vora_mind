import React, { useState, useEffect, useCallback } from "react";
import { Link } from "react-router-dom";
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

  const loadSources = useCallback(async (selectId?: string) => {
    setLoading(true);
    setError(null);
    try {
      const list = await srcListSources();
      setSources(list);
      if (selectId) {
        setSelectedSourceId(selectId);
      } else if (!selectedSourceId && list.length > 0) {
        setSelectedSourceId(list[0].id);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [selectedSourceId]);

  const loadBundle = useCallback(async (sourceId: string) => {
    try {
      const b = await srcGetSourceBundle(sourceId);
      setBundle(b);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    loadSources();
  }, [loadSources]);

  useEffect(() => {
    if (selectedSourceId) {
      loadBundle(selectedSourceId);
      setSelectedSegmentId(null);
    } else {
      setBundle(null);
      setSelectedSegmentId(null);
    }
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
