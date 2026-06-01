<script lang="ts">
  import { untrack, onDestroy } from 'svelte';
  import { formatMoney } from '$lib/utils';

  interface Props {
    value: number;
    class?: string;
  }

  let { value, class: className = '' }: Props = $props();

  const formatted = $derived(formatMoney(value));

  let displayedText = $state('');
  let animating = $state(false);
  let timer: ReturnType<typeof setInterval> | null = null;
  let isFirstRender = true;

  function clearTimer() {
    if (timer) { clearInterval(timer); timer = null; }
  }

  $effect(() => {
    const target = formatted;
    const current = untrack(() => displayedText);

    if (isFirstRender) {
      isFirstRender = false;
      displayedText = target;
      return;
    }

    if (target === current) return;

    clearTimer();
    animating = true;

    // Find common prefix — only erase/retype the differing suffix
    let common = 0;
    while (common < current.length && common < target.length && current[common] === target[common]) {
      common++;
    }

    const TICK = 30;
    let text = current;
    let phase: 'erase' | 'type' = current.length > common ? 'erase' : 'type';
    let typeIdx = common;

    timer = setInterval(() => {
      if (phase === 'erase') {
        text = text.slice(0, -1);
        displayedText = text;
        if (text.length <= common) {
          phase = 'type';
          typeIdx = common;
        }
      } else {
        if (typeIdx < target.length) {
          typeIdx++;
          displayedText = target.slice(0, typeIdx);
        } else {
          clearTimer();
          animating = false;
        }
      }
    }, TICK);
  });

  onDestroy(clearTimer);
</script>

<span class="retype-num {className}">{displayedText}<span class="cursor" class:visible={animating}></span></span>

<style>
  .retype-num {
    display: inline;
  }

  .cursor {
    display: inline-block;
    width: 1.5px;
    height: 0.85em;
    background: var(--rg-primary);
    vertical-align: baseline;
    margin-left: 1px;
    opacity: 0;
  }

  .cursor.visible {
    opacity: 1;
    animation: blink 0.53s step-end infinite;
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }
</style>
