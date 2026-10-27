// Parity (architecture §12): every `.ftl` file exists in both languages, parses cleanly, and
// defines the same messages with the same variables. Walks the parsed patterns, so variables
// inside selectors, variants and function calls all count.

import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { FluentResource } from "@fluent/bundle";
import { describe, expect, it } from "vitest";
import { upper } from "./i18n.svelte";

const LOCALES = resolve(import.meta.dirname, "../../../../locales");

function files(locale: string): string[] {
  return readdirSync(resolve(LOCALES, locale))
    .filter((f) => f.endsWith(".ftl"))
    .sort();
}

/** Collects `$variable` names from any node of a parsed pattern. */
function collect(node: unknown, out: Set<string>): void {
  if (Array.isArray(node)) {
    for (const n of node) collect(n, out);
  } else if (node && typeof node === "object") {
    const n = node as { type?: string; name?: string };
    if (n.type === "var" && n.name) out.add(n.name);
    for (const value of Object.values(node)) collect(value, out);
  }
}

function messages(locale: string, file: string): Map<string, string[]> {
  const source = readFileSync(resolve(LOCALES, locale, file), "utf8");
  const resource = new FluentResource(source);
  const body = (resource as unknown as { body: { id: string }[] }).body;
  const out = new Map<string, string[]>();
  for (const message of body) {
    const vars = new Set<string>();
    collect(message, vars);
    out.set(message.id, [...vars].sort());
  }
  // The parser skips broken entries silently; every `id =` line must have produced a message.
  const ids = source.match(/^[a-z][a-z0-9-]*(?=\s*=)/gm) ?? [];
  expect([...out.keys()].sort(), `${locale}/${file} has entries that do not parse`).toEqual(
    [...new Set(ids)].sort(),
  );
  return out;
}

describe("locale parity", () => {
  it("both languages have the same files", () => {
    expect(files("tr")).toEqual(files("en"));
  });

  for (const file of files("en")) {
    it(`${file}: same messages and variables in EN and TR`, () => {
      const en = messages("en", file);
      const tr = messages("tr", file);
      expect([...tr.keys()].sort()).toEqual([...en.keys()].sort());
      for (const [id, vars] of en) expect(tr.get(id), `${file}/${id}`).toEqual(vars);
    });
  }
});

describe("Turkish case mapping", () => {
  it("upper-cases i to İ in Turkish", async () => {
    const { setLocale } = await import("./i18n.svelte");
    setLocale("tr");
    expect(upper("istatistik")).toBe("İSTATİSTİK");
    setLocale("en");
    expect(upper("istatistik")).toBe("ISTATISTIK");
  });
});
