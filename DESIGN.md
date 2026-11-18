# Design

## Source of truth

Active, 2026-11-19. Quill native desktop dashboard. Evidence: user screenshot, PRODUCT.md, existing capsule, [Wispr Flow navigation](https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android), [usage statistics](https://docs.wisprflow.ai/articles/8760230576-your-usage-tab-track-your-dictation-stats-in-wispr-flow), [GTK4 Rust guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/).

## Brand

Quiet, precise, personal. Cream and black, editorial typography, genuine library icons. A black Q mark accompanies the Quill wordmark. No commercial surfaces, gradients, colored badges, or decorative graphs.

## Product goals

Expose words dictated, estimated typing time avoided, dictation count, and real history. Preserve Win+X. Provide working history search/copy and vocabulary editing. Launch each working phase for review.

## Personas and jobs

A Linux user moving between coding, writing, messaging, and games. Check progress, recover a transcript, add an unfamiliar term, and return to work.

## Information architecture

Home: shortcut guidance, real totals, recent dictations. History: search and copy stored transcripts. Dictionary: edit the vocabulary already consumed by the model. Settings: typing-speed estimate and local setup details. These four working destinations are the current scope.

## Design principles

Operate mode. Reference-led sidebar and history rhythm; Quill-specific serif headline and subdued cream surfaces. Open space separates sections; borders separate records. No card grid around every element. All foreground text and symbols remain black.

## Visual language

Main #faf9f5, sidebar #f2f0e9, instruction surface #eeebe1, ink #171713, border #e4e0d6. Bundled DM Sans for body text, Newsreader for editorial display, and DM Mono for statistic counters. Body 22 px, bold navigation 20 px, captions 18 px, page title 32 px, headline 50 px, statistics 56 px. Wordmark 40 px. Spacing 8/12/20/28/36/52. Sidebar width 232 px with 20 px horizontal padding. Outer window corners square; internal panels use 18–22 px radii. No decorative shadows. Native GTK crossfade between pages (180 ms). Counters ease from zero to the real totals on each Home click (700 ms), then animate when new dictations arrive. On Home, 136 rainbow ribbons emit steadily from the lower left and right at roughly 45°. Emission lasts 0.56 seconds at 120 ribbons per second per side; the last ribbons fade by 1.4 seconds. Duration does not scale flight speed or launch rate. They render above the entire window, including the sidebar, and hide on other pages. This brief celebration is the intentional color accent in the cream-and-black interface. The overlay does not accept input. Respect GTK’s animation preference. Use the native frame clock; no animation dependency.

## Components

GTK4 boxes, labels, buttons, stack, scrolled windows, search entry, text view, spin button. Hugeicons Free Stroke Rounded, black at 18–24 px, with stronger strokes for the sidebar. Sources and licenses live beside the assets. Token ownership: desktop/style.css. Window decorations are disabled; the wordmark is a native drag handle and a named close button sits at the upper right.

## Accessibility

High text contrast, 40+ px control height, focus outlines, keyboard traversal, action tooltips. Text size never below 18 px. Long transcripts wrap. Rows show text and Copy only, with no text cursor, date, duration, word-count metadata, editing, or deletion. Icons accompany text or have a clear accessible name.

## Responsive behavior

Desktop window initially 1240×850. Minimum 960×680. Sidebar stays readable; main content expands and vertically scrolls. Test initial and minimum sizes. No mobile surface in scope.

## Interaction states

Honest zero-state until new dictations arrive. Errors name the problem and show retry/recovery. History refreshes while the dashboard is open. Save feedback for vocabulary. Search with no matches has its own state. Never silently display zero when storage could not be read.

## Content voice

Short, concrete, noncommercial. “Speak. Quill will type.”, “Words you didn’t type”, “Estimated time saved”, “Recent dictations”. Explain the typing baseline. Avoid invented speed or accuracy claims.

## Implementation constraints

Rust GTK4 frontend, standard SQLite via rusqlite, Python sqlite3 writer. No browser runtime. Prefer established widgets, explicit branches and boolean comparisons, and descriptive identifiers. No ternary expressions or compact conditional tricks. Keep the package/socket paths stable. Tests use temporary databases and recordings; production contains no demonstration rows.
