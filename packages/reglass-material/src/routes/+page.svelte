<script lang="ts">
  import {
    Surface,
    Button,
    IconButton,
    TogglePill,
    Chip,
    Input,
    ListRow,
    Card,
    Fab,
    Snackbar,
    SegmentedToggle,
    Sheet,
    ConfirmDialog,
    ColorPicker,
    IconPicker,
  } from '$lib';

  const tints = ['#4285F4', '#EA4335', '#34A853', '#7C4DFF', '#E91E63'];
  const presetColors = ['#4285F4', '#EA4335', '#FBBC05', '#34A853', '#7C4DFF', '#00ACC1'];
  const demoIcons = ['a', 'b', 'c', 'd'] as const;

  let tab = $state<'one' | 'two' | 'three'>('one');
  let text = $state('');
  let chipActive = $state(true);
  let sheetOpen = $state(false);
  let confirmOpen = $state(false);
  let snackOpen = $state(false);
  let pickColor = $state('#7C4DFF');
  let pickIcon = $state<(typeof demoIcons)[number]>('a');
</script>

{#snippet gear()}
  <svg width="1.25rem" height="1.25rem" viewBox="0 0 24 24" fill="currentColor"><path d="M12 8a4 4 0 100 8 4 4 0 000-8z"/></svg>
{/snippet}

{#snippet renderDemoIcon(key: string)}
  <svg width="1.25rem" height="1.25rem" viewBox="0 0 24 24" fill="currentColor"><text x="12" y="17" text-anchor="middle" font-size="14">{key}</text></svg>
{/snippet}

<h1>Gallery</h1>

<section>
  <h2>Surface</h2>
  <Surface level={1} style="padding:1rem;margin-bottom:0.5rem">L1 glass</Surface>
  <Surface level={2} style="padding:1rem;margin-bottom:0.5rem">L2 solid</Surface>
  <div class="row">
    {#each tints as t}<Surface level={3} tint={t} style="padding:1rem;flex:1">L3</Surface>{/each}
  </div>
</section>

<section>
  <h2>Button</h2>
  <div class="row wrap">
    <Button variant="filled">Filled</Button>
    <Button variant="text">Text</Button>
    <Button variant="tonal">Tonal</Button>
    <Button variant="danger">Danger</Button>
    <Button variant="surface">Surface</Button>
  </div>
</section>

<section>
  <h2>IconButton</h2>
  <div class="row">
    <IconButton aria-label="ghost">{@render gear()}</IconButton>
    <IconButton variant="surface" aria-label="surface">{@render gear()}</IconButton>
    <IconButton variant="tonal" aria-label="tonal">{@render gear()}</IconButton>
    <IconButton variant="tonal" shape="round" aria-label="round">{@render gear()}</IconButton>
  </div>
</section>

<section>
  <h2>TogglePill / Chip</h2>
  <div class="row wrap">
    <TogglePill active label="On" />
    <TogglePill label="Off" />
    <Chip active={chipActive} onclick={() => (chipActive = !chipActive)}>Toggle me</Chip>
    <Chip active tint="#E91E63">Tinted</Chip>
  </div>
</section>

<section>
  <h2>Input</h2>
  <Input bind:value={text} placeholder="Type here…" />
  <div style="height:0.5rem"></div>
  <Input variant="bare" placeholder="Bare input" />
</section>

<section>
  <h2>ListRow</h2>
  <ListRow variant="nav" title="Navigation row" description="with chevron" chevron leading={gear} />
  <div style="height:0.375rem"></div>
  <ListRow title="Setting row" description="compact" leading={gear}>
    {#snippet trailing()}<TogglePill active label="On" />{/snippet}
  </ListRow>
</section>

<section>
  <h2>Card</h2>
  <Card tint="#34A853" style="padding:1rem">Tinted card (L3)</Card>
</section>

<section>
  <h2>SegmentedToggle</h2>
  <SegmentedToggle
    items={[
      { key: 'one', label: 'Траты' },
      { key: 'two', label: 'Доходы' },
      { key: 'three', label: 'Все' },
    ]}
    bind:value={tab}
  />
  <p>Selected: {tab}</p>
</section>

<section>
  <h2>Overlays</h2>
  <div class="row wrap">
    <Button onclick={() => (sheetOpen = true)}>Open Sheet</Button>
    <Button variant="danger" onclick={() => (confirmOpen = true)}>Confirm</Button>
    <Button variant="tonal" onclick={() => { snackOpen = true; setTimeout(() => (snackOpen = false), 1500); }}>Snackbar</Button>
  </div>
</section>

<section>
  <h2>ColorPicker</h2>
  <Surface level={2} style="padding:0.75rem">
    <ColorPicker bind:value={pickColor} colors={presetColors} />
  </Surface>
  <p>Value: {pickColor}</p>
</section>

<section>
  <h2>IconPicker</h2>
  <IconPicker icons={[...demoIcons]} bind:value={pickIcon} renderIcon={renderDemoIcon} />
  <p>Selected: {pickIcon}</p>
</section>

<Sheet show={sheetOpen} onclose={() => (sheetOpen = false)} scrollable>
  <h2>Sheet</h2>
  <p>A glass bottom-dialog. Press Escape or tap the backdrop.</p>
  <Button onclick={() => (sheetOpen = false)}>Close</Button>
</Sheet>

<ConfirmDialog
  show={confirmOpen}
  message="Delete this item?"
  onconfirm={() => (confirmOpen = false)}
  oncancel={() => (confirmOpen = false)}
/>

<Snackbar show={snackOpen} message="Saved!" />

<Fab aria-label="Add">
  <svg width="1.75rem" height="1.75rem" viewBox="0 0 24 24" fill="currentColor"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
</Fab>

<style>
  section { margin-bottom: 2rem; }
  h2 { font-size: 1rem; opacity: 0.7; margin-bottom: 0.5rem; }
  .row { display: flex; gap: 0.5rem; align-items: center; }
  .row.wrap { flex-wrap: wrap; }
  p { opacity: 0.7; font-size: 0.875rem; margin-top: 0.5rem; }
</style>
