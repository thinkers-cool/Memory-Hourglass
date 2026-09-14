import { describe, expect, it } from "vitest";
import {
  isActiveScan,
  isProgressActive,
  resolveStatusDisplay,
  resolveStatusLine,
  resolveSystemStatus,
  resolveProgressStatus,
  formatScanStatus,
} from "./statusBar";
import { errorNotification, successNotification } from "./notification";
import type { RootStats } from "../types";

const roots: RootStats[] = [
  {
    id: 1,
    path: "/photos",
    kind: "local",
    status: "idle",
    scan_policy: "watch",
    poll_secs: null,
    last_scan_at: null,
    asset_count: 10,
    missing_count: 0,
  },
];

describe("isActiveScan", () => {
  it("treats empty and done statuses as inactive", () => {
    expect(isActiveScan("")).toBe(false);
    expect(isActiveScan("done: 10 files")).toBe(false);
  });

  it("treats in-progress statuses as active", () => {
    expect(isActiveScan("indexing: 1/2")).toBe(true);
  });
});

describe("resolveStatusDisplay", () => {
  it("splits alert, flash, and context layers", () => {
    const display = resolveStatusDisplay({
      selectedCount: 2,
      scanStatus: "",
      roots,
      exportActive: false,
      exportProgress: null,
      notification: successNotification("Tagged 2 item(s)"),
      busy: false,
    });

    expect(display.flash?.text).toBe("Tagged 2 item(s)");
    expect(display.alert).toBeNull();
    expect(display.context).toBe("2 selected");
    expect(display.progress).toBeNull();
  });

  it("treats errors as sticky alerts", () => {
    const display = resolveStatusDisplay({
      selectedCount: 0,
      scanStatus: "",
      roots,
      exportActive: false,
      exportProgress: null,
      notification: errorNotification("disk full"),
      busy: false,
    });

    expect(display.alert?.text).toBe("disk full");
    expect(display.flash).toBeNull();
  });
});

describe("resolveSystemStatus", () => {
  it("shows progress over flash and context", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 3,
        scanStatus: "indexing: 1/2",
        roots,
        exportActive: true,
        exportProgress: {
          done: 1,
          total: 5,
          phase: "running",
          message: "export",
        },
        notification: successNotification("Tagged 2 item(s)"),
        busy: false,
      }),
    ).toBe("Export 1/5");
  });

  it("shows export progress when running", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: true,
        exportProgress: {
          done: 2,
          total: 10,
          phase: "running",
          message: "export",
        },
        notification: null,
        busy: false,
      }),
    ).toBe("Export 2/10");
  });

  it("shows scan status when active", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "indexing: 4/10",
        roots,
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("Previews 4/10");
  });

  it("shows source health summary", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        roots: [{ ...roots[0], status: "offline", missing_count: 2 }],
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("1 offline · 2 missing");
  });

  it("scopes source health to the active root filter", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        activeRootId: 1,
        roots: [
          { ...roots[0], missing_count: 0 },
          { ...roots[0], id: 2, path: "/archive", missing_count: 579 },
        ],
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("Ready");
  });

  it("shows per-root scan progress from scan events", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        scanStatusByRoot: { 1: "indexing: 4/10" },
        roots,
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("photos: Previews 4/10");
  });

  it("prefers focused root when multiple scans are active", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        scanStatusByRoot: {
          1: "scanning: 1/10",
          2: "cataloging: 3/8",
        },
        focusScanRootId: 2,
        roots: [
          roots[0],
          { ...roots[0], id: 2, path: "/archive" },
        ],
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("archive: Catalog 3/8");
  });

  it("falls back to the raw path when the root label is empty", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        scanStatusByRoot: { 1: "indexing: 1/2" },
        roots: [{ ...roots[0], path: "/" }],
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("/: Previews 1/2");
  });

  it("uses the root id when the scanning root is unknown", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        scanStatusByRoot: { 99: "indexing: 1/2" },
        roots,
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("#99: Previews 1/2");
  });

  it("shows plural scan summary when multiple scans are active", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        scanStatusByRoot: {
          1: "scanning: 1/10",
          2: "cataloging: 3/8",
        },
        roots: [
          roots[0],
          { ...roots[0], id: 2, path: "/archive" },
        ],
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("Scanning 2 sources");
  });

  it("falls back to plural scan summary when focus does not match", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        scanStatusByRoot: {
          1: "scanning: 1/10",
          2: "cataloging: 3/8",
        },
        focusScanRootId: 99,
        roots: [
          roots[0],
          { ...roots[0], id: 2, path: "/archive" },
        ],
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("Scanning 2 sources");
  });

  it("shows last action notification", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: false,
        exportProgress: null,
        notification: successNotification("Tagged 2 item(s)"),
        busy: false,
      }),
    ).toBe("Tagged 2 item(s)");
  });

  it("shows error notification text", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: false,
        exportProgress: null,
        notification: errorNotification("disk full"),
        busy: false,
      }),
    ).toBe("disk full");
  });

  it("falls back to ready", () => {
    expect(
      resolveSystemStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: false,
      }),
    ).toBe("Ready");
  });
});

describe("isProgressActive", () => {
  it("is true when busy", () => {
    expect(
      isProgressActive({
        busy: true,
        scanStatus: "",
        exportActive: false,
        exportProgress: null,
      }),
    ).toBe(true);
  });

  it("is true during scan", () => {
    expect(
      isProgressActive({
        busy: false,
        scanStatus: "indexing: 1/5",
        exportActive: false,
        exportProgress: null,
      }),
    ).toBe(true);
  });

  it("is true during export", () => {
    expect(
      isProgressActive({
        busy: false,
        scanStatus: "",
        exportActive: true,
        exportProgress: {
          done: 1,
          total: 5,
          phase: "running",
          message: "export",
        },
      }),
    ).toBe(true);
  });

  it("is false when idle", () => {
    expect(
      isProgressActive({
        busy: false,
        scanStatus: "",
        exportActive: false,
        exportProgress: null,
      }),
    ).toBe(false);
  });

  it("is false for completed scan status", () => {
    expect(
      isProgressActive({
        busy: false,
        scanStatus: "done: 10 files",
        exportActive: false,
        exportProgress: null,
      }),
    ).toBe(false);
  });

  it("is false when export is active without progress", () => {
    expect(
      isProgressActive({
        busy: false,
        scanStatus: "",
        exportActive: true,
        exportProgress: null,
      }),
    ).toBe(false);
  });
});

describe("resolveProgressStatus", () => {
  it("shows busy message and export file progress", () => {
    expect(
      resolveProgressStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: true,
        busyMessage: " Opening workspace ",
      })?.text,
    ).toBe("Opening workspace");

    expect(
      resolveProgressStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: true,
        exportProgress: {
          done: 1,
          total: 3,
          phase: "running",
          message: "copying",
          file_name: "a.jpg",
        },
        notification: null,
        busy: false,
      })?.text,
    ).toContain("a.jpg");
  });

  it("shows working text when busy without message", () => {
    expect(
      resolveProgressStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: false,
        exportProgress: null,
        notification: null,
        busy: true,
      })?.text,
    ).toBe("Working");
  });

  it("formats cataloging and scanning statuses", () => {
    expect(formatScanStatus("cataloging: 1/10")).toContain("Catalog");
    expect(formatScanStatus("scanning: 2/10")).toContain("Scanning");
  });

  it("returns raw scan status for unknown prefixes", () => {
    expect(formatScanStatus("queued: 1/10")).toBe("queued: 1/10");
  });

  it("formats scan statuses without count segments", () => {
    expect(formatScanStatus("cataloging:")).toContain("Catalog");
    expect(formatScanStatus("indexing:")).toContain("Previews");
    expect(formatScanStatus("scanning:")).toContain("Scanning");
  });

  it("shows export progress without file name", () => {
    expect(
      resolveProgressStatus({
        selectedCount: 0,
        scanStatus: "",
        roots,
        exportActive: true,
        exportProgress: {
          done: 2,
          total: 4,
          phase: "running",
          message: "export",
        },
        notification: null,
        busy: false,
      })?.text,
    ).toBe("Export 2/4");
  });
});

describe("resolveStatusLine", () => {
  it("prefers progress over alert, flash, and context", () => {
    const display = resolveStatusDisplay({
      selectedCount: 2,
      scanStatus: "indexing: 1/5",
      roots,
      exportActive: false,
      exportProgress: null,
      notification: errorNotification("disk full"),
      busy: false,
    });
    expect(resolveStatusLine(display)).toContain("Previews");
  });

  it("falls back through alert, flash, and context", () => {
    const alertDisplay = resolveStatusDisplay({
      selectedCount: 0,
      scanStatus: "",
      roots,
      exportActive: false,
      exportProgress: null,
      notification: errorNotification("disk full"),
      busy: false,
    });
    expect(resolveStatusLine(alertDisplay)).toBe("disk full");

    const flashDisplay = resolveStatusDisplay({
      selectedCount: 0,
      scanStatus: "",
      roots,
      exportActive: false,
      exportProgress: null,
      notification: successNotification("Tagged 2 item(s)"),
      busy: false,
    });
    expect(resolveStatusLine(flashDisplay)).toBe("Tagged 2 item(s)");
    expect(resolveStatusLine({ ...flashDisplay, flash: null })).toBe("Ready");
  });
});

describe("isActiveScan edge cases", () => {
  it("treats error statuses as inactive", () => {
    expect(isActiveScan("error: disk full")).toBe(false);
  });
});
