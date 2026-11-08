import React, { useEffect, useMemo, useState } from "react";
import { FileText, FileUp, LoaderCircle, X } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { dbService, type DocumentDto, type DocumentFileType } from "@/mind/services/dbService";
import { useAppStore } from "@/shared/store/useAppStore";
import { PdfViewer } from "./PdfViewer";
import { DocxViewer } from "./DocxViewer";
import { MarkdownViewer } from "./MarkdownViewer";

interface DocumentWorkspaceProps {
  active: boolean;
  initialDocumentId: string | null;
  initialHighlightId: string | null;
  boardId: string | null;
  onClose: () => void;
}

function getFileType(path: string): DocumentFileType | null {
  const parts = path.split(".");
  const extension = parts[parts.length - 1]?.toLowerCase();
  return extension === "pdf" || extension === "docx" || extension === "md" ? extension : null;
}

export const DocumentWorkspace: React.FC<DocumentWorkspaceProps> = ({
  active,
  initialDocumentId,
  initialHighlightId,
  boardId,
  onClose,
}) => {
  const [documents, setDocuments] = useState<DocumentDto[]>([]);
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(initialDocumentId);
  const [jumpToHighlightId, setJumpToHighlightId] = useState<string | null>(initialHighlightId);
  const [isLoading, setIsLoading] = useState(true);
  const [isImporting, setIsImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedDocument = useMemo(
    () => documents.find((item) => item.id === selectedDocumentId) ?? null,
    [documents, selectedDocumentId],
  );

  const reloadDocuments = async (): Promise<void> => {
    setError(null);
    setDocuments(await dbService.getDocuments());
  };

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    setIsLoading(true);
    void dbService.getDocuments().then((items) => {
      if (cancelled) return;
      setDocuments(items);
      setSelectedDocumentId((current) => initialDocumentId ?? current ?? items[0]?.id ?? null);
      setJumpToHighlightId(initialHighlightId);
    }).catch((cause: unknown) => {
      if (!cancelled) {
        console.error("Could not load documents:", cause);
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    }).finally(() => {
      if (!cancelled) setIsLoading(false);
    });
    return () => { cancelled = true; };
  }, [active, initialDocumentId, initialHighlightId]);

  const importDocument = async (): Promise<void> => {
    setIsImporting(true);
    setError(null);
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Documents", extensions: ["pdf", "docx", "md"] }],
      });
      if (typeof selected !== "string") return;
      const fileType = getFileType(selected);
      if (!fileType) throw new Error("Choose a PDF, DOCX, or Markdown file.");
      const newDocumentId = await dbService.importDocument(selected, fileType);
      await reloadDocuments();
      setSelectedDocumentId(newDocumentId);
      setJumpToHighlightId(null);
    } catch (cause) {
      console.error("Could not import document:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsImporting(false);
    }
  };

  const sendHighlightToLibrary = async (highlightId: string, content: string, targetBoardId: string): Promise<void> => {
    if (!selectedDocument) throw new Error("The source document is no longer available.");
    await useAppStore.getState().addHighlightToLibrary({
      boardId: targetBoardId,
      documentId: selectedDocument.id,
      documentTitle: selectedDocument.title,
      highlightId,
      content,
    });
  };

  if (!active) return null;

  return (
    <div className="absolute inset-0 z-[70] flex bg-[#101217] text-zinc-100">
      <aside className="flex w-64 shrink-0 flex-col border-r border-[#292e38] bg-[#151820]">
        <div className="flex h-14 items-center justify-between border-b border-[#292e38] px-4">
          <div className="flex items-center gap-2 text-sm font-semibold">
            <FileText size={16} className="text-violet-300" />
            Documents
          </div>
          <button type="button" onClick={onClose} aria-label="Close documents" className="rounded-md p-1.5 text-zinc-500 hover:bg-[#262b36] hover:text-white">
            <X size={16} />
          </button>
        </div>
        <button
          type="button"
          disabled={isImporting}
          onClick={() => void importDocument()}
          className="mx-3 mt-3 flex items-center justify-center gap-2 rounded-lg border border-violet-400/30 bg-violet-500/10 px-3 py-2 text-xs font-medium text-violet-200 transition hover:bg-violet-500/20 disabled:opacity-50"
        >
          {isImporting ? <LoaderCircle size={14} className="animate-spin" /> : <FileUp size={14} />}
          Import document
        </button>
        {error && <p role="alert" className="mx-3 mt-3 text-xs text-rose-300">{error}</p>}
        <div className="mt-3 flex-1 overflow-y-auto px-2 pb-3">
          {isLoading ? (
            <p className="px-3 py-4 text-xs text-zinc-500">Loading documents...</p>
          ) : documents.length === 0 ? (
            <p className="px-3 py-4 text-xs leading-relaxed text-zinc-500">Import a PDF, DOCX, or Markdown file to annotate it.</p>
          ) : documents.map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => {
                setSelectedDocumentId(item.id);
                setJumpToHighlightId(null);
              }}
              className={`mb-1 flex w-full items-center gap-2 rounded-lg px-3 py-2.5 text-left text-xs transition ${
                selectedDocumentId === item.id ? "bg-[#292e3a] text-white" : "text-zinc-400 hover:bg-[#20242e] hover:text-zinc-200"
              }`}
            >
              <FileText size={14} className="shrink-0 text-zinc-500" />
              <span className="min-w-0 flex-1 truncate">{item.title}</span>
              <span className="text-[9px] uppercase text-zinc-600">{item.file_type}</span>
            </button>
          ))}
        </div>
      </aside>
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center justify-between border-b border-[#292e38] px-5">
          <div className="min-w-0">
            <h1 className="truncate text-sm font-semibold">{selectedDocument?.title ?? "Document Workspace"}</h1>
            {selectedDocument && <p className="text-[10px] uppercase tracking-wide text-zinc-500">{selectedDocument.file_type}</p>}
          </div>
        </header>
        {selectedDocument ? selectedDocument.file_type === "pdf" ? (
          <PdfViewer
            key={selectedDocument.id}
            document={selectedDocument}
            jumpToHighlightId={jumpToHighlightId}
            boardId={boardId}
            onSendToLibrary={sendHighlightToLibrary}
          />
        ) : selectedDocument.file_type === "docx" ? (
          <DocxViewer
            key={selectedDocument.id}
            document={selectedDocument}
            jumpToHighlightId={jumpToHighlightId}
            boardId={boardId}
            onSendToLibrary={sendHighlightToLibrary}
          />
        ) : (
          <MarkdownViewer
            key={selectedDocument.id}
            document={selectedDocument}
            jumpToHighlightId={jumpToHighlightId}
            boardId={boardId}
            onSendToLibrary={sendHighlightToLibrary}
          />
        ) : (
          <div className="flex flex-1 items-center justify-center text-sm text-zinc-500">
            {documents.length ? "Select a document to view it." : "No documents imported yet."}
          </div>
        )}
      </main>
    </div>
  );
};
