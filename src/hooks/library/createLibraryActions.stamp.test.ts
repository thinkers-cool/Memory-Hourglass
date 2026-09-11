import { describe, expect, it, vi, beforeEach } from "vitest";
import { createLibraryActions } from "./createLibraryActions";
import { makeLibraryActionsDeps } from "../../test/createLibraryActionsDeps";

vi.mock("../../api/client", () => ({
  deleteTag: vi.fn(),
  deleteAlbum: vi.fn(),
}));

describe("createLibraryActions stamp", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("blocks tag delete when referenced by stamp", () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps as never);
    actions.deleteTag(7);
    expect(deps.requestConfirm).not.toHaveBeenCalled();
    expect(deps.setNotification).toHaveBeenCalled();
  });

  it("blocks album delete when referenced by stamp", () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps as never);
    actions.deleteAlbum(8);
    expect(deps.requestConfirm).not.toHaveBeenCalled();
    expect(deps.setNotification).toHaveBeenCalled();
  });

  it("delegates disarm to stamp hook", () => {
    const deps = makeLibraryActionsDeps();
    const actions = createLibraryActions(deps as never);
    actions.disarmStamp();
    expect(deps.disarmStamp).toHaveBeenCalledTimes(1);
  });
});
