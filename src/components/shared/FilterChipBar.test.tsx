import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FilterChipBar } from "./FilterChipBar";

const filters = [
  { id: "camera", label: "Camera", type: "text" as const },
  { id: "rating", label: "Rating", type: "text" as const },
];

function mockChipBarWidth(width: number, scrollWidth: number) {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
    width,
    height: 32,
    top: 0,
    left: 0,
    right: width,
    bottom: 32,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  } as DOMRect);

  Object.defineProperty(HTMLElement.prototype, "clientWidth", {
    configurable: true,
    get() {
      return width;
    },
  });
  Object.defineProperty(HTMLElement.prototype, "scrollWidth", {
    configurable: true,
    get() {
      return scrollWidth;
    },
  });
}

describe("FilterChipBar", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });
  it("shows clear all when filters are active", async () => {
    const user = userEvent.setup();
    const onClearAll = vi.fn();
    render(
      <FilterChipBar
        filters={filters}
        values={{ camera: "Sony" }}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={onClearAll}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Clear all/i }));
    expect(onClearAll).toHaveBeenCalledTimes(1);
  });

  it("hides chip label but keeps chip style in compact mode", () => {
    render(
      <FilterChipBar
        compact
        filters={filters}
        values={{ camera: "Sony" }}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    expect(screen.queryByText("Camera")).not.toBeInTheDocument();
    expect(screen.getByText("Sony")).toBeInTheDocument();
    expect(screen.getByTitle("Camera: Sony")).toBeInTheDocument();
  });

  it("collapses chip labels when content overflows", () => {
    mockChipBarWidth(200, 260);
    render(
      <FilterChipBar
        filters={filters}
        values={{ camera: "Sony" }}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    expect(screen.queryByText("Camera")).not.toBeInTheDocument();
    expect(screen.getByText("Sony")).toBeInTheDocument();
  });

  it("restores chip labels when container grows enough", () => {
    let triggerResize: (() => void) | undefined;
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: ResizeObserverCallback) {
          triggerResize = () => callback([], this as ResizeObserver);
        }
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );

    mockChipBarWidth(200, 260);
    const props = {
      filters,
      values: { camera: "Sony" },
      onFilterChange: vi.fn(),
      onFilterRemove: vi.fn(),
      onClearAll: vi.fn(),
    };
    render(<FilterChipBar {...props} />);
    expect(screen.queryByText("Camera")).not.toBeInTheDocument();

    mockChipBarWidth(300, 220);
    act(() => {
      triggerResize?.();
    });
    expect(screen.getByText("Camera")).toBeInTheDocument();
    expect(screen.getByText("Sony")).toBeInTheDocument();
  });

  it("keeps text filter input open while interacting", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[
          { id: "metadata", label: "Metadata", type: "text", debounceMs: 300 },
        ]}
        values={{ metadata: "" }}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Metadata" }));

    const input = screen.getByPlaceholderText("Filter by Metadata");
    await user.click(input);
    await user.type(input, "lens");

    expect(input).toHaveFocus();
    expect(input).toHaveValue("lens");
    expect(onFilterChange).toHaveBeenCalledTimes(1);
    expect(onFilterChange).toHaveBeenCalledWith("metadata", "");
  });

  it("opens filter menu", async () => {
    const user = userEvent.setup();
    render(
      <FilterChipBar
        filters={filters}
        values={{}}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Filter/i }));
    expect(screen.getByText("Camera")).toBeInTheDocument();
  });

  it("opens filter menu from focus event", () => {
    render(
      <FilterChipBar
        filters={filters}
        values={{}}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    act(() => {
      window.dispatchEvent(new Event("memhg:focus-filter"));
    });
    expect(screen.getByText("Camera")).toBeInTheDocument();
  });

  it("adds single status filter with default option", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[
          {
            id: "sync",
            label: "Sync",
            type: "status",
            statusOptions: ["ok", "missing"],
            statusOptionLabels: { ok: "OK", missing: "Missing" },
          },
        ]}
        values={{}}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Filter/i }));
    await user.click(screen.getByRole("button", { name: "Sync" }));
    expect(onFilterChange).toHaveBeenCalledWith("sync", "ok");
  });

  it("adds date filter as pending chip", async () => {
    const user = userEvent.setup();
    render(
      <FilterChipBar
        filters={[{ id: "capture", label: "Capture", type: "date" }]}
        values={{}}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
        onDateChange={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Filter/i }));
    await user.click(screen.getByRole("button", { name: "Capture" }));
    expect(screen.getByRole("button", { name: "Capture" })).toBeInTheDocument();
  });

  it("adds multi status filter as pending chip", async () => {
    const user = userEvent.setup();
    render(
      <FilterChipBar
        filters={[
          {
            id: "sync",
            label: "Sync",
            type: "status",
            multi: true,
            statusOptions: ["ok", "missing"],
          },
        ]}
        values={{}}
        multiValues={{}}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Filter/i }));
    await user.click(screen.getByRole("button", { name: "Sync" }));
    expect(screen.getByRole("button", { name: "Sync" })).toBeInTheDocument();
  });

  it("removes active filter chip", async () => {
    const user = userEvent.setup();
    const onFilterRemove = vi.fn();
    render(
      <FilterChipBar
        filters={filters}
        values={{ camera: "Sony" }}
        onFilterChange={vi.fn()}
        onFilterRemove={onFilterRemove}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Remove Camera" }));
    expect(onFilterRemove).toHaveBeenCalledWith("camera");
  });

  it("clears debounced text filter on escape", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[
          { id: "metadata", label: "Metadata", type: "text", debounceMs: 300 },
        ]}
        values={{ metadata: "lens" }}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getAllByRole("button", { name: /Metadata/i })[0]!);
    const input = screen.getByPlaceholderText("Filter by Metadata");
    await user.type(input, "{Escape}");
    expect(onFilterChange).toHaveBeenCalledWith("metadata", "");
  });

  it("shows date filter chip and clears on remove", async () => {
    const user = userEvent.setup();
    const onDateChange = vi.fn();
    const onFilterRemove = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "capture", label: "Capture", type: "date" }]}
        values={{}}
        dateGte="2024-01-01"
        dateLte="2024-12-31"
        onFilterChange={vi.fn()}
        onFilterRemove={onFilterRemove}
        onClearAll={vi.fn()}
        onDateChange={onDateChange}
      />,
    );
    expect(screen.getByText("Capture")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Remove Capture" }));
    expect(onDateChange).toHaveBeenCalledWith("", "");
    expect(onFilterRemove).toHaveBeenCalledWith("capture");
  });

  it("closes open chip when filters rerender", async () => {
    const user = userEvent.setup();
    const props = {
      filters,
      values: { camera: "Sony" },
      onFilterChange: vi.fn(),
      onFilterRemove: vi.fn(),
      onClearAll: vi.fn(),
    };
    const { rerender } = render(<FilterChipBar {...props} />);
    await user.click(screen.getByText("Sony").closest("button")!);
    expect(screen.getByPlaceholderText("Filter by Camera")).toBeInTheDocument();
    rerender(
      <FilterChipBar
        {...props}
        filters={[
          ...filters,
          { id: "metadata", label: "Metadata", type: "text" },
        ]}
      />,
    );
    expect(
      screen.queryByPlaceholderText("Filter by Camera"),
    ).not.toBeInTheDocument();
  });

  it("dismisses pending text chip without a value", async () => {
    const user = userEvent.setup();
    const onFilterRemove = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "metadata", label: "Metadata", type: "text" }]}
        values={{}}
        onFilterChange={vi.fn()}
        onFilterRemove={onFilterRemove}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Metadata" }));
    await user.click(screen.getByText("Metadata").closest("button")!);
    expect(onFilterRemove).toHaveBeenCalledWith("metadata");
  });

  it("closes chip popover when toggled again", async () => {
    const user = userEvent.setup();
    render(
      <FilterChipBar
        filters={filters}
        values={{ camera: "Sony" }}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    const chip = screen.getByText("Sony").closest("button")!;
    await user.click(chip);
    expect(screen.getByPlaceholderText("Filter by Camera")).toBeInTheDocument();
    await user.click(chip);
    expect(
      screen.queryByPlaceholderText("Filter by Camera"),
    ).not.toBeInTheDocument();
  });

  it("skips debounced onChange when draft matches committed value", async () => {
    vi.useFakeTimers();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[
          { id: "metadata", label: "Metadata", type: "text", debounceMs: 300 },
        ]}
        values={{ metadata: "lens" }}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    fireEvent.click(screen.getAllByRole("button", { name: /Metadata/i })[0]!);
    const input = screen.getByPlaceholderText("Filter by Metadata");
    fireEvent.change(input, { target: { value: "lensx" } });
    fireEvent.change(input, { target: { value: "lens" } });
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    expect(onFilterChange).not.toHaveBeenCalledWith("metadata", "lens");
  });

  it("debounces text filter changes", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[
          { id: "metadata", label: "Metadata", type: "text", debounceMs: 300 },
        ]}
        values={{ metadata: "" }}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Metadata" }));
    const input = await screen.findByPlaceholderText("Filter by Metadata");
    fireEvent.change(input, { target: { value: "lens" } });
    expect(onFilterChange).toHaveBeenCalledWith("metadata", "");
    onFilterChange.mockClear();
    await waitFor(() => {
      expect(onFilterChange).toHaveBeenCalledWith("metadata", "lens");
    });
  });

  it("updates multi status filter selections", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[
          {
            id: "sync",
            label: "Sync",
            type: "status",
            multi: true,
            statusOptions: ["ok", "missing"],
            statusOptionLabels: { ok: "OK", missing: "Missing" },
          },
        ]}
        values={{}}
        multiValues={{ sync: ["ok"] }}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByText("OK").closest("button")!);
    await user.click(screen.getByRole("button", { name: "Missing" }));
    expect(onFilterChange).toHaveBeenCalledWith("sync", "missing");
  });

  it("opens date chip popover and applies changes", async () => {
    const user = userEvent.setup();
    const onDateChange = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "capture", label: "Capture", type: "date" }]}
        values={{}}
        dateGte="2024-01-01"
        dateLte=""
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
        onDateChange={onDateChange}
      />,
    );
    await user.click(screen.getByText(/from 2024-01-01/).closest("button")!);
    fireEvent.change(screen.getByTitle("To"), {
      target: { value: "2024-12-31" },
    });
    expect(onDateChange).toHaveBeenCalledWith("2024-01-01", "2024-12-31");
  });

  it("renders chip without icon for unknown filter ids", () => {
    render(
      <FilterChipBar
        filters={[{ id: "custom", label: "Custom", type: "text" }]}
        values={{ custom: "value" }}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    expect(screen.getByText("Custom")).toBeInTheDocument();
    expect(screen.getByText("value")).toBeInTheDocument();
  });

  it("focuses text input when chip opens", async () => {
    const user = userEvent.setup();
    render(
      <FilterChipBar
        filters={[{ id: "metadata", label: "Metadata", type: "text" }]}
        values={{ metadata: "lens" }}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByText("lens").closest("button")!);
    await waitFor(() => {
      expect(screen.getByPlaceholderText("Filter by Metadata")).toHaveFocus();
    });
  });

  it("closes add-filter menu on outside click", async () => {
    const user = userEvent.setup();
    render(
      <>
        <FilterChipBar
          filters={filters}
          values={{}}
          onFilterChange={vi.fn()}
          onFilterRemove={vi.fn()}
          onClearAll={vi.fn()}
        />
        <button type="button">Outside</button>
      </>,
    );
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    expect(screen.getByRole("button", { name: "Camera" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(
      screen.queryByRole("button", { name: "Camera" }),
    ).not.toBeInTheDocument();
  });

  it("applies immediate text filter changes without debounce", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "metadata", label: "Metadata", type: "text" }]}
        values={{ metadata: "" }}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Metadata" }));
    const input = screen.getByPlaceholderText("Filter by Metadata");
    fireEvent.change(input, { target: { value: "raw" } });
    expect(onFilterChange).toHaveBeenLastCalledWith("metadata", "raw");
  });

  it("closes single status filter after selecting an option", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[
          {
            id: "sync",
            label: "Sync",
            type: "status",
            statusOptions: ["ok", "missing"],
            statusOptionLabels: { ok: "OK", missing: "Missing" },
          },
        ]}
        values={{ sync: "ok" }}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByText("OK").closest("button")!);
    await user.click(screen.getByRole("button", { name: "Missing" }));
    expect(onFilterChange).toHaveBeenCalledWith("sync", "missing");
    expect(
      screen.queryByRole("button", { name: "Missing" }),
    ).not.toBeInTheDocument();
  });

  it("closes text chip on enter", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "metadata", label: "Metadata", type: "text" }]}
        values={{}}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Metadata" }));
    const input = screen.getByPlaceholderText("Filter by Metadata");
    fireEvent.change(input, { target: { value: "lens" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onFilterChange).toHaveBeenCalledWith("metadata", "lens");
    expect(
      screen.queryByPlaceholderText("Filter by Metadata"),
    ).not.toBeInTheDocument();
  });

  it("removes date filter through chip remove handler", async () => {
    const user = userEvent.setup();
    const onDateChange = vi.fn();
    const onFilterRemove = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "capture", label: "Capture", type: "date" }]}
        values={{}}
        dateGte="2024-01-01"
        dateLte=""
        onFilterChange={vi.fn()}
        onFilterRemove={onFilterRemove}
        onClearAll={vi.fn()}
        onDateChange={onDateChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Remove Capture" }));
    expect(onDateChange).toHaveBeenCalledWith("", "");
    expect(onFilterRemove).toHaveBeenCalledWith("capture");
  });

  it("adds single status filter without predefined options", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "sync", label: "Sync", type: "status" }]}
        values={{}}
        onFilterChange={onFilterChange}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Filter/i }));
    await user.click(screen.getByRole("button", { name: "Sync" }));
    expect(onFilterChange).toHaveBeenCalledWith("sync", "");
  });

  it("clears pending chip id after value is applied", async () => {
    const user = userEvent.setup();
    const props = {
      filters: [{ id: "metadata", label: "Metadata", type: "text" as const }],
      values: { metadata: "" },
      onFilterChange: vi.fn(),
      onFilterRemove: vi.fn(),
      onClearAll: vi.fn(),
    };
    const { rerender } = render(<FilterChipBar {...props} />);
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Metadata" }));
    rerender(<FilterChipBar {...props} values={{ metadata: "lens" }} />);
    expect(screen.getByText("lens")).toBeInTheDocument();
  });

  it("removes pending filter chip through chip remove handler", async () => {
    const user = userEvent.setup();
    const onFilterRemove = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "metadata", label: "Metadata", type: "text" }]}
        values={{}}
        onFilterChange={vi.fn()}
        onFilterRemove={onFilterRemove}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Add filter/i }));
    await user.click(screen.getByRole("button", { name: "Metadata" }));
    await user.click(screen.getByRole("button", { name: "Remove Metadata" }));
    expect(onFilterRemove).toHaveBeenCalledWith("metadata");
  });

  it("removes open filter chip through chip remove handler", async () => {
    const user = userEvent.setup();
    const onFilterRemove = vi.fn();
    render(
      <FilterChipBar
        filters={[{ id: "metadata", label: "Metadata", type: "text" }]}
        values={{ metadata: "lens" }}
        onFilterChange={vi.fn()}
        onFilterRemove={onFilterRemove}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByText("lens").closest("button")!);
    await user.click(screen.getByRole("button", { name: "Remove Metadata" }));
    expect(onFilterRemove).toHaveBeenCalledWith("metadata");
  });

  it("renders status chip popover without predefined options", async () => {
    const user = userEvent.setup();
    render(
      <FilterChipBar
        filters={[{ id: "sync", label: "Sync", type: "status" }]}
        values={{ sync: "ok" }}
        onFilterChange={vi.fn()}
        onFilterRemove={vi.fn()}
        onClearAll={vi.fn()}
      />,
    );
    await user.click(screen.getByText("ok").closest("button")!);
    expect(document.body.querySelector("ul")).toBeTruthy();
  });
});
