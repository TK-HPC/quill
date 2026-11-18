# Product

<!-- impeccable:product-schema 1 -->

## Platform

Native Linux desktop, specifically Arch Linux and Hyprland.

## Stack

Rust and GTK4 for the desktop dashboard. Existing Python recording/capsule and transcribe.cpp inference remain responsible for dictation. SQLite holds local history. No web server or browser runtime.

## Users

People who dictate into desktop applications and want to see how much typing they avoid. The current user also plays games, so GPU memory can be unavailable.

## Product Purpose

Turn speech into text locally, keep the Win+X workflow fast, and show meaningful statistics from real dictations.

## Positioning

A personal local dictation tool. No account, subscription, referrals, upgrade badges, or promotional content.

## Operating Context

Win+X starts/stops the existing recording capsule. The dashboard is a separate window; closing it must not stop dictation. The user wants each implementation phase launched for inspection.

## Capabilities and Constraints

Working CUDA inference with CPU retry, 16 kHz mono recording, clipboard/paste integration, custom vocabulary in keywords.txt. New history begins when logging is installed; earlier totals are unknown and must not be fabricated. Estimated typing time must state its baseline.

## Brand Commitments

Quill. Very pale cream instead of white; black text and authentic monochrome library icons. A black Q mark accompanies a bold serif wordmark. Wispr Flow's desktop interface is the explicit reference. Readable large text, generous space, square outer corners, and no titlebar strip. Code must use descriptive names, explicit boolean comparisons, and full readable branches.

## Evidence on Hand

User-provided desktop screenshot, README, working Python source, real local model and CLI. No historical transcript database exists at the start of this work.

## Product Principles

- Show real data and useful empty states.
- Reuse platform controls and the working inference engine.
- Every visible control must work.
- Keep code small and explicit.

## Accessibility & Inclusion

Native keyboard navigation, visible focus, named icon controls, text labels, high contrast, read-only transcripts with a Copy control, and resizable windows.
