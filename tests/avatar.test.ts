import { describe, expect, it } from "vitest";
import { poseFor } from "../src/avatar/Avatar";
import type { AvatarState } from "../src/api/types";

const avatar = (over: Partial<AvatarState> = {}): AvatarState => ({
  hunger: 70, energy: 80, happiness: 70, xp: 0, level: 1, coins: 100,
  streak: 0, best_streak: 0, last_active_date: null, started_on: "2026-09-16",
  updated_at: 0, ...over,
});

describe("poseFor", () => {
  it("celebrates immediately after a reward, whatever the meters say", () => {
    expect(poseFor(avatar({ hunger: 5, energy: 5, happiness: 5 }), 12, true)).toBe("celebrating");
  });
  it("prioritises hunger over tiredness and sadness", () => {
    expect(poseFor(avatar({ hunger: 10, energy: 10, happiness: 10 }), 12, false)).toBe("eating");
  });
  it("sleeps when drained or late at night", () => {
    expect(poseFor(avatar({ energy: 12 }), 12, false)).toBe("sleepy");
    expect(poseFor(avatar(), 2, false)).toBe("sleepy");
    expect(poseFor(avatar(), 23, false)).toBe("sleepy");
  });
  it("sulks when unhappy and is otherwise idle", () => {
    expect(poseFor(avatar({ happiness: 20 }), 12, false)).toBe("sad");
    expect(poseFor(avatar(), 12, false)).toBe("idle");
  });
});
