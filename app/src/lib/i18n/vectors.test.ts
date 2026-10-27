// The shared formatter vectors (architecture §12): the same file runs in
// crates/tokenradar-i18n/tests/vectors.rs, so both sides print identical strings.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { createBundle, format } from "./fluent";
import { clock, dur, type Locale, num, parseDurStyle, pct, weekdayClock } from "./format";
import { parseCase, trSuffix } from "./suffix";

type Vector = Record<string, string | number | boolean>;
const vectors = JSON.parse(
  readFileSync(resolve(import.meta.dirname, "../../../../testdata/i18n-vectors.json"), "utf8"),
) as { schema: number } & Record<string, Vector[]>;

const list = (key: string): Vector[] => vectors[key] ?? [];
const loc = (v: Vector) => v.locale as Locale;
const n = (v: Vector, key: string) => Number(v[key]);

describe("shared i18n vectors", () => {
  it("has a known schema", () => {
    expect(vectors.schema).toBe(1);
  });

  it("NUM", () => {
    for (const v of list("num"))
      expect(num(loc(v), n(v, "n"), n(v, "decimals")), JSON.stringify(v)).toBe(v.out);
  });

  it("PCT", () => {
    for (const v of list("pct")) expect(pct(loc(v), n(v, "n")), JSON.stringify(v)).toBe(v.out);
  });

  it("DUR", () => {
    for (const v of list("dur")) {
      expect(dur(loc(v), n(v, "ms"), parseDurStyle(v.style)), JSON.stringify(v)).toBe(v.out);
    }
  });

  it("clock", () => {
    for (const v of list("clock")) {
      const got = weekdayClock(
        loc(v),
        n(v, "weekday"),
        n(v, "hour"),
        n(v, "minute"),
        v.hour12 === true,
      );
      expect(got, JSON.stringify(v)).toBe(v.out);
    }
    expect(clock("tr", 9, 5, true)).toBe("09:05");
  });

  it("TR_SUFFIX", () => {
    for (const v of list("tr_suffix")) {
      const text = typeof v.input === "number" ? num("tr", v.input, 0) : String(v.input);
      const c = parseCase(v.case);
      expect(c, JSON.stringify(v)).not.toBeNull();
      if (c) expect(trSuffix(text, c), JSON.stringify(v)).toBe(v.out);
    }
  });
});

describe("Fluent functions", () => {
  const src = [
    "pct = { PCT($n) }",
    'dur = { DUR($ms, "compact") }',
    "num = { NUM($n, 1) }",
    'sfx = { TR_SUFFIX($at, "loc") } yenilenir',
    'sfx-num = { TR_SUFFIX(1000000, "abl") }',
    'sfx-dec = { TR_SUFFIX(23.5, "loc") }',
    'sfx-bad = { TR_SUFFIX(5, "nope") }',
  ].join("\n");
  const tr = createBundle("tr", [src]);
  const en = createBundle("en", [src]);

  it("reach the messages like the Rust bundle", () => {
    expect(format(tr, "pct", { n: 72 })).toBe("%72");
    expect(format(en, "pct", { n: 23.5 })).toBe("23.5%");
    expect(format(tr, "dur", { ms: 6480000 })).toBe("1sa48dk");
    expect(format(tr, "num", { n: 1234.5 })).toBe("1.234,5");
    expect(format(tr, "sfx", { at: "Çar 10:59" })).toBe("Çar 10:59'da yenilenir");
    expect(format(en, "sfx", { at: "Wed 10:59" })).toBe("Wed 10:59 yenilenir");
    expect(format(tr, "sfx-num")).toBe("1.000.000'dan");
    expect(format(tr, "sfx-dec")).toBe("23,5'te");
    expect(format(tr, "sfx-bad")).toBe("5");
  });

  it("show the id of a missing message", () => {
    expect(format(tr, "no-such-key")).toBe("no-such-key");
  });
});
