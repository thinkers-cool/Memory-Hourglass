import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../../api/client";
import { SmbFolderTree } from "./SmbFolderTree";

vi.mock("../../api/client", () => ({
  listFolderChildren: vi.fn(),
}));

describe("SmbFolderTree", () => {
  beforeEach(() => {
    vi.mocked(api.listFolderChildren).mockReset();
    vi.mocked(api.listFolderChildren).mockResolvedValue([
      { name: "Photos", path: "/smb/root/Photos" },
      { name: "Videos", path: "/smb/root/Videos" },
    ]);
  });

  it("loads and renders folder children", async () => {
    render(
      <SmbFolderTree
        rootPath="/smb/root"
        rootLabel="NAS"
        selectedRelativePath=""
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByText("NAS")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText("Photos")).toBeInTheDocument();
      expect(screen.getByText("Videos")).toBeInTheDocument();
    });
  });

  it("selects a folder and expands nested children", async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    vi.mocked(api.listFolderChildren)
      .mockResolvedValueOnce([{ name: "Photos", path: "/smb/root/Photos" }])
      .mockResolvedValueOnce([{ name: "2024", path: "/smb/root/Photos/2024" }]);

    render(
      <SmbFolderTree
        rootPath="/smb/root"
        rootLabel="NAS"
        selectedRelativePath=""
        onSelect={onSelect}
      />,
    );
    await waitFor(() => expect(screen.getByText("Photos")).toBeInTheDocument());

    await user.click(screen.getByText("Photos"));
    expect(onSelect).toHaveBeenCalledWith("Photos");

    await user.click(screen.getAllByLabelText("Expand folder")[0]);
    await waitFor(() => expect(screen.getByText("2024")).toBeInTheDocument());
  });

  it("collapses expanded folders", async () => {
    const user = userEvent.setup();
    render(
      <SmbFolderTree
        rootPath="/smb/root"
        rootLabel="NAS"
        selectedRelativePath=""
        onSelect={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByText("Photos")).toBeInTheDocument());

    await user.click(screen.getAllByLabelText("Collapse folder")[0]);
    expect(screen.queryByText("Photos")).not.toBeInTheDocument();
    expect(screen.queryByText("Videos")).not.toBeInTheDocument();
  });

  it("handles load errors by showing empty children", async () => {
    vi.mocked(api.listFolderChildren).mockRejectedValue(new Error("read failed"));
    render(
      <SmbFolderTree
        rootPath="/smb/root"
        rootLabel="NAS"
        selectedRelativePath=""
        onSelect={vi.fn()}
      />,
    );
    await waitFor(() => expect(api.listFolderChildren).toHaveBeenCalled());
    expect(screen.getByText("NAS")).toBeInTheDocument();
  });

  it("loads nested folders while siblings stay unloaded", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listFolderChildren)
      .mockResolvedValueOnce([
        { name: "Photos", path: "/smb/root/Photos" },
        { name: "Videos", path: "/smb/root/Videos" },
      ])
      .mockResolvedValueOnce([{ name: "2024", path: "/smb/root/Photos/2024" }]);
    render(
      <SmbFolderTree
        rootPath="/smb/root"
        rootLabel="NAS"
        selectedRelativePath=""
        onSelect={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByText("Photos")).toBeInTheDocument());
    await user.click(screen.getAllByLabelText("Expand folder")[0]);
    await waitFor(() => expect(screen.getByText("2024")).toBeInTheDocument());
  });

  it("toggles folders from chevron Enter and Space keys", async () => {
    const user = userEvent.setup();
    render(
      <SmbFolderTree
        rootPath="/smb/root"
        rootLabel="NAS"
        selectedRelativePath=""
        onSelect={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByText("Photos")).toBeInTheDocument());
    const chevron = screen.getAllByLabelText("Collapse folder")[0];
    chevron.focus();
    await user.keyboard("{Enter}");
    expect(screen.queryByText("Photos")).not.toBeInTheDocument();
    await user.keyboard(" ");
    await waitFor(() => expect(screen.getByText("Photos")).toBeInTheDocument());
  });
});
