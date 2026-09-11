import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../../api/client";
import { SmbConnectDialog } from "./SmbConnectDialog";

const pickFolder = vi.hoisted(() => vi.fn());

vi.mock("../../api/client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../api/client")>();
  return {
    ...actual,
    listSmbShares: vi.fn(),
    mountSmbForBrowse: vi.fn(),
    listFolderChildren: vi.fn(),
  };
});

vi.mock("../../lib/pickFolder", () => ({
  pickFolder,
}));

describe("SmbConnectDialog", () => {
  beforeEach(() => {
    pickFolder.mockReset();
    vi.mocked(api.listSmbShares).mockReset();
    vi.mocked(api.mountSmbForBrowse).mockReset();
    vi.mocked(api.listFolderChildren).mockReset();
  });

  it("lists shares, browses folders, then connects", async () => {
    const user = userEvent.setup();
    const onConnect = vi.fn();
    vi.mocked(api.listSmbShares).mockResolvedValue([
      { name: "media", comment: "Photos" },
    ]);
    vi.mocked(api.mountSmbForBrowse).mockResolvedValue("/tmp/media");
    vi.mocked(api.listFolderChildren).mockResolvedValue([
      { name: "Photos", path: "/tmp/media/Photos" },
    ]);

    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={onConnect}
        onAddMountedPath={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "192.168.1.1");
    await user.type(screen.getByLabelText("Username"), "admin");
    await user.type(screen.getByLabelText("Password"), "secret");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(screen.getByText("media")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Continue" }));

    await waitFor(() => {
      expect(api.mountSmbForBrowse).toHaveBeenCalledWith({
        host: "192.168.1.1",
        share: "media",
        username: "admin",
        password: "secret",
      });
    });

    await waitFor(() => {
      expect(screen.getByText("Photos")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Photos" }));
    await user.click(screen.getByRole("button", { name: "Add" }));

    expect(onConnect).toHaveBeenCalledWith({
      host: "192.168.1.1",
      share: "media",
      username: "admin",
      password: "secret",
      pollSecs: 300,
      folderPath: "Photos",
    });
  });

  it("returns null when closed", () => {
    const { container } = render(
      <SmbConnectDialog
        open={false}
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("picks a mounted folder path", async () => {
    const user = userEvent.setup();
    const onAddMountedPath = vi.fn();
    pickFolder.mockResolvedValue("/Volumes/nas/photos");
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={onAddMountedPath}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Choose folder" }));
    expect(pickFolder).toHaveBeenCalled();
    expect(onAddMountedPath).toHaveBeenCalledWith("/Volumes/nas/photos");
  });

  it("shows error when no shares are returned", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listSmbShares).mockResolvedValue([]);
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => {
      expect(screen.getByText("No shares found.")).toBeInTheDocument();
    });
  });

  it("shows API errors from sign in and mount", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listSmbShares).mockRejectedValue(
      JSON.stringify({ code: "io", message: "connection refused" }),
    );
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => {
      expect(screen.getByText("connection refused")).toBeInTheDocument();
    });

    vi.mocked(api.listSmbShares).mockResolvedValue([{ name: "media", comment: "" }]);
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(screen.getByText("media")).toBeInTheDocument());
    vi.mocked(api.mountSmbForBrowse).mockRejectedValue(
      JSON.stringify({ code: "io", message: "mount failed" }),
    );
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => {
      expect(screen.getByText("mount failed")).toBeInTheDocument();
    });
  });

  it("navigates back from shares and folders steps", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listSmbShares).mockResolvedValue([{ name: "media", comment: "" }]);
    vi.mocked(api.mountSmbForBrowse).mockResolvedValue("/tmp/media");
    vi.mocked(api.listFolderChildren).mockResolvedValue([]);
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(screen.getByText("media")).toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByLabelText("Username")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(screen.getByText("media")).toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(api.mountSmbForBrowse).toHaveBeenCalled());
    await user.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() => expect(screen.getByText("media")).toBeInTheDocument());
  });

  it("includes custom poll interval on connect", async () => {
    const user = userEvent.setup();
    const onConnect = vi.fn();
    vi.mocked(api.listSmbShares).mockResolvedValue([{ name: "media", comment: "" }]);
    vi.mocked(api.mountSmbForBrowse).mockResolvedValue("/tmp/media");
    vi.mocked(api.listFolderChildren).mockResolvedValue([]);
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={onConnect}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    const pollInput = screen.getByRole("spinbutton");
    fireEvent.change(pollInput, { target: { value: "120" } });
    await user.click(screen.getByRole("button", { name: "Add" }));
    expect(onConnect).toHaveBeenCalledWith(
      expect.objectContaining({ pollSecs: 120 }),
    );
  });

  it("ignores cancelled mounted folder pick", async () => {
    const user = userEvent.setup();
    pickFolder.mockResolvedValue(null);
    const onAddMountedPath = vi.fn();
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={onAddMountedPath}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Choose folder" }));
    expect(onAddMountedPath).not.toHaveBeenCalled();
  });

  it("switches to connect mode and selects inactive share", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listSmbShares).mockResolvedValue([
      { name: "media", comment: "" },
      { name: "backup", comment: "" },
    ]);
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(screen.getByText("backup")).toBeInTheDocument());
    await user.click(screen.getByText("backup"));
  });

  it("switches back to mounted mode", async () => {
    const user = userEvent.setup();
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.click(screen.getByRole("button", { name: "Mounted folder" }));
    expect(screen.getByRole("button", { name: "Choose folder" })).toBeInTheDocument();
  });

  it("ignores sign in and continue when credentials are incomplete", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listSmbShares).mockResolvedValue([{ name: "media", comment: "" }]);
    vi.mocked(api.mountSmbForBrowse).mockResolvedValue("/tmp/media");
    vi.mocked(api.listFolderChildren).mockResolvedValue([]);
    const onConnect = vi.fn();
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={onConnect}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(api.listSmbShares).not.toHaveBeenCalled();
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(screen.getByText("media")).toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(api.mountSmbForBrowse).toHaveBeenCalled());
    await user.click(screen.getByRole("button", { name: "Back" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    fireEvent.change(screen.getByRole("spinbutton"), { target: { value: "" } });
    await user.click(screen.getByRole("button", { name: "Add" }));
    expect(onConnect).toHaveBeenCalledWith(
      expect.objectContaining({ pollSecs: 300, folderPath: undefined }),
    );
  });

  it("selects first share when listing returns unnamed entries", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listSmbShares).mockResolvedValue([{ name: "", comment: "" }]);
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Continue" })).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(api.mountSmbForBrowse).not.toHaveBeenCalled();
  });

  it("ignores sign in without username", async () => {
    const user = userEvent.setup();
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Password"), "pass");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(api.listSmbShares).not.toHaveBeenCalled();
  });

  it("ignores sign in without password", async () => {
    const user = userEvent.setup();
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(api.listSmbShares).not.toHaveBeenCalled();
  });

  it("ignores sign in with whitespace username", async () => {
    const user = userEvent.setup();
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "   ");
    await user.type(screen.getByLabelText("Password"), "pass");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(api.listSmbShares).not.toHaveBeenCalled();
  });

  it("selects share when listing returns entries without names", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listSmbShares).mockResolvedValue([{ name: undefined as unknown as string, comment: "" }]);
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={vi.fn()}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Continue" })).toBeInTheDocument());
  });

  it("ignores add when mount path is empty", async () => {
    const user = userEvent.setup();
    const onConnect = vi.fn();
    vi.mocked(api.listSmbShares).mockResolvedValue([{ name: "media", comment: "" }]);
    vi.mocked(api.mountSmbForBrowse).mockResolvedValue("");
    render(
      <SmbConnectDialog
        open
        busy={false}
        onClose={vi.fn()}
        onConnect={onConnect}
        onAddMountedPath={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Connect SMB" }));
    await user.type(screen.getByPlaceholderText("192.168.1.10 or smb://nas.local"), "10.0.0.1");
    await user.type(screen.getByLabelText("Username"), "user");
    await user.type(screen.getByLabelText("Password"), "pass");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Add" })).toBeDisabled());
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(onConnect).not.toHaveBeenCalled();
  });
});
