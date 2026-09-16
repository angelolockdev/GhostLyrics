<script lang="ts">
  import Overlay from "./lib/components/Overlay.svelte";
  import Settings from "./lib/components/Settings.svelte";

  let currentRoute = $state<string>(window.location.hash || "");

  window.addEventListener("hashchange", () => {
    currentRoute = window.location.hash;
  });
</script>

{#if currentRoute === "#/settings"}
  <Settings />
{:else}
  <Overlay />
{/if}
