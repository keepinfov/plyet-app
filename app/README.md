# Plyet — app

The SvelteKit frontend and Tauri shell for Plyet. See the
[root README](../README.md) for the project overview, setup, and build
instructions.

## Quick reference

```sh
pnpm dev          # vite dev server
pnpm tauri dev    # run inside the Tauri shell
pnpm tauri build  # desktop bundle
pnpm check        # type-check
```

- `src/` — Svelte UI (routes, components, stores)
- `src-tauri/` — Rust backend, Tauri config, mobile gen project
