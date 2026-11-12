import { render } from "svelte/server";
import { beforeEach, describe, expect, it } from "vitest";
import { galleryCatalog } from "../../pages/gallery/catalog";
import { sessions as fixtures, limit, weekly } from "../../pages/gallery/fixtures";
import { initI18n } from "../i18n/i18n.svelte";
import type { SessionShareView } from "../ipc/bindings";
import LimitBar from "./LimitBar.svelte";
import RingGauge from "./RingGauge.svelte";
import SessionRow from "./SessionRow.svelte";
import SourceBadge from "./SourceBadge.svelte";
import Toggle from "./Toggle.svelte";

const [, untitledWithTitle, unnamed] = fixtures as [unknown, SessionShareView, SessionShareView];

beforeEach(() => initI18n(galleryCatalog, "tr"));

const ring = (props: Record<string, unknown>) => render(RingGauge, { props: props as never }).body;

describe("RingGauge", () => {
  it("reads both rings as one sentence, outer first", () => {
    const body = ring({ inner: limit(72), outer: weekly(34) });
    expect(body).toMatch(
      /aria-label="Haftalık limit %34 dolu, .*5 saatlik pencere %72 dolu, 1 saat 48 dakika sonra yenilenecek/,
    );
  });

  it("draws one weekly ring and centers its value without a 5-hour window", () => {
    const body = ring({ inner: limit(72), outer: weekly(34), single: true });
    expect(body).toContain("%34");
    expect(body).not.toContain("%72");
    expect(body).not.toContain("5 saatlik");
    expect(body.match(/class="track[ "]/g)).toHaveLength(1);
    expect(body).toContain('stroke-width="20"');
  });

  it("marks estimates with ≈ and a dashed mask", () => {
    const body = ring({ inner: limit(35, { source: "estimate" }), outer: null });
    expect(body).toContain("≈%35");
    expect(body).toMatch(/mask="url\(#[^)]+-inner-dash\)"/);
    expect(body).toContain("tahmini");
  });

  it("notches a critical ring and keeps four dash entries", () => {
    const body = ring({ inner: limit(94, { severity: "crit" }), outer: null });
    expect(body).toContain('stroke-dasharray="91 1.5 1.5 100"');
    expect(body).toContain('stroke-dasharray="0 0 0 100"');
  });

  it("draws a ghost arc and a ▲ only when projected past 100", () => {
    expect(ring({ inner: limit(42, { projected: 61 }), outer: null })).not.toContain("over-glyph");
    const over = ring({ inner: limit(70, { projected: 130 }), outer: null });
    expect(over).toContain('stroke-dasharray="0 70 30 100"');
    expect(over).toContain("over-glyph");
  });

  it("says when there is no data", () => {
    const body = ring({ inner: null, outer: null });
    expect(body).toContain("5 saatlik pencere: henüz veri yok");
    expect(body).toContain("–");
  });
});

describe("LimitBar", () => {
  it("is a meter with a full sentence", () => {
    const body = render(LimitBar, {
      props: { limit: weekly(81, { kind: "weekly_scoped", model: "Fable", severity: "warn" }) },
    }).body;
    expect(body).toContain('role="meter"');
    expect(body).toContain('aria-valuenow="81"');
    expect(body).toContain("Fable · haftalık");
    expect(body).toContain('aria-label="Uyarı"');
  });
});

describe("SourceBadge", () => {
  it("names a second source only when it lifted the value", () => {
    const now = Date.now();
    const alone = render(SourceBadge, { props: { source: "statusline", observedAt: now } }).body;
    expect(alone).toContain("Canlı");
    expect(alone).not.toContain("Canlı ve API birlikte");
    const combined = render(SourceBadge, {
      props: { source: "statusline", observedAt: now, combinedWith: "api" },
    }).body;
    expect(combined).toContain("Az önce güncellendi. Canlı ve API birlikte");
  });
});

describe("SessionRow", () => {
  it("falls back to the title, the project, then a placeholder", () => {
    expect(render(SessionRow, { props: { session: untitledWithTitle } }).body).toContain(
      "Fix flaky watcher test",
    );
    const body = render(SessionRow, { props: { session: unnamed } }).body;
    expect(body).toContain("scratch");
    expect(body).not.toContain("bu pencere");
    const bare = { ...unnamed, project: null };
    expect(render(SessionRow, { props: { session: bare } }).body).toContain("Adsız oturum");
  });
});

describe("Toggle", () => {
  it("is a switch", () => {
    const body = render(Toggle, { props: { checked: true, label: "x" } }).body;
    expect(body).toContain('role="switch"');
    expect(body).toContain('aria-checked="true"');
  });
});
