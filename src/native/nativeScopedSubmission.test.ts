import { describe, expect, it, vi } from "vitest";
import { runNativeScopedSubmission } from "./nativeScopedSubmission";

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe("native scoped async submission", () => {
  it("keeps Recode busy through completion and rejects a superseded dialog scope", async () => {
    const currentOperation = deferred<void>();
    const currentBusy: boolean[] = [];
    const currentComplete = vi.fn();
    const current = runNativeScopedSubmission({
      perform: () => currentOperation.promise,
      isCurrent: () => true,
      setBusy: (busy) => currentBusy.push(busy),
      complete: currentComplete,
      fail: vi.fn(),
    });
    expect(currentBusy).toEqual([true]);
    expect(currentComplete).not.toHaveBeenCalled();
    currentOperation.resolve();
    await current;
    expect(currentBusy).toEqual([true, false]);
    expect(currentComplete).toHaveBeenCalledOnce();

    const staleOperation = deferred<void>();
    const staleBusy: boolean[] = [];
    const staleComplete = vi.fn();
    let originatingDialogStillCurrent = true;
    const stale = runNativeScopedSubmission({
      perform: () => staleOperation.promise,
      isCurrent: () => originatingDialogStillCurrent,
      setBusy: (busy) => staleBusy.push(busy),
      complete: staleComplete,
      fail: vi.fn(),
    });
    originatingDialogStillCurrent = false;
    staleOperation.resolve();
    await stale;
    expect(staleBusy).toEqual([true]);
    expect(staleComplete).not.toHaveBeenCalled();
  });

  it("holds the busy guard until a deferred mutation completes", async () => {
    const operation = deferred<void>();
    const setBusy = vi.fn();
    const complete = vi.fn();
    const pending = runNativeScopedSubmission({
      perform: () => operation.promise,
      isCurrent: () => true,
      setBusy,
      complete,
      fail: vi.fn(),
    });

    expect(setBusy).toHaveBeenCalledTimes(1);
    expect(setBusy).toHaveBeenLastCalledWith(true);
    expect(complete).not.toHaveBeenCalled();

    operation.resolve();
    await pending;

    expect(setBusy.mock.calls).toEqual([[true], [false]]);
    expect(complete).toHaveBeenCalledOnce();
  });

  it("ignores a stale completion after its dialog scope is superseded", async () => {
    const operation = deferred<void>();
    const setBusy = vi.fn();
    const complete = vi.fn();
    let current = true;
    const pending = runNativeScopedSubmission({
      perform: () => operation.promise,
      isCurrent: () => current,
      setBusy,
      complete,
      fail: vi.fn(),
    });

    current = false;
    operation.resolve();
    await pending;

    expect(setBusy.mock.calls).toEqual([[true]]);
    expect(complete).not.toHaveBeenCalled();
  });

  it("reports a current deferred failure without completing", async () => {
    const operation = deferred<void>();
    const fail = vi.fn();
    const complete = vi.fn();
    const setBusy = vi.fn();
    const pending = runNativeScopedSubmission({
      perform: () => operation.promise,
      isCurrent: () => true,
      setBusy,
      complete,
      fail,
    });

    const reason = new Error("write failed");
    operation.reject(reason);
    await pending;

    expect(setBusy.mock.calls).toEqual([[true], [false]]);
    expect(fail).toHaveBeenCalledWith(reason);
    expect(complete).not.toHaveBeenCalled();
  });
});
