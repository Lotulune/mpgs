import { describe, expect, it } from "vitest";
import { RequestGeneration } from "../src/app/requestGeneration";

describe("RequestGeneration", () => {
  it("invalidates an in-flight response when the visible request is cleared", () => {
    const requests = new RequestGeneration();
    const inFlight = requests.next();

    requests.invalidate();

    expect(requests.isCurrent(inFlight)).toBe(false);
  });

  it("only accepts the latest request", () => {
    const requests = new RequestGeneration();
    const first = requests.next();
    const second = requests.next();

    expect(requests.isCurrent(first)).toBe(false);
    expect(requests.isCurrent(second)).toBe(true);
  });
});
