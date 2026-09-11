import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { CompareViewer } from "./CompareViewer";
import { AssetFullView } from "./AssetFullView";
import { GalleryPlayer } from "./GalleryPlayer";
import { ConfirmDialog } from "./layout/ConfirmDialog";
import { PurgeConfirmDialog } from "./layout/PurgeConfirmDialog";
import { NamePromptDialog } from "./layout/NamePromptDialog";
import { ExportDialog } from "./layout/ExportDialog";
import { LibraryFilterToolbar } from "./layout/LibraryFilterToolbar";
import { FloatingSelectionBar } from "./layout/FloatingSelectionBar";
import { InspectorPanel, InspectorPlaceholder } from "./layout/InspectorPanel";
import { LibraryPanel } from "./layout/LibraryPanel";
import { NavRail, type LeftTab } from "./layout/NavRail";
import { SmbConnectDialog } from "./layout/SmbConnectDialog";
import { StatusBar } from "./layout/StatusBar";
import {
  ResizableSplit,
  ResizableTrailingPanel,
} from "./shared/ResizableSplit";
import { VirtualGrid } from "./VirtualGrid";
import { useKeyboard } from "../hooks/useKeyboard";
import { useLibrary } from "../hooks/useLibrary";
import {
  resolveFloatingBarMode,
  shouldShowFloatingBar,
} from "../lib/floatingBarMode";
import { resolveSelectionRating } from "../lib/selection";
import {
  INSPECTOR_STORAGE_KEY,
  INSPECTOR_WIDTH_DEFAULT,
  INSPECTOR_WIDTH_MAX,
  INSPECTOR_WIDTH_MIN,
  LEFT_PANEL_STORAGE_KEY,
  LEFT_PANEL_WIDTH_DEFAULT,
  LEFT_PANEL_WIDTH_MAX,
  LEFT_PANEL_WIDTH_MIN,
  clampPanelWidth,
  loadPanelWidth,
  savePanelWidth,
} from "../lib/panelLayout";

export function LibraryApp({
  onCloseWorkspace,
  readOnly = false,
}: {
  onCloseWorkspace: () => void;
  readOnly?: boolean;
}) {
  const [leftTab, setLeftTab] = useState<LeftTab>("library");
  const [leftPanelWidth, setLeftPanelWidth] = useState(() =>
    loadPanelWidth(
      LEFT_PANEL_STORAGE_KEY,
      LEFT_PANEL_WIDTH_DEFAULT,
      LEFT_PANEL_WIDTH_MIN,
      LEFT_PANEL_WIDTH_MAX,
    ),
  );
  const [inspectorWidth, setInspectorWidth] = useState(() =>
    loadPanelWidth(
      INSPECTOR_STORAGE_KEY,
      INSPECTOR_WIDTH_DEFAULT,
      INSPECTOR_WIDTH_MIN,
      INSPECTOR_WIDTH_MAX,
    ),
  );
  const library = useLibrary(readOnly);
  const purgeEnabled = !readOnly;
  const {
    roots,
    albums,
    collections,
    tags,
    deletedCount,
    items,
    total,
    detail,
    busy,
    notification,
    scanStatus,
    galleryIndex,
    setGalleryIndex,
    fullView,
    inspectorVisible,
    gridColumnCount,
    setGridColumnCount,
    compareOpen,
    compareItems,
    compareDetails,
    smbDialogOpen,
    setSmbDialogOpen,
    collectionDialogOpen,
    closeCollectionDialog,
    tagMenuOpen,
    setTagMenuOpen,
    albumMenuOpen,
    setAlbumMenuOpen,
    purgeDialogOpen,
    purgeTargetIds,
    closePurgeDialog,
    loadingMore,
    hasMore,
    filterBar,
    extraFilter,
    selectedCollectionId,
    setFilterBar,
    selectedId,
    selectedIds,
    actions,
    exportDialog,
    closeExportDialog,
    updateExportDialog,
    startExportFromDialog,
    confirmDialog,
    closeConfirmDialog,
    dismissToast,
    undoActivity,
    stampConfig,
    stampArmed,
    stampMatchedIds,
  } = library;
  const { t } = useTranslation("dialogs");

  useEffect(() => {
    savePanelWidth(LEFT_PANEL_STORAGE_KEY, leftPanelWidth);
  }, [leftPanelWidth]);

  useEffect(() => {
    savePanelWidth(INSPECTOR_STORAGE_KEY, inspectorWidth);
  }, [inspectorWidth]);

  const selectionExportIds = useMemo(
    () => Array.from(selectedIds),
    [selectedIds],
  );

  const keyboardExportIds = useMemo(() => {
    if (selectedIds.size > 0) {
      return selectionExportIds;
    }
    if (selectedId !== null) {
      return [selectedId];
    }
    return items.map((item) => item.id);
  }, [items, selectedId, selectedIds.size, selectionExportIds]);

  useKeyboard(
    actions,
    galleryIndex === null && !compareOpen,
    fullView,
    selectedIds.size > 0,
    filterBar.deleteStatus === "deleted",
    purgeEnabled,
    stampArmed,
    selectionExportIds,
    keyboardExportIds,
  );

  const fullViewCard = useMemo(() => {
    if (selectedId === null) return null;
    return items.find((item) => item.id === selectedId) ?? null;
  }, [items, selectedId]);

  const fullViewIndex = useMemo(() => {
    if (selectedId === null) return -1;
    return items.findIndex((item) => item.id === selectedId);
  }, [items, selectedId]);

  const floatingBarMode = useMemo(
    () => resolveFloatingBarMode(filterBar.deleteStatus),
    [filterBar.deleteStatus],
  );

  const showFloatingBar = shouldShowFloatingBar(
    selectedIds.size,
    galleryIndex !== null,
    compareOpen,
  );

  const selectedTagKeys = useMemo(() => {
    if (selectedIds.size !== 1 || detail === null) {
      return [];
    }
    if (!selectedIds.has(detail.asset.id)) {
      return [];
    }
    return detail.tag_ids.map(String);
  }, [selectedIds, detail]);

  const selectedAlbumKeys = useMemo(() => {
    if (selectedIds.size !== 1 || detail === null) {
      return [];
    }
    if (!selectedIds.has(detail.asset.id)) {
      return [];
    }
    return detail.album_ids.map(String);
  }, [selectedIds, detail]);

  const selectionRating = useMemo(
    () => resolveSelectionRating(items, selectedIds),
    [items, selectedIds],
  );

  const showInspector = selectedIds.size > 0 && inspectorVisible;

  const inspector = showInspector ? (
    detail ? (
      <InspectorPanel
        detail={detail}
        tags={tags}
        onClose={actions.closeInspector}
        onPurge={actions.purge}
        purgeEnabled={purgeEnabled}
        onSelectLink={actions.openLinked}
        onSoftDeleteDuplicate={actions.softDeleteDuplicate}
        onUndoActivity={(id) => void undoActivity(id)}
      />
    ) : (
      <InspectorPlaceholder onClose={actions.closeInspector} />
    )
  ) : null;

  const contentArea =
    compareOpen && compareItems.length >= 2 ? (
      <CompareViewer
        items={compareItems}
        compareDetails={compareDetails}
        tags={tags}
        albums={albums}
        busy={busy}
        stampArmed={stampArmed}
        onToggleStamp={() => void actions.toggleStampOnTargets()}
        onClose={actions.closeCompare}
        onRate={(id, rating) => void actions.rateAsset(id, rating)}
        onToggleTag={(id, tagId, add) =>
          void actions.toggleTagOnAsset(id, tagId, add)
        }
        onCreateTag={(id, tagName) => actions.createTagOnAsset(id, tagName)}
        onToggleAlbum={(id, albumId, add) =>
          void actions.toggleAlbumOnAsset(id, albumId, add)
        }
        onCreateAlbum={(id, name) => actions.createAlbumOnAsset(id, name)}
        onExport={(id) => actions.openExport([id])}
        onDelete={(id) => void actions.deleteAsset(id)}
      />
    ) : fullView && fullViewCard && fullViewIndex >= 0 ? (
      <ResizableTrailingPanel
        main={
          <AssetFullView
            card={fullViewCard}
            index={fullViewIndex}
            total={items.length}
            onClose={actions.closeFullView}
            onNavigateRelative={actions.navigateRelative}
          />
        }
        side={inspector}
        sideWidth={inspectorWidth}
        onSideWidthChange={(width) =>
          setInspectorWidth(
            clampPanelWidth(width, INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX),
          )
        }
        minMain={320}
        minSide={INSPECTOR_WIDTH_MIN}
        showSide={showInspector}
      />
    ) : (
      <ResizableTrailingPanel
        main={
          <VirtualGrid
            items={items}
            selectedId={selectedId}
            selectedIds={selectedIds}
            columnCount={gridColumnCount}
            stampConfig={stampConfig}
            stampArmed={stampArmed}
            stampMatchedIds={stampMatchedIds}
            onSelect={actions.selectAsset}
            onOpenFullView={actions.openFullViewFor}
            onLoadMore={actions.loadMore}
            hasMore={hasMore}
            loadingMore={loadingMore}
          />
        }
        side={inspector}
        sideWidth={inspectorWidth}
        onSideWidthChange={(width) =>
          setInspectorWidth(
            clampPanelWidth(width, INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX),
          )
        }
        minMain={280}
        minSide={INSPECTOR_WIDTH_MIN}
        showSide={showInspector}
      />
    );

  return (
    <div className="flex h-screen app-canvas">
      <NavRail
        active={leftTab}
        onChange={setLeftTab}
        onExit={onCloseWorkspace}
      />

      <ResizableSplit
        leading={
          <LibraryPanel
            tab={leftTab}
            roots={roots}
            albums={albums}
            collections={collections}
            tags={tags}
            deletedCount={deletedCount}
            filterBar={filterBar}
            extraFilter={extraFilter}
            selectedCollectionId={selectedCollectionId}
            deleteStatus={filterBar.deleteStatus}
            busy={busy}
            actions={actions}
          />
        }
        trailing={
          <main className="relative flex min-h-0 min-w-0 flex-1 flex-col">
            <LibraryFilterToolbar
              tags={tags}
              albums={albums}
              filterBar={filterBar}
              setFilterBar={setFilterBar}
              gridColumnCount={gridColumnCount}
              onGridColumnCountChange={setGridColumnCount}
              onAdjustGridSize={actions.adjustGridSize}
              onSaveCollection={actions.saveCollection}
              showSaveCollection={leftTab !== "collections"}
              onOpenSlideshow={actions.openGallery}
              onOpenExport={() =>
                actions.openExport(
                  selectedIds.size > 0
                    ? Array.from(selectedIds)
                    : selectedId !== null
                      ? [selectedId]
                      : items.map((item) => item.id),
                )
              }
              slideshowDisabled={items.length === 0}
              stampConfig={stampConfig}
              stampArmed={stampArmed}
              onDisarmStamp={actions.disarmStamp}
              onStampRatingChange={actions.setStampRating}
              onToggleStampTag={actions.toggleStampTag}
              onToggleStampAlbum={actions.toggleStampAlbum}
            />

            <div className="relative z-0 flex min-h-0 flex-1 flex-col">
              {contentArea}
              {showFloatingBar && (
                <FloatingSelectionBar
                  mode={floatingBarMode}
                  count={selectedIds.size}
                  tags={tags}
                  albums={albums}
                  busy={busy}
                  tagMenuOpen={tagMenuOpen}
                  onTagMenuOpenChange={setTagMenuOpen}
                  albumMenuOpen={albumMenuOpen}
                  onAlbumMenuOpenChange={setAlbumMenuOpen}
                  onClear={actions.clearSelection}
                  rating={selectionRating}
                  onRate={actions.batchRate}
                  onToggleTag={actions.toggleTagOnSelection}
                  onCreateTag={actions.createTagOnSelection}
                  onToggleAlbum={actions.toggleAlbumOnSelection}
                  onCreateAlbum={actions.createAlbumOnSelection}
                  onExport={() => actions.openExport(Array.from(selectedIds))}
                  onCompare={actions.openCompare}
                  onDelete={actions.batchRemove}
                  onRestore={actions.restoreSelected}
                  onPurge={actions.batchPurge}
                  purgeEnabled={purgeEnabled}
                  selectedTagKeys={selectedTagKeys}
                  selectedAlbumKeys={selectedAlbumKeys}
                />
              )}
            </div>

            <StatusBar
              total={total}
              selectedCount={selectedIds.size}
              scanStatus={scanStatus}
              roots={roots}
              exportActive={exportDialog.jobId !== null}
              exportProgress={exportDialog.progress}
              notification={notification}
              busy={busy}
              onDismissAlert={dismissToast}
            />
          </main>
        }
        leadingWidth={leftPanelWidth}
        onLeadingWidthChange={(width) =>
          setLeftPanelWidth(
            clampPanelWidth(width, LEFT_PANEL_WIDTH_MIN, LEFT_PANEL_WIDTH_MAX),
          )
        }
        minLeading={LEFT_PANEL_WIDTH_MIN}
        minTrailing={480}
      />

      {galleryIndex !== null && (
        <GalleryPlayer
          items={items}
          index={galleryIndex}
          onClose={() => setGalleryIndex(null)}
          onNavigate={setGalleryIndex}
          onRate={(id, rating) => void actions.rateAsset(id, rating)}
        />
      )}

      <SmbConnectDialog
        open={smbDialogOpen}
        busy={busy}
        onClose={() => setSmbDialogOpen(false)}
        onConnect={(params) => void actions.connectSmbShare(params)}
        onAddMountedPath={(path) => {
          setSmbDialogOpen(false);
          void actions.addMountedSmbPath(path);
        }}
      />

      <NamePromptDialog
        open={collectionDialogOpen}
        title={t("namePrompt.collectionTitle")}
        label={t("namePrompt.collectionLabel")}
        submitLabel={t("namePrompt.collectionSubmit")}
        busy={busy}
        onClose={closeCollectionDialog}
        onSubmit={(name) => void actions.submitSaveCollection(name)}
      />

      {purgeEnabled && (
        <PurgeConfirmDialog
          open={purgeDialogOpen}
          fileName={detail?.asset.file_name ?? ""}
          itemCount={purgeTargetIds.length}
          busy={busy}
          onClose={closePurgeDialog}
          onConfirm={() => void actions.submitPurge()}
        />
      )}

      <ExportDialog
        state={exportDialog}
        onClose={closeExportDialog}
        onUpdate={updateExportDialog}
        onStart={() => void startExportFromDialog()}
      />

      <ConfirmDialog
        open={confirmDialog.open}
        title={confirmDialog.title}
        message={confirmDialog.message}
        busy={busy}
        onClose={closeConfirmDialog}
        onConfirm={() => {
          void Promise.resolve(confirmDialog.onConfirm()).then(
            closeConfirmDialog,
          );
        }}
      />
    </div>
  );
}
