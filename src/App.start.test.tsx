import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";

const useWorkspaceMock = vi.hoisted(() => vi.fn());

vi.mock("./hooks/useWorkspace", () => ({
  useWorkspace: () => useWorkspaceMock(),
}));

describe("App start page", () => {
  it("renders workspace picker", () => {
    useWorkspaceMock.mockReturnValue({
      phase: "start",
      workspace: null,
      recent: [
        {
          path: "/tmp/old",
          name: "Old",
          last_opened: 1,
          valid: true,
          root_count: 1,
          album_count: 0,
          tag_count: 0,
          read_only: false,
        },
      ],
      busy: false,
      busyMessage: "",
      notification: null,
      dismissToast: vi.fn(),
      bootstrap: vi.fn(),
      createWorkspaceAt: vi.fn(),
      pickAndCreateWorkspace: vi.fn(),
      openWorkspacePath: vi.fn(),
      pickAndOpenWorkspace: vi.fn(),
      closeWorkspace: vi.fn(),
      removeRecent: vi.fn(),
    });
    render(<App />);
    expect(
      screen.getByRole("heading", { name: "Memory Hourglass" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Open Workspace" }),
    ).toBeInTheDocument();
    expect(screen.getByText("/tmp/old")).toBeInTheDocument();
  });

  it("starts workspace actions from start page", async () => {
    const pickAndOpenWorkspace = vi.fn();
    const pickAndCreateWorkspace = vi.fn();
    const openWorkspacePath = vi.fn();
    const removeRecent = vi.fn();
    useWorkspaceMock.mockReturnValue({
      phase: "start",
      workspace: null,
      recent: [
        {
          path: "/tmp/old",
          name: "Old",
          last_opened: 1,
          valid: true,
          root_count: 1,
          album_count: 0,
          tag_count: 0,
          read_only: false,
        },
      ],
      busy: false,
      busyMessage: "",
      notification: null,
      dismissToast: vi.fn(),
      bootstrap: vi.fn(),
      createWorkspaceAt: vi.fn(),
      pickAndCreateWorkspace,
      openWorkspacePath,
      pickAndOpenWorkspace,
      closeWorkspace: vi.fn(),
      removeRecent,
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Open Workspace" }));
    expect(pickAndOpenWorkspace).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "Create Workspace" }));
    await user.click(
      screen.getByRole("menuitem", { name: "Read-Write Workspace" }),
    );
    expect(pickAndCreateWorkspace).toHaveBeenCalledWith(false);

    await user.click(screen.getByText("/tmp/old"));
    expect(openWorkspacePath).toHaveBeenCalledWith("/tmp/old");

    await user.click(screen.getByTitle("Remove workspace"));
    expect(removeRecent).toHaveBeenCalledWith("/tmp/old");
  });

  it("renders start page when workspace is missing in library phase", () => {
    useWorkspaceMock.mockReturnValue({
      phase: "library",
      workspace: null,
      recent: [],
      busy: false,
      busyMessage: "",
      notification: null,
      dismissToast: vi.fn(),
      pickAndCreateWorkspace: vi.fn(),
      pickAndOpenWorkspace: vi.fn(),
      openWorkspacePath: vi.fn(),
      closeWorkspace: vi.fn(),
      removeRecent: vi.fn(),
    });
    render(<App />);
    expect(
      screen.getByRole("heading", { name: "Memory Hourglass" }),
    ).toBeInTheDocument();
  });
});
