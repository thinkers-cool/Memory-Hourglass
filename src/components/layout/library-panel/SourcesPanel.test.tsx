import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { emptyFilterBar, mockLibraryActions } from "../../../test/fixtures";
import type { RootStats } from "../../../types";
import { SourcesPanel } from "./SourcesPanel";

const roots: RootStats[] = [
  {
    id: 1,
    path: "/photos/A",
    kind: "local",
    status: "idle",
    scan_policy: "watch",
    poll_secs: null,
    last_scan_at: null,
    asset_count: 2,
    missing_count: 0,
  },
  {
    id: 2,
    path: "/photos/B",
    kind: "smb",
    status: "offline",
    scan_policy: "watch",
    poll_secs: null,
    last_scan_at: null,
    asset_count: 0,
    missing_count: 0,
  },
];

describe("SourcesPanel", () => {
  it("renders roots and toggles section order controls", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <SourcesPanel
        workspaceId="ws-sources"
        roots={roots}
        albums={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        scanStatusByRoot={{ 1: "indexing: 1/2" }}
        actions={actions}
      />,
    );

    expect(screen.getByText("A")).toBeInTheDocument();
    expect(screen.getByText("B")).toBeInTheDocument();
    expect(screen.getByText("Offline")).toBeInTheDocument();

    const orderButtons = screen.getAllByRole("button", {
      name: "Name (A to Z)",
    });
    await user.click(orderButtons[0]);
    await user.click(orderButtons[1]);
    await user.click(orderButtons[2]);
  });

  it("wires source row actions", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <SourcesPanel
        workspaceId="ws-sources-actions"
        roots={[roots[0]]}
        albums={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );

    await user.click(screen.getByText("A"));
    expect(actions.selectRoot).toHaveBeenCalledWith(1);
    await user.click(screen.getByLabelText("Sync"));
    expect(actions.syncRoot).toHaveBeenCalledWith(1);
    await user.click(screen.getByLabelText("Relink"));
    expect(actions.relinkRoot).toHaveBeenCalledWith(1);
    await user.click(screen.getByLabelText("Remove"));
    expect(actions.removeRoot).toHaveBeenCalledWith(1);
  });

  it("opens smb connect and local folder actions", async () => {
    const user = userEvent.setup();
    const actions = mockLibraryActions();
    render(
      <SourcesPanel
        workspaceId="ws-sources-header"
        roots={[]}
        albums={[]}
        tags={[]}
        filterBar={emptyFilterBar}
        extraFilter={{}}
        deleteStatus=""
        deletedCount={0}
        busy={false}
        actions={actions}
      />,
    );

    await user.click(screen.getByLabelText("Add folder"));
    expect(actions.addLocalRoot).toHaveBeenCalled();
    await user.click(screen.getByLabelText("Connect SMB share"));
    expect(actions.openSmbConnect).toHaveBeenCalled();
  });
});
