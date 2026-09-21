<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Overlay from "./lib/components/Overlay.svelte";
  import Settings from "./lib/components/Settings.svelte";

  let isSettings = $state<boolean>(false);

  // 1. Détection prioritaire par le label natif de la fenêtre Tauri (ultra-fiable)
  try {
    const win = getCurrentWindow();
    if (win && win.label === "settings") {
      isSettings = true;
    }
  } catch (e) {
    // Si exécuté hors contexte Tauri (navigateur standard)
    if (typeof window !== "undefined" && window.location.hash.includes("settings")) {
      isSettings = true;
    }
  }

  // 2. Détection par l'URL hash (fallback)
  if (typeof window !== "undefined" && window.location.hash.includes("settings")) {
    isSettings = true;
  }

  if (typeof window !== "undefined") {
    window.addEventListener("hashchange", () => {
      if (window.location.hash.includes("settings")) {
        isSettings = true;
      } else {
        try {
          const win = getCurrentWindow();
          isSettings = win ? win.label === "settings" : false;
        } catch (e) {
          isSettings = false;
        }
      }
    });
  }
</script>

{#if isSettings}
  <Settings />
{:else}
  <Overlay />
{/if}
