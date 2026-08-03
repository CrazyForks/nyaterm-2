import type React from "react";
import type { DragEvent } from "react";
import { ContextMenu, ContextMenuTrigger } from "@/components/ui/context-menu";
import type { NoteTreeNode } from "@/types/notes";
import NoteTreeContextMenu, { type NoteTreeMenuLabels } from "./NoteTreeContextMenu";
import NoteTreeItem from "./NoteTreeItem";

interface NoteTreeProps {
  nodes: NoteTreeNode[];
  folders: NoteTreeNode[];
  selectedNodeId: string | null;
  expandedFolderIds: Set<string>;
  editingNodeId: string | null;
  dragOverNodeId: string | null;
  labels: NoteTreeMenuLabels;
  onSelect: (node: NoteTreeNode) => void;
  onToggle: (node: NoteTreeNode) => void;
  onOpen: (node: NoteTreeNode) => void;
  onRenameStart: (node: NoteTreeNode) => void;
  onRenameSubmit: (node: NoteTreeNode, name: string) => void;
  onRenameCancel: () => void;
  onCreateNote: (parentId: string | null) => void;
  onCreateFolder: (parentId: string | null) => void;
  onMove: (node: NoteTreeNode, parentId: string | null) => void;
  onDelete: (node: NoteTreeNode) => void;
  onRefresh: () => void;
  onDragStartNode: (node: NoteTreeNode) => void;
  onDragOverNode: (event: DragEvent<HTMLDivElement>, node: NoteTreeNode) => void;
  onDropNode: (event: DragEvent<HTMLDivElement>, node: NoteTreeNode) => void;
  onDragEnd: () => void;
  onDragOverRoot: (event: DragEvent<HTMLDivElement>) => void;
  onDropRoot: (event: DragEvent<HTMLDivElement>) => void;
}

function renderNodes(props: NoteTreeProps, nodes: NoteTreeNode[], depth: number): React.ReactNode {
  return nodes.map((node) => (
    <div key={node.id}>
      <NoteTreeItem
        node={node}
        depth={depth}
        folders={props.folders}
        selected={props.selectedNodeId === node.id}
        expanded={props.expandedFolderIds.has(node.id)}
        editing={props.editingNodeId === node.id}
        dragOver={props.dragOverNodeId === node.id}
        labels={props.labels}
        onSelect={props.onSelect}
        onToggle={props.onToggle}
        onOpen={props.onOpen}
        onRenameStart={props.onRenameStart}
        onRenameSubmit={props.onRenameSubmit}
        onRenameCancel={props.onRenameCancel}
        onCreateNote={props.onCreateNote}
        onCreateFolder={props.onCreateFolder}
        onMove={props.onMove}
        onDelete={props.onDelete}
        onRefresh={props.onRefresh}
        onDragStartNode={props.onDragStartNode}
        onDragOverNode={props.onDragOverNode}
        onDropNode={props.onDropNode}
        onDragEnd={props.onDragEnd}
      />
      {node.kind === "folder" && props.expandedFolderIds.has(node.id)
        ? renderNodes(props, node.children, depth + 1)
        : null}
    </div>
  ));
}

export default function NoteTree(props: NoteTreeProps) {
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          className="terminal-scroll flex-1 space-y-0.5 overflow-auto p-1.5 text-xs"
          onDragOver={props.onDragOverRoot}
          onDrop={props.onDropRoot}
        >
          {renderNodes(props, props.nodes, 0)}
        </div>
      </ContextMenuTrigger>
      <NoteTreeContextMenu
        node={null}
        folders={props.folders}
        labels={props.labels}
        onOpen={props.onOpen}
        onCreateNote={props.onCreateNote}
        onCreateFolder={props.onCreateFolder}
        onRename={props.onRenameStart}
        onMove={props.onMove}
        onDelete={props.onDelete}
        onRefresh={props.onRefresh}
      />
    </ContextMenu>
  );
}
