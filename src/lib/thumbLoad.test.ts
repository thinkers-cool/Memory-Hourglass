import { afterEach, describe, expect, it } from "vitest";
import {
  acquireThumbLoadSlot,
  releaseThumbLoadSlot,
  resetThumbLoadQueueForTests,
} from "./thumbLoad";

describe("thumbLoad", () => {
  afterEach(() => {
    resetThumbLoadQueueForTests();
  });

  it("ignores release when no slots are active", () => {
    releaseThumbLoadSlot();
    expect(true).toBe(true);
  });

  it("grants slots up to the concurrency limit", async () => {
    const slots = await Promise.all(
      Array.from({ length: 6 }, () => acquireThumbLoadSlot()),
    );
    expect(slots).toHaveLength(6);

    let granted = false;
    void acquireThumbLoadSlot().then(() => {
      granted = true;
    });
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(granted).toBe(false);

    releaseThumbLoadSlot();
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(granted).toBe(true);
  });
});
