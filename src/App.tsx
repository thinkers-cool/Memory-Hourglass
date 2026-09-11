import { lazy, Suspense } from "react";
import { StartPage } from "./components/StartPage";
import { useWorkspace } from "./hooks/useWorkspace";

const LibraryApp = lazy(() =>
  import("./components/LibraryApp").then((module) => ({
    default: module.LibraryApp,
  })),
);

function AppLoadingSpinner() {
  return (
    <div className="flex min-h-screen items-center justify-center app-canvas">
      <span className="loading loading-spinner loading-md" />
    </div>
  );
}

export function App() {
  const workspaceState = useWorkspace();
  const {
    phase,
    workspace,
    recent,
    busy,
    busyMessage,
    notification,
    dismissToast,
    pickAndCreateWorkspace,
    pickAndOpenWorkspace,
    openWorkspacePath,
    closeWorkspace,
    removeRecent,
  } = workspaceState;

  if (phase === "loading") {
    return <AppLoadingSpinner />;
  }

  if (phase === "start" || !workspace) {
    return (
      <StartPage
        recent={recent}
        busy={busy}
        busyMessage={busyMessage}
        notification={notification}
        onDismissAlert={dismissToast}
        onCreate={(readOnly) => void pickAndCreateWorkspace(readOnly)}
        onOpen={() => void pickAndOpenWorkspace()}
        onOpenRecent={(path) => void openWorkspacePath(path)}
        onRemoveRecent={(path) => void removeRecent(path)}
      />
    );
  }

  return (
    <Suspense fallback={<AppLoadingSpinner />}>
      <LibraryApp
        onCloseWorkspace={() => void closeWorkspace()}
        readOnly={workspace.read_only}
      />
    </Suspense>
  );
}
