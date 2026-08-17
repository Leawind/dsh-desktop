import { describe, expect, it } from "vitest";

import { withDefaultTargetProtocol } from "./useWindowTarget";

describe("withDefaultTargetProtocol", () => {
  it("defaults host and port input to HTTP", () => {
    expect(withDefaultTargetProtocol(" 127.0.0.1:3080 ")).toBe("http://127.0.0.1:3080");
    expect(withDefaultTargetProtocol("localhost:3080")).toBe("http://localhost:3080");
    expect(withDefaultTargetProtocol("[::1]:3080")).toBe("http://[::1]:3080");
  });

  it("preserves an explicitly selected protocol", () => {
    expect(withDefaultTargetProtocol("https://example.com:3080")).toBe("https://example.com:3080");
  });
});
