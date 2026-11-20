<!--
  The main window's shell (Step 15): the navigation on the left, the page title and the active
  page. Pages load on demand (dynamic `import()`), so the first paint carries only the shell and
  the opening page.

  - Routing is the URL fragment (`router.ts`): links are plain `#route` anchors, back and forward
    work, and a CLI or panel link arrives as `main-route-requested`.
  - `Ctrl+1..5` switch pages, `Ctrl+,` opens the settings window.
  - The title bar follows `document.title` (`main_window.rs`). After a page change the focus moves
    to the page heading, so a screen reader announces the new page.
  - The window is shown once the first page has painted (`main_ready`).
-->
<script lang="ts">
import { type Component, onMount, tick } from "svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, events, type MainRoute } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import NavIcon from "./NavIcon.svelte";
import { hrefFor, navKey, parseHash, ROUTES, shortcutFor } from "./router";

/** `param`: `#sessions/42` → `"42"`; `query`: the text after `?` (a list's filters). */
type Page = Component<{ param: string | null; query: string }>;

const PAGES: Record<MainRoute, () => Promise<{ default: Page }>> = {
  overview: () => import("./routes/Overview.svelte"),
  windows: () => import("./routes/Windows.svelte"),
  sessions: () => import("./routes/Sessions.svelte"),
  projects: () => import("./routes/Projects.svelte"),
  patterns: () => import("./routes/Patterns.svelte"),
};

let where = $state(parseHash(location.hash));
let page = $state<{ route: MainRoute; component: Page } | null>(null);
let failed = $state(false);
let heading: HTMLHeadingElement;
let content: HTMLElement;
let shown = false;

const title = $derived(t(navKey(where.route)));

$effect(() => {
  document.title = t("main-title", { page: title });
});

// Load the page module, then paint; the first paint shows the window, later ones move focus.
$effect(() => {
  const route = where.route;
  let current = true;
  failed = false;
  PAGES[route]()
    .then(async (module) => {
      if (!current) return;
      page = { route, component: module.default };
      await tick();
      requestAnimationFrame(() => {
        if (!current) return;
        performance.mark("route-painted");
        if (performance.getEntriesByName("route-start").length > 0) {
          performance.measure(`route:${route}`, "route-start", "route-painted");
          performance.clearMarks("route-start");
        }
        if (!shown) {
          shown = true;
          if (inTauri()) void commands.mainReady();
        } else {
          heading.focus();
        }
      });
    })
    .catch(() => {
      if (current) failed = true;
    });
  return () => {
    current = false;
  };
});

function go(route: MainRoute) {
  const href = hrefFor(route);
  if (location.hash === href) return;
  performance.mark("route-start");
  location.hash = href;
}

function onhashchange() {
  const next = parseHash(location.hash);
  // A new page or item starts at the top; a filter change (only `query`) keeps the place.
  if (next.route !== where.route || next.param !== where.param) content.scrollTop = 0;
  where = next;
}

function onkeydown(e: KeyboardEvent) {
  const shortcut = shortcutFor(e);
  if (!shortcut) return;
  e.preventDefault();
  if (shortcut.kind === "settings") openSettings();
  else go(shortcut.route);
}

function openSettings() {
  if (inTauri()) void commands.openSettings("general");
}

onMount(() => {
  if (!inTauri()) return;
  const off = events.mainRouteRequested.listen((e) => go(e.payload));
  return () => void off.then((f) => f());
});
</script>

<svelte:window {onhashchange} {onkeydown} />

<div class="window">
  <nav aria-label={t("main-nav")}>
    <ul>
      {#each ROUTES as route, i (route)}
        <li>
          <a
            href={hrefFor(route)}
            aria-current={where.route === route ? "page" : undefined}
            aria-keyshortcuts="Control+{i + 1}"
            onclick={() => performance.mark("route-start")}
          >
            <NavIcon name={route} />
            <span>{t(navKey(route))}</span>
          </a>
        </li>
      {/each}
    </ul>
    <button type="button" class="settings" aria-keyshortcuts="Control+," onclick={openSettings}>
      <NavIcon name="settings" />
      <span>{t("main-nav-settings")}</span>
    </button>
  </nav>
  <main bind:this={content}>
    <h1 bind:this={heading} tabindex="-1">{title}</h1>
    {#if failed}
      <p class="error" role="alert">{t("main-load-failed")}</p>
    {:else if page}
      {@const Page = page.component}
      {#key page.route}
        <Page param={where.param} query={where.query} />
      {/key}
    {:else}
      <span class="visually-hidden">{t("main-loading")}</span>
    {/if}
  </main>
</div>

<style>
  .window {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100vh;
  }
  nav {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: var(--space-3) var(--space-2);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  a,
  .settings {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-height: 36px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    color: var(--text-1);
    text-decoration: none;
    font: var(--type-body);
    text-align: start;
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  a :global(svg),
  .settings :global(svg) {
    color: var(--text-2);
  }
  a:hover,
  .settings:hover {
    background: var(--surface-1);
  }
  a[aria-current="page"] {
    background: var(--surface-2);
    font-weight: 600;
  }
  a[aria-current="page"] :global(svg) {
    color: var(--accent);
  }
  /* The Windows 11 selection pill, as in the settings window. */
  a[aria-current="page"]::before {
    content: "";
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: var(--radius-pill);
    background: var(--accent);
  }
  main {
    overflow-y: auto;
    padding: var(--space-4) var(--space-5) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  h1 {
    margin: 0;
    font: var(--type-heading);
    color: var(--text-1);
  }
  h1:focus {
    outline: none;
  }
  h1:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: var(--radius-control);
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
  @media (forced-colors: active) {
    a[aria-current="page"]::before {
      background: Highlight;
    }
  }
</style>
