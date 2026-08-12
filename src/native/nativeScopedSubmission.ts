export interface NativeScopedSubmissionOptions {
  perform: () => Promise<void>;
  isCurrent: () => boolean;
  setBusy: (busy: boolean) => void;
  complete: () => void;
  fail: (reason: unknown) => void;
}

/**
 * Runs an async dialog mutation without letting a disposed/superseded dialog
 * change the busy state or close the dialog that replaced it.
 */
export async function runNativeScopedSubmission({
  perform,
  isCurrent,
  setBusy,
  complete,
  fail,
}: NativeScopedSubmissionOptions): Promise<void> {
  setBusy(true);
  try {
    await perform();
    if (!isCurrent()) return;
    setBusy(false);
    complete();
  } catch (reason) {
    if (!isCurrent()) return;
    setBusy(false);
    fail(reason);
  }
}
