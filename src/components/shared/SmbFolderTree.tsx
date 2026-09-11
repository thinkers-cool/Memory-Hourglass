import { ChevronDown, ChevronRight, Folder } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import * as api from "../../api/client";
import { ghostBtnClass } from "../../lib/buttonClass";
import { relativeFolderPath } from "../../lib/smbFolderPath";

type TreeNode = {
  name: string;
  path: string;
  relativePath: string;
  children: TreeNode[] | null;
  loading: boolean;
};

function createRootNode(rootPath: string, rootLabel: string): TreeNode {
  return {
    name: rootLabel,
    path: rootPath,
    relativePath: "",
    children: null,
    loading: false,
  };
}

function TreeRow({
  node,
  depth,
  expanded,
  selectedRelativePath,
  onToggle,
  onSelect,
}: {
  node: TreeNode;
  depth: number;
  expanded: boolean;
  selectedRelativePath: string;
  onToggle: () => void;
  onSelect: () => void;
}) {
  const hasLoadedChildren = node.children !== null;
  const hasChildren = hasLoadedChildren ? node.children!.length > 0 : true;
  const selected = node.relativePath === selectedRelativePath;

  return (
    <button
      type="button"
      className={`${ghostBtnClass("h-auto min-h-0 w-full justify-start rounded-md px-2 py-1.5 text-sm font-normal")} ${
        selected ? "bg-interactive-selected text-primary" : ""
      }`}
      style={{ paddingLeft: `${depth * 14 + 8}px` }}
      onClick={onSelect}
    >
      {hasChildren ? (
        <span
          className="inline-flex h-4 w-4 shrink-0 items-center justify-center"
          onClick={(event) => {
            event.stopPropagation();
            onToggle();
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              event.stopPropagation();
              onToggle();
            }
          }}
          role="button"
          tabIndex={0}
          aria-label={expanded ? "Collapse folder" : "Expand folder"}
        >
          {node.loading ? (
            <span className="loading loading-spinner loading-xs opacity-60" />
          ) : expanded ? (
            <ChevronDown className="h-3.5 w-3.5 opacity-70" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 opacity-70" />
          )}
        </span>
      ) : (
        <span className="inline-block h-4 w-4 shrink-0" />
      )}
      <Folder className="h-3.5 w-3.5 shrink-0 opacity-60" />
      <span className="truncate">{node.name}</span>
    </button>
  );
}

export function SmbFolderTree({
  rootPath,
  rootLabel,
  selectedRelativePath,
  onSelect,
}: {
  rootPath: string;
  rootLabel: string;
  selectedRelativePath: string;
  onSelect: (relativePath: string) => void;
}) {
  const [root, setRoot] = useState<TreeNode>(() =>
    createRootNode(rootPath, rootLabel),
  );
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(
    () => new Set([""]),
  );

  const loadChildren = useCallback(
    async (node: TreeNode) => {
      setRoot((current) =>
        updateNode(current, node.path, (value) => ({
          ...value,
          loading: true,
        })),
      );
      try {
        const entries = await api.listFolderChildren(node.path);
        const children = entries.map((entry) => ({
          name: entry.name,
          path: entry.path,
          relativePath: relativeFolderPath(rootPath, entry.path),
          children: null,
          loading: false,
        }));
        setRoot((current) =>
          updateNode(current, node.path, (value) => ({
            ...value,
            children,
            loading: false,
          })),
        );
      } catch {
        setRoot((current) =>
          updateNode(current, node.path, (value) => ({
            ...value,
            children: [],
            loading: false,
          })),
        );
      }
    },
    [rootPath],
  );

  useEffect(() => {
    setRoot(createRootNode(rootPath, rootLabel));
    setExpandedPaths(new Set([""]));
    void loadChildren(createRootNode(rootPath, rootLabel));
  }, [loadChildren, rootLabel, rootPath]);

  const toggleNode = (node: TreeNode) => {
    const nextExpanded = new Set(expandedPaths);
    if (nextExpanded.has(node.relativePath)) {
      nextExpanded.delete(node.relativePath);
      setExpandedPaths(nextExpanded);
      return;
    }
    nextExpanded.add(node.relativePath);
    setExpandedPaths(nextExpanded);
    if (node.children === null) {
      void loadChildren(node);
    }
  };

  const rows: Array<{ node: TreeNode; depth: number }> = [];
  const walk = (node: TreeNode, depth: number) => {
    rows.push({ node, depth });
    if (!expandedPaths.has(node.relativePath) || node.children === null) {
      return;
    }
    for (const child of node.children) {
      walk(child, depth + 1);
    }
  };
  walk(root, 0);

  return (
    <div className="max-h-56 overflow-y-auto rounded-lg border border-interactive-border bg-surface-inset p-1">
      {rows.map(({ node, depth }) => (
        <TreeRow
          key={node.path}
          node={node}
          depth={depth}
          expanded={expandedPaths.has(node.relativePath)}
          selectedRelativePath={selectedRelativePath}
          onToggle={() => toggleNode(node)}
          onSelect={() => onSelect(node.relativePath)}
        />
      ))}
    </div>
  );
}

function updateNode(
  node: TreeNode,
  path: string,
  updater: (value: TreeNode) => TreeNode,
): TreeNode {
  if (node.path === path) {
    return updater(node);
  }
  if (node.children === null) {
    return node;
  }
  return {
    ...node,
    children: node.children.map((child) => updateNode(child, path, updater)),
  };
}
