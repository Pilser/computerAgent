# frontend-egui — shell + slot architecture

Second frontend for computerAgent (Slint stays until egui reaches screenshot
parity). Same engine, same `command-api`/`Engine` seams, same warm theme.
This doc is the build contract for the egui phases; `docs/ARCHITECTURE.md`
§7 stays authoritative for the workspace.

## 0. Why egui, why a shell

Slint's retained layout negotiates space between siblings: one card's height
bug infected every row (see screenshots Oct-14). egui is immediate-mode, so
we invert control: the **shell pre-allocates every rect before any component
draws**. A component physically cannot move its siblings — there is no shared
layout negotiation to corrupt.

## 1. Folder structure

```text
crates/frontend-egui/
  Cargo.toml            # eframe, egui_commonmark, egui_extras(svg), egui_kittest
                        # (dev) + tokio/anyhow + engine/command-api path deps
  ARCHITECTURE.md       # this file
  src/
    lib.rs              # re-export only
    app.rs              # ComputerAgentApp: eframe::App; update() = shell.run()
    state/              # ALL ui state, one place, tested without egui boot
      mod.rs            # ChatState facade
      chat.rs           # Vec<Message>, draft, sending
      threads.rs        # thread list, active id, search filter
      models.rs         # catalog snapshot, current model
      usage.rs          # per-thread + total tokens/cost rollups
      status.rs         # status-ok + text, banner queue
    stream/
      mod.rs
      pump.rs           # worker thread: tokio runtime → engine.dispatch →
                        # mpsc Event{Chunk,Done,Err}; UI drains per frame,
                        # request_repaint() ONLY while streaming
    shell/              # layout ownership — components never lay out siblings
      mod.rs            # Shell::run(): regions → slots → registry → overlays
      slot.rs           # Slot policies: Fixed(px) | Flex(weight) |
                        # ContentCapped(max) — computed ONCE per frame
      registry.rs       # Component trait + inventory (one register() line)
      layout.rs         # region map + density scale (the only resize logic)
      overlay.rs        # overlay stack: Order + modality + input routing (§3)
    components/         # one folder per plug-in, zero sibling knowledge
      sidebar/component.rs
      feed/component.rs
      composer/component.rs
      topbar/component.rs
      banner/component.rs
      overlays/
        approval/component.rs  # Ask-approval dialog (modal overlay)
        toast/component.rs     # transient notices (non-modal overlay)
        palette/component.rs   # Ctrl+K command palette (modal overlay)
        menu/component.rs      # message context menu (anchored overlay)
    markdown/           # reused concepts from frontend-slint, egui-native
      mod.rs
      blocks.rs         # fence splitter → Block model (moved, not rewritten)
      render.rs         # Block → egui_commonmark (prose) + custom code /
                        # mermaid-SVG-image / quote / table renderers
      diagram.rs        # merman SVG bytes + hash cache (moved, not rewritten)
    theme.rs            # warm Visuals (paper/terracotta/amber), type scale,
                        # spacing scale — the ONLY styling entry point
  tests/
    state_tests.rs      # append/filter/usage math, no egui boot
    snapshot_tests.rs   # egui_kittest: one PNG per component + full shell
crates/desktop-egui/
  Cargo.toml + src/main.rs   # eframe::run_native("computerAgent", …) <60 lines
```

Anti-bloat (inherits root AGENTS.md): 500 hard / 300 target per file,
`lib.rs`/`mod.rs` re-export only, new block kind = new file under
`views/`-equivalent, never growth. Views take narrow state slices.

## 2. Shell contract

```rust
pub trait Component {
    fn id(&self) -> &'static str;   // "feed", "composer", …
    fn slot(&self) -> SlotId;       // where the shell puts it
    fn layer(&self) -> Layer;       // Base | Overlay (see §3)
    fn show(&mut self, ui: &mut Ui, slot: Allocated);
}
```

`Shell::run()` per frame: `layout()` → allocate base slots → iterate
registry in order → allocate + drain overlay stack. `Allocated` is a child
`Ui` clipped to the slot rect with theme overrides applied. Components
return nothing layout-relevant.

Slot policies: sidebar `Fixed(300, collapsible <900px)`, feed `Flex(1)`
(all leftover), composer `Fixed(132)`, topbar `Fixed(48)`, banner
`ContentCapped(96)`. Widths are fixed BEFORE painting, so wrapped text
measures against a known number — the Slint overlap class (measured
unbounded, painted bounded) is structurally impossible.

## 3. Overlays as plug-ins: the multi-layer model

The window is a layer stack, painted bottom-up every frame:

```text
┌────────────────────────────────────────────┐
│ OVERLAY layer (absolute, window space)     │  dialogs · toasts · menus
│  · paints LAST, positioned by anchor       │  palette · tooltips
├────────────────────────────────────────────┤
│ BASE layer (shell slots, partitioned)      │  topbar / sidebar / feed /
│  · rects fixed before any drawing          │  composer / banner
├────────────────────────────────────────────┤
│ BACKGROUND (theme wash, zero logic)        │
└────────────────────────────────────────────┘
```

An overlay is the SAME `Component` trait plus two answers:

```rust
fn layer(&self) -> Layer { Layer::Overlay(OverlayKind::Modal) }
fn anchor(&self) -> Anchor;  // Center | Cursor | Widget(id) | Dock(edge)
```

Kinds: `Modal` (dims below, consumes ALL input — approval dialog,
palette), `Toast` (timed, pass-through input — "copied", send errors),
`Menu`/`Tooltip` (anchored to cursor or widget rect, dismiss on
click-elsewhere/hover-out). Rules the shell enforces, so plug-ins stay
safe: overlays never resize base slots; exactly ONE modal at a time
(second queues); anchors clamp to the window rect; overlay input routing
(modal consumes, toast/menu pass-through) lives in `shell/overlay.rs`,
never in components. Mid-frame theme/density changes flow through the
same slot scope as base components.

Mapped to our app: Ask-approval → modal overlay plug-in (replaces the
missing Slint dialog milestone); run errors → banner (base) + toast
(overlay) from the same status queue; message right-click → menu overlay;
token bar hover → tooltip overlay; Ctrl+K → palette overlay over dimmed
base. Each is one folder under `components/overlays/` + one register()
line — adding the emoji picker later touches nothing existing.

## 4. Data flow (streaming per the researched pattern)

```text
worker thread (pump.rs, tokio) → mpsc Event{Chunk,Done,Err}
  → app.update() drains → state.chat append → ctx.request_repaint()
  → feed ScrollArea::show_rows renders visible rows only (virtualized)
  → stick-to-bottom unless user scrolled up
```

Repaint is reactive (input-driven) EXCEPT while `sending`, when the pump's
events force it. Idle app costs ~0 CPU. Copy buttons on code blocks and
message hover (egui text selection stays secondary by design).

## 5. Proof loop

`release.yml` gains eframe link deps (`libxcb-render0-dev`,
`libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libxkbcommon-dev`, `libssl-dev`)
and a `computer-agent-egui` artifact. `egui_kittest` snapshot tests render
one PNG per component (seeded slot rect, no app boot) plus one full-shell
PNG — a broken card fails exactly one test. Slint binary keeps shipping
until egui reaches screenshot parity.

## 6. Phases

1. Scaffold: Cargo/manifests, theme, state, shell+registry+overlay,
   kittest harness → CI green (compile proof).
2. Feed/message/composer + markdown bridge → snapshot proof.
3. Sidebar/topbar/banner + desktop-egui binary → artifact + live test.
4. Mermaid images inline + toast/menu overlays → parity, retire Slint feed.
5. Warm styling: full Visuals + type/spacing scale, bubbles, chips,
   dark toggle, xvfb screenshot proof in `preview.yml`.

## 7. Styling (egui5): Visuals ownership, bubble pattern, screenshot proof

`src/theme.rs` is the ONLY styling entry point and it owns the whole
look: `apply(ctx, dark)` runs at the start of EVERY `Shell::run` frame
(never once-at-boot, so no widget or panel can keep stale stock
visuals), setting full `Visuals` — paper `#FAF5EC` window wash,
`#FFFDF8` panels, terracotta `#C4622D` primary with amber `#B98A2F`
hovers/selection, dark ink text, 8px button / 10px frame rounding, no
shadows — plus the type scale (body 15, button 14, small 12.5, heading
20) and `(8, 6)` item spacing that the stock theme never touched.
Components read ambient visuals and the tiny helpers (`bubble`,
`soft_text`, `hairline`, `chip`, `danger`, `toast`); they hard-code no
color. Dark mode is a single `dark_mode: bool` on `ChatState` (topbar
☾/☀ chip, `CA_EGUI_DARK=1` boot override) flipping a full ember
palette (`#211A13` bg, lit terracotta/amber accents) — there is no
half-themed state.

Bubbles are `Frame`s, not labels: user messages align right in
terracotta with paper text, assistant messages align left on the
`#F1E7D3` surface wash, both capped at ~72% of the feed width via
`available_width` (so wrap measures against the real column), with a
muted `▲ N tok` footnote and copy/… bordered chips in the header row.
The composer is a surfaced `TextEdit` with its own `Frame` (1px
hairline, radius 10, 3 rows) above a filled terracotta Send that swaps
to a danger Stop while streaming; sidebar rows are full-width tinted
cards with a left brand bar on the active thread, letterspaced muted
section labels, and hairline separators. Proof is visual, not verbal:
`preview.yml`'s `egui-shots` job builds `computer-agent-egui`, boots it
under `xvfb` three ways (fresh `CA_DATA_DIR`, seeded fixture thread
from `tests/fixtures/seeded`, seeded + dark), captures each window with
`scrot`, and uploads the PNGs — falling back to `egui_kittest`
`UPDATE_SNAPSHOTS` PNGs only if the xvfb capture comes back empty.
Lesson from v0.4.4: the app panicked at boot on the runner
(`libxkbcommon-x11.so could not be loaded`), so `scrot` captured three
identical 3KB all-black PNGs; the job now installs
`libxkbcommon-x11-0` (preview-only apt) and rejects any capture under
20KB before accepting the xvfb path.
