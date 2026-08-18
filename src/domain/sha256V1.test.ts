import { describe, expect, it } from "vitest";
import { sha256HexBytesV1, sha256HexUtf8V1 } from "./sha256V1";

describe("synchronous SHA-256 v1", () => {
  it("matches published UTF-8 and raw-byte vectors", () => {
    expect(sha256HexUtf8V1("")).toBe("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    expect(sha256HexUtf8V1("abc")).toBe("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    expect(sha256HexBytesV1(Uint8Array.from([0, 255, 1]))).toBe("47ffa3ea45a70b8a41c2c0825df323c00a8b7a01c1ea06083cc41dddcc001123");
  });
});
