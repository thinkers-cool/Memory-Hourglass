import { describe, expect, it } from "vitest";
import { exportProgressLabel, isJobFinished, toExportProgress } from "./jobProgress";

describe("jobProgress", () => {
  it("detects finished phases", () => {
    expect(isJobFinished("completed")).toBe(true);
    expect(isJobFinished("failed")).toBe(true);
    expect(isJobFinished("started")).toBe(false);
  });

  it("maps job progress", () => {
    expect(
      toExportProgress({
        job_id: "1",
        done: 2,
        total: 5,
        phase: "running",
        message: "exporting",
        file_name: "a.jpg",
      }),
    ).toEqual({
      done: 2,
      total: 5,
      phase: "running",
      message: "exporting",
      file_name: "a.jpg",
    });
  });

  it("formats progress labels", () => {
    expect(
      exportProgressLabel({
        done: 1,
        total: 2,
        phase: "completed",
        message: "done",
      }),
    ).toBe("done");
    expect(
      exportProgressLabel({
        done: 1,
        total: 2,
        phase: "running",
        message: "copying",
        file_name: "a.jpg",
      }),
    ).toBe("copying: a.jpg");
    expect(
      exportProgressLabel({
        done: 0,
        total: 2,
        phase: "running",
        message: "starting",
      }),
    ).toBe("starting");
  });
});
