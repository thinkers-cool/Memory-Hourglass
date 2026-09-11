import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "./useWorkspace";

vi.mock("../api/client", () => ({
  tryOpenLastWorkspace: vi.fn(),
  listRecentWorkspaces: vi.fn(),
  openWorkspace: vi.fn(),
  createWorkspace: vi.fn(),
  closeWorkspace: vi.fn(),
  removeRecentWorkspace: vi.fn(),
  onMessageNotify: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../lib/pickFolder", () => ({
  pickFolder: vi.fn(),
}));

import * as api from "../api/client";
import { pickFolder } from "../lib/pickFolder";

const workspaceInfo = {
  path: "/tmp/ws",
  name: "Demo",
  id: "ws-1",
  read_only: false,
};

function Probe() {
  const state = useWorkspace();
  return (
    <div>
      <span data-testid="phase">{state.phase}</span>
      <span data-testid="workspace">{state.workspace?.name ?? ""}</span>
      <span data-testid="notification">{state.notification?.text ?? ""}</span>
      <button
        type="button"
        onClick={() => void state.openWorkspacePath("/tmp/open")}
      >
        open-path
      </button>
      <button
        type="button"
        onClick={() => void state.createWorkspaceAt("/tmp/new", true)}
      >
        create-path
      </button>
      <button type="button" onClick={() => void state.pickAndOpenWorkspace()}>
        pick-open
      </button>
      <button
        type="button"
        onClick={() => void state.pickAndCreateWorkspace(true)}
      >
        pick-create
      </button>
      <button type="button" onClick={() => void state.closeWorkspace()}>
        close
      </button>
      <button type="button" onClick={() => void state.removeRecent("/tmp/old")}>
        remove-recent
      </button>
    </div>
  );
}

describe("useWorkspace", () => {
  beforeEach(() => {
    vi.mocked(api.tryOpenLastWorkspace).mockReset();
    vi.mocked(api.listRecentWorkspaces).mockReset();
    vi.mocked(api.openWorkspace).mockReset();
    vi.mocked(api.createWorkspace).mockReset();
    vi.mocked(api.closeWorkspace).mockReset();
    vi.mocked(api.removeRecentWorkspace).mockReset();
    vi.mocked(pickFolder).mockReset();
    vi.mocked(api.listRecentWorkspaces).mockResolvedValue([]);
    vi.mocked(api.openWorkspace).mockResolvedValue(workspaceInfo);
    vi.mocked(api.createWorkspace).mockResolvedValue(workspaceInfo);
    vi.mocked(api.closeWorkspace).mockResolvedValue(undefined);
    vi.mocked(api.removeRecentWorkspace).mockResolvedValue(undefined);
  });

  it("opens last workspace on bootstrap", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(workspaceInfo);

    render(<Probe />);

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("library");
    });
    expect(screen.getByTestId("workspace").textContent).toBe("Demo");
  });

  it("shows start page when no last workspace", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);

    render(<Probe />);

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("start");
    });
  });

  it("shows start page when bootstrap throws", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockRejectedValue(
      new Error("boot failed"),
    );

    render(<Probe />);

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("start");
    });
    expect(screen.getByTestId("workspace").textContent).toBe("");
    await waitFor(() => {
      expect(screen.getByTestId("notification").textContent).toContain(
        "boot failed",
      );
    });
  });

  it("opens workspace by path", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "open-path" }));

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("library");
    });
    expect(api.openWorkspace).toHaveBeenCalledWith("/tmp/open");
    expect(screen.getByTestId("workspace").textContent).toBe("Demo");
  });

  it("creates workspace at path", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "create-path" }));

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("library");
    });
    expect(api.createWorkspace).toHaveBeenCalledWith("/tmp/new", true);
  });

  it("picks and opens workspace", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    vi.mocked(pickFolder).mockResolvedValue("/picked/ws");
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "pick-open" }));

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("library");
    });
    expect(pickFolder).toHaveBeenCalled();
    expect(api.openWorkspace).toHaveBeenCalledWith("/picked/ws");
  });

  it("ignores cancelled pick for open", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    vi.mocked(pickFolder).mockResolvedValue(null);
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "pick-open" }));

    await waitFor(() => {
      expect(api.openWorkspace).not.toHaveBeenCalled();
    });
    expect(screen.getByTestId("phase").textContent).toBe("start");
  });

  it("reports pick errors for open", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    vi.mocked(pickFolder).mockRejectedValue(new Error("picker failed"));
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "pick-open" }));

    await waitFor(() => {
      expect(screen.getByTestId("notification").textContent).toContain(
        "picker failed",
      );
    });
    expect(api.openWorkspace).not.toHaveBeenCalled();
  });

  it("ignores cancelled pick for create", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    vi.mocked(pickFolder).mockResolvedValue(null);
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "pick-create" }));

    await waitFor(() => {
      expect(api.createWorkspace).not.toHaveBeenCalled();
    });
    expect(screen.getByTestId("phase").textContent).toBe("start");
  });

  it("reports pick errors for create", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    vi.mocked(pickFolder).mockRejectedValue(new Error("create picker failed"));
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "pick-create" }));

    await waitFor(() => {
      expect(screen.getByTestId("notification").textContent).toContain(
        "create picker failed",
      );
    });
    expect(api.createWorkspace).not.toHaveBeenCalled();
  });

  it("picks and creates workspace", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    vi.mocked(pickFolder).mockResolvedValue("/picked/new");
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "pick-create" }));

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("library");
    });
    expect(api.createWorkspace).toHaveBeenCalledWith("/picked/new", true);
  });

  it("closes workspace", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(workspaceInfo);
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("library"),
    );

    await user.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(screen.getByTestId("phase").textContent).toBe("start");
    });
    expect(api.closeWorkspace).toHaveBeenCalled();
    expect(screen.getByTestId("workspace").textContent).toBe("");
  });

  it("removes recent workspace", async () => {
    vi.mocked(api.tryOpenLastWorkspace).mockResolvedValue(null);
    const user = userEvent.setup();
    render(<Probe />);
    await waitFor(() =>
      expect(screen.getByTestId("phase").textContent).toBe("start"),
    );

    await user.click(screen.getByRole("button", { name: "remove-recent" }));

    await waitFor(() => {
      expect(api.removeRecentWorkspace).toHaveBeenCalledWith("/tmp/old");
    });
    expect(api.listRecentWorkspaces).toHaveBeenCalled();
  });
});
