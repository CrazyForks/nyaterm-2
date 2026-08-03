import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { useApp } from "@/context/AppContext";
import { getErrorMessage } from "@/lib/errors";
import { invoke } from "@/lib/invoke";
import { logger } from "@/lib/logger";
import { listenNotesChanged } from "@/lib/noteEvents";
import type {
  DeleteNoteNodeResult,
  NoteDocument,
  NoteFolder,
  NoteNodeKind,
  NoteSummary,
  NoteTreePayload,
} from "@/types/notes";

export function useNotesTree() {
  const { appSettings, updateUi } = useApp();
  const [folders, setFolders] = useState<NoteFolder[]>([]);
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const selectedNodeId = appSettings.ui.notes_last_selected_node_id ?? null;
  const expandedFolderIds = useMemo(
    () => new Set(appSettings.ui.notes_expanded_folder_ids ?? []),
    [appSettings.ui.notes_expanded_folder_ids],
  );

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const payload = await invoke<NoteTreePayload>("list_note_tree");
      setFolders(payload.folders);
      setNotes(payload.notes);
      setError(null);
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      logger.error({
        domain: "ui.error",
        event: "notes.tree.load_failed",
        message: "Failed to load notes",
        error: err,
      });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listenNotesChanged((event) => {
      if (disposed) return;
      if (event.kind === "updated" && event.nodeKind === "note" && event.ids.length === 1) {
        void invoke<NoteDocument>("get_note", { noteId: event.ids[0] })
          .then((note) => {
            setNotes((current) =>
              current.map((item) =>
                item.id === note.id
                  ? {
                      id: note.id,
                      parent_id: note.parent_id,
                      title: note.title,
                      sort_order: note.sort_order,
                      revision: note.revision,
                      created_at_ms: note.created_at_ms,
                      updated_at_ms: note.updated_at_ms,
                    }
                  : item,
              ),
            );
          })
          .catch(() => refresh());
        return;
      }
      void refresh();
    })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch((err) => {
        logger.warn({
          domain: "ui.error",
          event: "notes.event.listen_failed",
          message: "Failed to listen for notes changes",
          error: err,
        });
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [refresh]);

  const setSelectedNodeId = useCallback(
    (id: string | null) => updateUi({ notes_last_selected_node_id: id }),
    [updateUi],
  );

  const setExpandedFolderIds = useCallback(
    (ids: Set<string>) => updateUi({ notes_expanded_folder_ids: Array.from(ids) }),
    [updateUi],
  );

  const createFolder = useCallback(
    async (parentId: string | null, name?: string) => {
      const folder = await invoke<NoteFolder>("create_note_folder", { parentId, name });
      setSelectedNodeId(folder.id);
      setExpandedFolderIds(new Set([...expandedFolderIds, ...(parentId ? [parentId] : [])]));
      await refresh();
      return folder;
    },
    [expandedFolderIds, refresh, setExpandedFolderIds, setSelectedNodeId],
  );

  const createNote = useCallback(
    async (parentId: string | null, title?: string, markdown?: string) => {
      const note = await invoke<NoteDocument>("create_note", { parentId, title, markdown });
      setSelectedNodeId(note.id);
      setExpandedFolderIds(new Set([...expandedFolderIds, ...(parentId ? [parentId] : [])]));
      await refresh();
      return note;
    },
    [expandedFolderIds, refresh, setExpandedFolderIds, setSelectedNodeId],
  );

  const renameNode = useCallback(
    async (nodeKind: NoteNodeKind, nodeId: string, name: string) => {
      await invoke("rename_note_node", { nodeKind, nodeId, name });
      await refresh();
    },
    [refresh],
  );

  const moveNode = useCallback(
    async (nodeKind: NoteNodeKind, nodeId: string, parentId: string | null, sortOrder: number) => {
      await invoke("move_note_node", { nodeKind, nodeId, parentId, sortOrder });
      if (parentId) setExpandedFolderIds(new Set([...expandedFolderIds, parentId]));
      await refresh();
    },
    [expandedFolderIds, refresh, setExpandedFolderIds],
  );

  const deleteNode = useCallback(
    async (nodeKind: NoteNodeKind, nodeId: string) => {
      const result = await invoke<DeleteNoteNodeResult>("delete_note_node", { nodeKind, nodeId });
      if (selectedNodeId === nodeId) setSelectedNodeId(null);
      await refresh();
      return result;
    },
    [refresh, selectedNodeId, setSelectedNodeId],
  );

  const runAction = useCallback(async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (err) {
      toast.error(getErrorMessage(err));
      logger.error({
        domain: "ui.error",
        event: "notes.action.failed",
        message: "Notes action failed",
        error: err,
      });
    }
  }, []);

  return {
    folders,
    notes,
    loading,
    error,
    refresh,
    selectedNodeId,
    setSelectedNodeId,
    expandedFolderIds,
    setExpandedFolderIds,
    createFolder,
    createNote,
    renameNode,
    moveNode,
    deleteNode,
    runAction,
  };
}
