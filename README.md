# Plyet

A personal budget app built with Tauri 2, SvelteKit, and SQLite. Track income
and expenses, recurring payments (subscriptions, salary, rent), and financial
products (deposits, loans, mortgages) — with a glassy, theme-able UI.

> The interface is in Russian.

<p align="center">
  <img src="design/plyet-feed-light.png" width="24%" alt="Feed — light" />
  <img src="design/plyet-feed-dark.png" width="24%" alt="Feed — dark" />
  <img src="design/plyet-reg-light.png" width="24%" alt="Recurring — light" />
  <img src="design/plyet-reg-dark.png" width="24%" alt="Recurring — dark" />
</p>

## Features

- **Budget hierarchy** — one or more root budgets, each with auto-created
  monthly sub-budgets (e.g. "Июль 2026") plus optional themed custom
  sub-budgets ("Отпуск"); the app opens on the current month by default and
  a "Весь бюджет" view aggregates everything under a root.
- **Feed** — income/expense items with categories, a donut balance chart, and
  search/filter.
- **Recurring** — subscriptions, salary, and rent that recur on a schedule.
- **Financial products** — deposits, loans, and mortgages with interest,
  annuity, and early-repayment modeling.
- **Theming** — light/dark themes, accent colors, glass blur and transparency
  toggles.
- **Local-first** — all data lives in a local SQLite database; no account, no
  cloud yet. The schema already carries stable UUIDs, timestamps, soft
  deletes and a members/roles table as groundwork for a future cloud sync
  and multi-user budgets.

## Tech stack

- **Frontend:** SvelteKit (Svelte 5 runes) + `@sveltejs/adapter-static`
- **Shell:** Tauri 2 (Rust)
- **Storage:** SQLite (`rusqlite`, bundled)
- **UI kit:** `reglass-material` (local workspace package under `packages/`)
- **Tooling:** pnpm workspaces, TypeScript, Vite

## Project layout

```
app/                  SvelteKit app + Tauri shell
  src/                Svelte UI (routes, components, stores)
  src-tauri/          Rust backend, Tauri config, mobile gen project
packages/
  reglass-material/   Shared UI component kit
design/               Design references and screenshots
flake.nix             Nix dev shell (Rust, Node, pnpm, Android SDK)
```

## Getting started

Requires [pnpm](https://pnpm.io) and the Rust toolchain. A Nix dev shell is
provided (`nix develop` / direnv) that pins Rust, Node, pnpm, the JDK, and the
Android SDK.

```sh
pnpm install

# Run the desktop app (Tauri dev)
pnpm --filter Plyet tauri dev

# Or run just the web frontend
pnpm dev:app
```

## Building

```sh
# Desktop bundle
pnpm --filter Plyet tauri build

# Android
pnpm --filter Plyet tauri android build
```

## Scripts

| Command           | Description                          |
| ----------------- | ------------------------------------ |
| `pnpm dev:app`    | Run the SvelteKit frontend           |
| `pnpm build:app`  | Build the frontend                   |
| `pnpm build:kit`  | Build the `reglass-material` kit     |
| `pnpm check`      | Type-check all workspace packages    |

## CI & releases

- **`.github/workflows/ci.yml`** runs on every push/PR: frontend type-check
  (`pnpm check`) plus `cargo fmt --check`, `cargo check` and `cargo test` for
  the Rust backend, all inside the pinned `nix develop` shell.
- **`.github/workflows/release.yml`** builds the Android release APK and
  attaches it to the GitHub Release when a release is published (or on a
  manual `workflow_dispatch`).

Signing is optional. Without a keystore configured, the workflow still
produces a working APK — just unsigned, and the artifact/release file is
named with an `-unsigned` suffix so that's obvious. To get signed releases:

1. Generate a release keystore (once, keep it safe — losing it means you
   can never update the app under the same signature again):
   ```sh
   keytool -genkeypair -v -keystore android-release-key.keystore \
     -alias plyet -keyalg RSA -keysize 2048 -validity 10000
   ```
2. In the repo's **Settings → Secrets and variables → Actions**, add:
   - `RELEASE_KEYSTORE_BASE64` — `base64 -w0 android-release-key.keystore`
   - `RELEASE_STORE_PASSWORD`, `RELEASE_KEY_ALIAS`, `RELEASE_KEY_PASSWORD`
3. Re-run the release workflow (or publish a new release) — the APK will now
   be signed.

For local signed builds, copy
`app/src-tauri/gen/android/keystore.properties.example` to
`keystore.properties` next to it and fill in your keystore details.

## License

[MIT](LICENSE) © keepinfov
