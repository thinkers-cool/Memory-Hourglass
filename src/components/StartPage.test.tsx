import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { StartPage } from "./StartPage";
import { errorNotification } from "../lib/notification";

describe("StartPage", () => {
  const onCreate = vi.fn();
  const onOpen = vi.fn();
  const onOpenRecent = vi.fn();
  const onRemoveRecent = vi.fn();

  beforeEach(() => {
    onCreate.mockReset();
    onOpen.mockReset();
    onOpenRecent.mockReset();
    onRemoveRecent.mockReset();
  });

  const defaultProps = {
    busy: false,
    busyMessage: "",
    notification: null,
    onCreate,
    onOpen,
    onOpenRecent,
    onRemoveRecent,
  };

  it("centers branding when no workspaces are registered", () => {
    render(<StartPage recent={[]} {...defaultProps} />);

    expect(
      screen.getByRole("button", { name: "Open Workspace" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create Workspace" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Workspaces")).not.toBeInTheDocument();
    expect(screen.queryByText("No workspaces yet")).not.toBeInTheDocument();
  });

  it("renders split layout when workspaces are registered", () => {
    render(
      <StartPage
        recent={[
          {
            path: "/tmp/demo",
            name: "Demo",
            last_opened: Math.floor(Date.now() / 1000),
            valid: true,
            root_count: 2,
            album_count: 1,
            tag_count: 5,
            read_only: false,
          },
        ]}
        {...defaultProps}
      />,
    );

    expect(screen.getByText("Workspaces")).toBeInTheDocument();
    expect(screen.getByText("/tmp/demo")).toBeInTheDocument();
    expect(
      screen.getByText("2 Libraries · 1 Album · 5 Tags"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Path not found")).not.toBeInTheDocument();
  });

  it("shows unavailable indication for invalid paths", () => {
    render(
      <StartPage
        recent={[
          {
            path: "/missing/workspace",
            name: "Missing",
            last_opened: Math.floor(Date.now() / 1000),
            valid: false,
            root_count: 0,
            album_count: 0,
            tag_count: 0,
            read_only: false,
          },
        ]}
        {...defaultProps}
      />,
    );

    expect(screen.getByText("/missing/workspace")).toBeInTheDocument();
    expect(screen.getByText("Path not found")).toBeInTheDocument();
  });

  it("opens workspace from primary action", async () => {
    const user = userEvent.setup();
    render(<StartPage recent={[]} {...defaultProps} />);
    await user.click(screen.getByRole("button", { name: "Open Workspace" }));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("closes create menu on outside click", async () => {
    const user = userEvent.setup();
    render(
      <>
        <StartPage recent={[]} {...defaultProps} />
        <button type="button">Outside</button>
      </>,
    );
    await user.click(screen.getByRole("button", { name: "Create Workspace" }));
    expect(
      screen.getByRole("menuitem", { name: "Read-Write Workspace" }),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(
      screen.queryByRole("menuitem", { name: "Read-Write Workspace" }),
    ).not.toBeInTheDocument();
  });

  it("starts read-write create flow from dropdown", async () => {
    const user = userEvent.setup();
    render(<StartPage recent={[]} {...defaultProps} />);

    await user.click(screen.getByRole("button", { name: "Create Workspace" }));
    await user.click(
      screen.getByRole("menuitem", { name: "Read-Write Workspace" }),
    );
    expect(onCreate).toHaveBeenCalledWith(false);
  });

  it("shows read-only badge on registered read-only workspaces", () => {
    render(
      <StartPage
        recent={[
          {
            path: "/tmp/readonly",
            name: "Readonly",
            last_opened: Math.floor(Date.now() / 1000),
            valid: true,
            root_count: 1,
            album_count: 0,
            tag_count: 0,
            read_only: true,
          },
        ]}
        {...defaultProps}
      />,
    );

    const row = screen.getByText("/tmp/readonly").closest("button");
    expect(row).not.toBeNull();
    expect(
      within(row as HTMLElement).getByText("Read Only"),
    ).toBeInTheDocument();
  });

  it("starts read-only create flow from dropdown", async () => {
    const user = userEvent.setup();
    render(<StartPage recent={[]} {...defaultProps} />);

    await user.click(screen.getByRole("button", { name: "Create Workspace" }));
    await user.click(
      screen.getByRole("menuitem", { name: "Read-Only Workspace" }),
    );
    expect(onCreate).toHaveBeenCalledWith(true);
  });

  it("opens registered workspace on click", async () => {
    const user = userEvent.setup();
    render(
      <StartPage
        recent={[
          {
            path: "/tmp/demo",
            name: "Demo",
            last_opened: Math.floor(Date.now() / 1000),
            valid: true,
            root_count: 0,
            album_count: 0,
            tag_count: 0,
            read_only: false,
          },
        ]}
        {...defaultProps}
      />,
    );

    await user.click(screen.getByText("/tmp/demo"));
    expect(onOpenRecent).toHaveBeenCalledWith("/tmp/demo");
  });

  it("unregisters workspace", async () => {
    const user = userEvent.setup();
    render(
      <StartPage
        recent={[
          {
            path: "/tmp/demo",
            name: "Demo",
            last_opened: Math.floor(Date.now() / 1000),
            valid: true,
            root_count: 0,
            album_count: 0,
            tag_count: 0,
            read_only: false,
          },
        ]}
        {...defaultProps}
      />,
    );

    await user.click(screen.getByTitle("Remove workspace"));
    expect(onRemoveRecent).toHaveBeenCalledWith("/tmp/demo");
  });

  it("shows floating error toast", () => {
    render(
      <StartPage
        recent={[]}
        {...defaultProps}
        notification={errorNotification(
          "This folder is not a workspace. Use Open Workspace and choose a folder that contains workspace.json.",
        )}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent("not a workspace");
  });
});
