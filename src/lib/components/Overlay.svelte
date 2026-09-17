<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen } from "@tauri-apps/api/event";
  import type { LyricLine, SongMetadata, PlaybackState, AppSettings, CurrentMediaState } from "../types";

  // State using Svelte 5 Runes
  let currentSong = $state<SongMetadata>({
    title: "En attente de lecture...",
    artist: "Lancez une musique sur Spotify, Deezer ou YouTube",
    durationMs: 0,
    sourceApp: "",
  });

  let playback = $state<PlaybackState>({
    isPlaying: false,
    positionMs: 0,
    lastUpdatedMs: Date.now(),
    playbackRate: 1.0,
  });

  let lyrics = $state<LyricLine[]>([
    { startTimeMs: 0, text: "GhostLyrics — Prêt à afficher vos paroles" },
    { startTimeMs: 4000, text: "Lancez Spotify ou YouTube pour démarrer la synchronisation" },
    { startTimeMs: 8000, text: "Ou testez la démo instantanée dans les Paramètres (⚙️)" },
  ]);

  let settings = $state<AppSettings>({
    displayMode: "standard",
    fontSize: 22,
    opacity: 0.88,
    textColor: "rgba(255, 255, 255, 0.45)",
    activeColor: "#38bdf8",
    backgroundColor: "rgba(15, 15, 20, 0.82)",
    timeOffsetMs: 0,
    hotkey: "Ctrl+Shift+L",
    clickThrough: false,
  });

  let isHeaderHidden = $state<boolean>(false);

  function toggleHeaderVisibility() {
    isHeaderHidden = !isHeaderHidden;
    try {
      localStorage.setItem("ghost_lyrics_header_hidden", JSON.stringify(isHeaderHidden));
    } catch (e) {}
  }

  const modeLabel = $derived(
    settings.displayMode === "standard"
      ? "🎴 Standard"
      : settings.displayMode === "glass"
        ? "🪟 Verre"
        : "👻 Fantôme"
  );

  async function cycleDisplayMode() {
    const modes: ("standard" | "glass" | "ghost")[] = ["standard", "glass", "ghost"];
    const nextIdx = (modes.indexOf(settings.displayMode) + 1) % modes.length;
    settings.displayMode = modes[nextIdx];
    if (settings.displayMode === "standard") {
      settings.opacity = 0.88;
      isHeaderHidden = false;
    } else if (settings.displayMode === "glass") {
      settings.opacity = 0.28;
      isHeaderHidden = false;
    } else if (settings.displayMode === "ghost") {
      settings.opacity = 0.0;
      isHeaderHidden = true;
    }

    try {
      const snap = $state.snapshot(settings);
      localStorage.setItem("ghost_lyrics_settings", JSON.stringify(snap));
      localStorage.setItem("ghost_lyrics_header_hidden", JSON.stringify(isHeaderHidden));
      await emit("settings_changed", snap);
    } catch (e) {}
  }

  let hasDetectedPlayer = $state<boolean>(false);
  let isDemoMode = $state<boolean>(false);
  let isFetchingLyrics = $state<boolean>(false);
  let currentLyricsSource = $state<string>("");
  let isCurrentSynced = $state<boolean>(true);
  let availableUpdate = $state<string | null>(null);
  let currentLineIndex = $state<number>(0);
  let interpolatedPositionMs = $state<number>(0);
  let lastInterpolatedPositionMs = 0;
  let currentScrollY = $state<number>(0);
  let isManualScrolling = $state<boolean>(false);
  let autoResyncTimeout: number | null = null;

  const activeScrollY = $derived.by(() => {
    if (currentLineIndex >= 0 && lineElements[currentLineIndex]) {
      const el = lineElements[currentLineIndex];
      return el ? el.offsetTop + el.offsetHeight / 2 : 0;
    }
    return 0;
  });

  const scrollDirection = $derived.by(() => {
    if (!isManualScrolling) return "synced";
    const diff = activeScrollY - currentScrollY;
    if (diff > 45) return "down";
    if (diff < -45) return "up";
    return "here";
  });

  let lineElements: (HTMLDivElement | null)[] = $state([]);
  let animationFrameId: number;
  let pollIntervalId: number;
  let lastFetchedKey = "";
  let unlistenDemo: (() => void) | null = null;
  let unlistenManual: (() => void) | null = null;
  let unlistenSettings: (() => void) | null = null;

  async function pollMedia() {
    try {
      const media = await invoke<CurrentMediaState | null>("get_media_state");
      if (media && media.title) {
        hasDetectedPlayer = true;

        // Si l'utilisateur lance une vraie musique, quitter le mode démo
        if (isDemoMode && media.isPlaying) {
          isDemoMode = false;
        }

        if (!isDemoMode) {
          playback.isPlaying = media.isPlaying;
          playback.playbackRate = media.playbackRate > 0 ? media.playbackRate : 1.0;

          const now = Date.now();
          const currentEstimated = playback.isPlaying
            ? playback.positionMs + ((now - playback.lastUpdatedMs) * playback.playbackRate)
            : playback.positionMs;

          const drift = media.positionMs - currentEstimated;

          if (!playback.isPlaying || Math.abs(drift) > 500) {
            // Recalage immédiat si pause ou seek franc (> 500ms)
            playback.positionMs = media.positionMs;
            playback.lastUpdatedMs = now;
            lastInterpolatedPositionMs = media.positionMs + settings.timeOffsetMs;
          } else if (Math.abs(drift) > 30) {
            // Dérive modérée : lissage progressif sans jamais faire reculer le temps
            const adjusted = currentEstimated + (drift * 0.25);
            playback.positionMs = playback.isPlaying ? Math.max(adjusted, currentEstimated) : adjusted;
            playback.lastUpdatedMs = now;
          }
          // Si |drift| <= 30ms : l'horloge locale est en phase parfaite, aucun à-coup d'horloge

          const songKey = `${media.title}-${media.artist}`;
          if (songKey !== lastFetchedKey) {
            lastFetchedKey = songKey;
            currentSong = {
              title: media.title,
              artist: media.artist,
              album: media.album,
              durationMs: media.durationMs,
              sourceApp: media.sourceApp,
            };

            await loadLyricsForSong(media.title, media.artist, media.album, media.durationMs);
          } else {
            currentSong.sourceApp = media.sourceApp;
          }
        }
      } else {
        if (!isDemoMode) {
          hasDetectedPlayer = false;
        }
      }
    } catch (e) {
      // Ignorer si en dehors de Tauri
    }
  }

  async function loadLyricsForSong(title: string, artist: string, album?: string, durationMs?: number) {
    isFetchingLyrics = true;
    currentLyricsSource = "";
    lyrics = [{ startTimeMs: 0, text: `Recherche des paroles pour "${title}"...` }];
    lineElements = [];
    currentLineIndex = 0;
    currentScrollY = 0;
    lastInterpolatedPositionMs = 0;
    isManualScrolling = false;
    if (autoResyncTimeout) {
      clearTimeout(autoResyncTimeout);
      autoResyncTimeout = null;
    }

    try {
      const data = await invoke<any>("fetch_song_lyrics", {
        title,
        artist,
        album: album || null,
        durationSec: durationMs && durationMs > 0 ? durationMs / 1000 : null,
      });

      if (data && data.lines && data.lines.length > 0) {
        lyrics = data.lines;
        currentLyricsSource = data.source || "LRCLIB";
        isCurrentSynced = data.isSynced !== false;
      } else if (data && data.instrumental) {
        lyrics = [{ startTimeMs: 0, text: "🎵 Morceau instrumental identifié" }];
        currentLyricsSource = "Instrumental";
        isCurrentSynced = true;
      } else {
        lyrics = [
          { startTimeMs: 0, text: `Aucune parole trouvée pour "${title}"` },
          { startTimeMs: 4000, text: "Vérifiez le titre ou lancez un autre morceau" },
        ];
        currentLyricsSource = "";
      }
    } catch (err) {
      console.error("Erreur recherche paroles:", err);
      lyrics = [{ startTimeMs: 0, text: "Paroles introuvables (LRCLIB & lyrics.ovh)" }];
      currentLyricsSource = "";
    } finally {
      isFetchingLyrics = false;
    }
  }

  async function handleMinimize() {
    try {
      await invoke("minimize_overlay");
    } catch (e) {
      console.error("Erreur minimize:", e);
    }
  }

  async function handleOpenSettings() {
    try {
      await invoke("show_settings_window");
    } catch (e) {
      console.error("Erreur settings:", e);
    }
  }

  async function handleCloseApp() {
    try {
      await invoke("close_app");
    } catch (e) {
      console.error("Erreur close:", e);
    }
  }

  function updateInterpolation() {
    let currentPos = 0;
    if (playback.isPlaying) {
      const elapsed = Date.now() - playback.lastUpdatedMs;
      currentPos = playback.positionMs + (elapsed * playback.playbackRate) + settings.timeOffsetMs;
      // En lecture continue, l'horloge des paroles ne doit jamais régresser
      if (currentPos >= lastInterpolatedPositionMs) {
        lastInterpolatedPositionMs = currentPos;
      } else {
        currentPos = lastInterpolatedPositionMs;
      }
    } else {
      currentPos = playback.positionMs + settings.timeOffsetMs;
      lastInterpolatedPositionMs = currentPos;
    }
    interpolatedPositionMs = currentPos;

    if (lyrics.length === 0) {
      currentLineIndex = -1;
    } else if (interpolatedPositionMs < lyrics[0].startTimeMs) {
      // Intro musicale avant la première phrase de paroles
      currentLineIndex = -1;
    } else {
      let targetIndex = 0;
      for (let i = 0; i < lyrics.length; i++) {
        if (lyrics[i].startTimeMs <= interpolatedPositionMs) {
          targetIndex = i;
        } else {
          break;
        }
      }

      // Hystérésis anti-rebond :
      // 1. Pour avancer à la ligne suivante ou rester sur la même ligne : transition immédiate.
      // 2. Pour revenir à la ligne précédente : requiert un saut en arrière franc (> 250ms avant le début de la ligne).
      // Cela élimine tout effet de « va-et-vient » lors du passage d'un couplet à l'autre.
      if (currentLineIndex === -1 || targetIndex >= currentLineIndex) {
        currentLineIndex = targetIndex;
      } else {
        const currentLineStart = lyrics[currentLineIndex]?.startTimeMs ?? 0;
        if (interpolatedPositionMs < currentLineStart - 250) {
          currentLineIndex = targetIndex;
        }
      }
    }

    // Centrage automatique exact sur la ligne active via offsetTop (uniquement si défilement automatique actif)
    if (!isManualScrolling) {
      if (currentLineIndex >= 0 && lineElements[currentLineIndex]) {
        const el = lineElements[currentLineIndex];
        if (el) {
          currentScrollY = el.offsetTop + el.offsetHeight / 2;
        }
      } else if (currentLineIndex === -1 && lineElements[0]) {
        const el = lineElements[0];
        if (el) {
          currentScrollY = el.offsetTop;
        }
      }
    }

    animationFrameId = requestAnimationFrame(updateInterpolation);
  }

  function handleWheel(e: WheelEvent) {
    if (lyrics.length <= 1) return;

    // Empêche tout scroll de page natif indésirable
    e.preventDefault();

    isManualScrolling = true;

    if (autoResyncTimeout) {
      clearTimeout(autoResyncTimeout);
      autoResyncTimeout = null;
    }

    const firstEl = lineElements[0];
    const lastEl = lineElements[lyrics.length - 1];
    const minScroll = firstEl ? firstEl.offsetTop + firstEl.offsetHeight / 2 : 0;
    const maxScroll = lastEl ? lastEl.offsetTop + lastEl.offsetHeight / 2 : minScroll;

    const delta = e.deltaY;
    const minBound = Math.max(0, minScroll - 40);
    const maxBound = maxScroll + 40;

    currentScrollY = Math.max(minBound, Math.min(maxBound, currentScrollY + delta * 0.75));

    // Réarmement automatique après 10s d'inactivité
    autoResyncTimeout = window.setTimeout(() => {
      resyncLyrics();
    }, 10000);
  }

  function resyncLyrics() {
    if (autoResyncTimeout) {
      clearTimeout(autoResyncTimeout);
      autoResyncTimeout = null;
    }
    isManualScrolling = false;

    if (currentLineIndex >= 0 && lineElements[currentLineIndex]) {
      const el = lineElements[currentLineIndex];
      if (el) {
        currentScrollY = el.offsetTop + el.offsetHeight / 2;
      }
    } else if (lineElements[0]) {
      currentScrollY = lineElements[0].offsetTop;
    }
  }

  function scrollToLine(index: number) {
    if (lyrics.length <= 1 || !lineElements[index]) return;

    if (index === currentLineIndex) {
      resyncLyrics();
      return;
    }

    isManualScrolling = true;
    if (autoResyncTimeout) {
      clearTimeout(autoResyncTimeout);
    }
    const el = lineElements[index];
    if (el) {
      currentScrollY = el.offsetTop + el.offsetHeight / 2;
    }
    autoResyncTimeout = window.setTimeout(() => {
      resyncLyrics();
    }, 10000);
  }

  onMount(async () => {
    // 1. Charge les paramètres persistés
    const saved = localStorage.getItem("ghost_lyrics_settings");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        Object.assign(settings, parsed);
      } catch (e) {}
    }

    const savedHidden = localStorage.getItem("ghost_lyrics_header_hidden");
    if (savedHidden !== null) {
      try {
        isHeaderHidden = JSON.parse(savedHidden);
      } catch (e) {}
    } else if (settings.displayMode === "ghost") {
      isHeaderHidden = true;
    }

    // 2. Démarrage de la boucle d'interpolation et du polling à 350ms
    animationFrameId = requestAnimationFrame(updateInterpolation);
    pollIntervalId = window.setInterval(pollMedia, 350);
    pollMedia();

    // 3. Écouteurs d'événements Tauri
    try {
      unlistenSettings = await listen<AppSettings>("settings_changed", (event) => {
        const prevMode = settings.displayMode;
        Object.assign(settings, event.payload);
        if (event.payload.displayMode === "ghost" && prevMode !== "ghost") {
          isHeaderHidden = true;
        } else if (event.payload.displayMode !== "ghost" && prevMode === "ghost") {
          isHeaderHidden = false;
        }
      });

      unlistenDemo = await listen<{ title: string; artist: string; durationMs: number }>("play_demo_song", async (event) => {
        isDemoMode = true;
        hasDetectedPlayer = true;
        lastFetchedKey = "";
        currentSong = {
          title: event.payload.title,
          artist: event.payload.artist,
          durationMs: event.payload.durationMs,
          sourceApp: "Démo Test",
        };
        playback = {
          isPlaying: true,
          positionMs: 0,
          lastUpdatedMs: Date.now(),
          playbackRate: 1.0,
        };
        lastInterpolatedPositionMs = 0;
        await loadLyricsForSong(event.payload.title, event.payload.artist, undefined, event.payload.durationMs);
      });

      unlistenManual = await listen<{ title: string; artist: string }>("play_manual_song", async (event) => {
        isDemoMode = true;
        hasDetectedPlayer = true;
        lastFetchedKey = "";
        currentSong = {
          title: event.payload.title,
          artist: event.payload.artist,
          durationMs: 0,
          sourceApp: "Recherche manuelle",
        };
        playback = {
          isPlaying: true,
          positionMs: 0,
          lastUpdatedMs: Date.now(),
          playbackRate: 1.0,
        };
        lastInterpolatedPositionMs = 0;
        await loadLyricsForSong(event.payload.title, event.payload.artist);
      });
    } catch (e) {
      console.warn("Écouteurs d'événements non activés:", e);
    }

    // 4. Navigation clavier et touche Échap pour synchroniser
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isManualScrolling) {
        e.preventDefault();
        resyncLyrics();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        isManualScrolling = true;
        currentScrollY = Math.max(0, currentScrollY - 60);
        if (autoResyncTimeout) clearTimeout(autoResyncTimeout);
        autoResyncTimeout = window.setTimeout(resyncLyrics, 10000);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        isManualScrolling = true;
        currentScrollY = currentScrollY + 60;
        if (autoResyncTimeout) clearTimeout(autoResyncTimeout);
        autoResyncTimeout = window.setTimeout(resyncLyrics, 10000);
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    // 5. Vérification silencieuse des mises à jour au démarrage
    if (settings.autoCheckUpdates !== false) {
      try {
        invoke<any>("check_for_updates").then((info) => {
          if (info && info.hasUpdate) {
            availableUpdate = info.latestVersion;
          }
        }).catch(() => {});
      } catch (e) {}
    }
  });

  onDestroy(() => {
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
    if (pollIntervalId) clearInterval(pollIntervalId);
    if (autoResyncTimeout) clearTimeout(autoResyncTimeout);
    window.removeEventListener("keydown", handleKeyDown);
    if (unlistenDemo) unlistenDemo();
    if (unlistenManual) unlistenManual();
    if (unlistenSettings) unlistenSettings();
  });
</script>

<div
  class="overlay-container mode-{settings.displayMode} {isHeaderHidden ? 'header-hidden' : ''}"
  style="
    --bg-opacity: {settings.opacity};
    font-size: {settings.fontSize}px;
  "
  data-tauri-drag-region
>
  {#if !isHeaderHidden}
    <!-- Header Bar compact & épuré -->
    <header class="overlay-header">
      <!-- Zone Déplaçable (Drag region) -->
      <div class="drag-zone" data-tauri-drag-region>
        <span class="music-icon" data-tauri-drag-region>🎵</span>
        <div class="song-meta" data-tauri-drag-region>
          <span class="track-title" title="{currentSong.title}">{currentSong.title}</span>
          {#if currentSong.artist}
            <span class="track-artist" title="{currentSong.artist}">— {currentSong.artist}</span>
          {/if}
        </div>
      </div>

      <!-- Badges d'état et contrôles de la fenêtre -->
      <div class="header-actions">
        <!-- Sélecteur de mode d'affichage rapide -->
        <button
          class="mode-pill-btn"
          onclick={cycleDisplayMode}
          title="Style d'affichage : Standard / Verre / Fantôme (clic pour basculer)"
          aria-label="Changer de mode"
        >
          <span class="mode-text">{modeLabel}</span>
        </button>

        <!-- Indicateur d'état du lecteur ultra-compact -->
        <div
          class="player-status-pill"
          title={isDemoMode
            ? "Mode Démo actif"
            : !hasDetectedPlayer
              ? "En attente d'un lecteur (Spotify, Deezer, YouTube...)"
              : `${currentSong.sourceApp || 'Lecteur'} • ${playback.isPlaying ? 'En lecture' : 'En pause'}`}
        >
          {#if isDemoMode}
            <span class="status-dot dot-demo"></span>
            <span class="status-text">Démo</span>
          {:else if !hasDetectedPlayer}
            <span class="status-dot dot-waiting"></span>
            <span class="status-text">Attente</span>
          {:else if playback.isPlaying}
            <span class="status-dot dot-playing"></span>
            <span class="status-text">{currentSong.sourceApp || "Actif"}</span>
          {:else}
            <span class="status-dot dot-paused"></span>
            <span class="status-text">Pause</span>
          {/if}
        </div>

        {#if isFetchingLyrics}
          <span class="badge badge-loading" title="Recherche des paroles en cours...">⏳</span>
        {:else if currentLyricsSource}
          <span
            class="badge {isCurrentSynced ? 'badge-synced' : 'badge-paced'}"
            title={isCurrentSynced ? `Synchronisé (.lrc) via ${currentLyricsSource}` : `Défilement temporel estimé via ${currentLyricsSource}`}
          >
            {isCurrentSynced ? '🟢 LRC' : '🟡 Texte'}
          </span>
        {/if}

        {#if availableUpdate}
          <button
            class="badge badge-update-alert"
            onclick={handleOpenSettings}
            title="Nouvelle version v{availableUpdate} disponible ! Cliquez pour mettre à jour."
          >
            ✨ v{availableUpdate}
          </button>
        {/if}

        {#if isManualScrolling}
          <button
            class="badge badge-sync-btn"
            onclick={resyncLyrics}
            title="Défilement manuel actif. Cliquez pour revenir au couplet en direct (ou Échap)"
          >
            🔄 Direct
          </button>
        {/if}

        <!-- Boutons de contrôle -->
        <div class="window-controls">
          <!-- Bouton pour masquer la barre de menu (Mode Paroles Seules) -->
          <button
            class="ctrl-btn btn-toggle-header"
            onclick={toggleHeaderVisibility}
            title="Masquer les menus (Mode épuré / Paroles seules)"
            aria-label="Masquer les menus"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path>
              <line x1="1" y1="1" x2="23" y2="23"></line>
            </svg>
          </button>
          <button
            class="ctrl-btn btn-settings"
            onclick={handleOpenSettings}
            title="Ouvrir les paramètres (⚙️)"
            aria-label="Paramètres"
          >
            ⚙️
          </button>
          <button
            class="ctrl-btn btn-minimize"
            onclick={handleMinimize}
            title="Réduire dans la zone de notification Windows (icônes cachées)"
            aria-label="Réduire"
          >
            —
          </button>
          <button
            class="ctrl-btn btn-close"
            onclick={handleCloseApp}
            title="Quitter GhostLyrics"
            aria-label="Quitter"
          >
            ✕
          </button>
        </div>
      </div>
    </header>
  {:else}
    <!-- Mini Dock Flottant : Visible quand les menus sont masqués (Paroles seules) -->
    <div class="mini-floating-dock" data-tauri-drag-region>
      {#if isManualScrolling}
        <button
          class="mini-dock-btn mini-dock-sync-btn"
          onclick={resyncLyrics}
          title="Revenir au couplet en direct (ou Échap)"
          aria-label="Synchroniser"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2"/>
          </svg>
        </button>
      {/if}
      <button
        class="mini-dock-btn"
        onclick={toggleHeaderVisibility}
        title="Afficher les menus et contrôles"
        aria-label="Afficher les menus"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
          <circle cx="12" cy="12" r="3"></circle>
        </svg>
      </button>
    </div>
  {/if}

  <!-- Lyrics Display Area : Centrage vertical exact sur la ligne active -->
  <main class="lyrics-viewport" data-tauri-drag-region onwheel={handleWheel}>
    <div
      class="lyrics-list"
      style="transform: translateY(-{currentScrollY}px);"
      data-tauri-drag-region
    >
      {#each lyrics as line, index}
        <div
          bind:this={lineElements[index]}
          class="lyric-line {index === currentLineIndex ? 'active' : ''} {isManualScrolling ? 'interactive' : ''}"
          role="button"
          tabindex="0"
          onclick={() => scrollToLine(index)}
          onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              scrollToLine(index);
            }
          }}
          style="
            color: {index === currentLineIndex ? settings.activeColor : settings.textColor};
            font-size: {index === currentLineIndex ? settings.fontSize * 1.16 : settings.fontSize}px;
          "
        >
          {line.text}
        </div>
      {/each}
    </div>

    <!-- Bouton Flottant de Synchronisation -->
    {#if isManualScrolling}
      <div class="sync-pill-container">
        <button
          class="sync-pill-btn"
          onclick={resyncLyrics}
          title="Revenir au couplet en direct (Échap pour synchroniser)"
          aria-label="Synchroniser"
        >
          <span class="sync-pulse-dot"></span>
          <span class="sync-label">
            {#if scrollDirection === 'down'}
              ↓ Revenir au direct
            {:else if scrollDirection === 'up'}
              ↑ Revenir au direct
            {:else}
              🎯 Synchroniser
            {/if}
          </span>
          <span class="sync-hotkey-badge">Échap</span>
        </button>
      </div>
    {/if}
  </main>
</div>

<style>
  .overlay-container {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    padding: 6px 14px 10px 14px;
    border-radius: 12px;
    overflow: hidden;
    user-select: none;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    transition: background 0.25s ease, border 0.25s ease, box-shadow 0.25s ease;
  }

  .overlay-container.header-hidden {
    padding-top: 4px;
  }

  /* Mini Dock Flottant : Apparaît quand les menus sont masqués */
  .mini-floating-dock {
    position: absolute;
    top: 6px;
    right: 8px;
    z-index: 50;
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0.2;
    transition: opacity 0.2s ease, transform 0.15s ease;
  }

  .mini-floating-dock:hover {
    opacity: 1;
    transform: scale(1.05);
  }

  .mini-dock-btn {
    background: rgba(15, 15, 20, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.18);
    color: #e2e8f0;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    transition: all 0.15s ease;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .mini-dock-btn:hover {
    background: rgba(56, 189, 248, 0.3);
    border-color: rgba(56, 189, 248, 0.6);
    color: #38bdf8;
  }

  /* Mode 1: Standard (Verre dépoli complet) */
  .mode-standard {
    background: rgba(15, 15, 20, var(--bg-opacity, 0.88));
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 10px 36px 0 rgba(0, 0, 0, 0.45);
  }

  /* Mode 2: Verre Discret (Subtil et léger) */
  .mode-glass {
    background: rgba(15, 15, 20, var(--bg-opacity, 0.28));
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.05);
    box-shadow: none;
  }

  /* Mode 3: Fantôme Minimaliste (Fond 100% invisible, seules les paroles flottent) */
  .mode-ghost {
    background: transparent !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    border: none !important;
    box-shadow: none !important;
  }

  /* En mode Fantôme, l'en-tête est très discret quand affiché */
  .mode-ghost .overlay-header {
    opacity: 0.4;
    transition: opacity 0.2s ease;
    border-bottom: 1px solid transparent;
  }

  .mode-ghost:hover .overlay-header {
    opacity: 1;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .overlay-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-bottom: 5px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    min-height: 26px;
    z-index: 10;
  }

  .drag-zone {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    cursor: grab;
    white-space: nowrap;
    overflow: hidden;
  }

  .drag-zone:active {
    cursor: grabbing;
  }

  .music-icon {
    font-size: 0.8em;
    opacity: 0.8;
    flex-shrink: 0;
  }

  .song-meta {
    display: flex;
    align-items: baseline;
    gap: 4px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .track-title {
    font-weight: 600;
    color: #f8fafc;
    font-size: 0.76em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
    max-width: 140px;
  }

  .track-artist {
    color: #94a3b8;
    font-size: 0.72em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
    max-width: 100px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
  }

  .mode-pill-btn {
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #f1f5f9;
    font-size: 0.68em;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 9999px;
    cursor: pointer;
    transition: all 0.15s ease;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    white-space: nowrap;
  }

  .mode-pill-btn:hover {
    background: rgba(56, 189, 248, 0.25);
    border-color: rgba(56, 189, 248, 0.5);
    color: #38bdf8;
  }

  .player-status-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: rgba(0, 0, 0, 0.35);
    padding: 2px 7px;
    border-radius: 9999px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 0.68em;
    font-weight: 500;
    color: #e2e8f0;
    white-space: nowrap;
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }

  .dot-waiting {
    background-color: #ef4444;
    box-shadow: 0 0 6px rgba(239, 68, 68, 0.6);
  }

  .dot-playing {
    background-color: #22c55e;
    box-shadow: 0 0 6px rgba(34, 197, 94, 0.7);
    animation: pulse 2s infinite;
  }

  .dot-paused {
    background-color: #eab308;
    box-shadow: 0 0 6px rgba(234, 179, 8, 0.6);
  }

  .dot-demo {
    background-color: #a855f7;
    box-shadow: 0 0 6px rgba(168, 85, 247, 0.7);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .badge {
    font-size: 0.66em;
    padding: 2px 6px;
    border-radius: 9999px;
    font-weight: 500;
    white-space: nowrap;
  }

  .badge-loading {
    background: rgba(56, 189, 248, 0.2);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
  }

  .badge-synced {
    background: rgba(34, 197, 94, 0.2);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.35);
  }

  .badge-paced {
    background: rgba(234, 179, 8, 0.2);
    color: #facc15;
    border: 1px solid rgba(234, 179, 8, 0.35);
  }

  .badge-update-alert {
    background: rgba(168, 85, 247, 0.25);
    color: #d8b4fe;
    border: 1px solid rgba(168, 85, 247, 0.5);
    cursor: pointer;
    transition: all 0.2s ease;
    animation: pulse 2.5s infinite;
  }

  .badge-update-alert:hover {
    background: rgba(168, 85, 247, 0.45);
    color: #ffffff;
    transform: scale(1.05);
  }

  .window-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    background: rgba(0, 0, 0, 0.4);
    padding: 1px 3px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .ctrl-btn {
    background: transparent;
    border: none;
    color: #cbd5e1;
    cursor: pointer;
    font-size: 0.78em;
    width: 22px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: all 0.15s ease;
    padding: 0;
  }

  .ctrl-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #ffffff;
  }

  .btn-toggle-header:hover {
    background: rgba(56, 189, 248, 0.25);
    color: #38bdf8;
  }

  .btn-close:hover {
    background: #ef4444;
    color: #ffffff;
  }

  .lyrics-viewport {
    position: relative;
    flex: 1;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, transparent 0%, black 20%, black 80%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 20%, black 80%, transparent 100%);
  }

  .lyrics-list {
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    transition: transform 0.4s cubic-bezier(0.25, 1, 0.5, 1);
    will-change: transform;
  }

  .lyric-line {
    text-align: center;
    padding: 6px 16px;
    transition: all 0.3s ease;
    font-weight: 600;
    line-height: 1.4;
    /* Ombres haute intensité : garantit une lisibilité totale même sans aucun fond */
    text-shadow:
      0 2px 4px rgba(0, 0, 0, 0.95),
      0 0 8px rgba(0, 0, 0, 0.9),
      0 0 2px #000000;
  }

  .lyric-line.active {
    font-weight: 800;
    transform: scale(1.06);
    /* Halo lumineux accentué + contour noir profond */
    text-shadow:
      0 0 16px rgba(56, 189, 248, 0.6),
      0 2px 6px #000000,
      0 0 8px #000000,
      0 0 2px #000000;
  }

  .lyric-line.interactive {
    cursor: pointer;
  }

  .lyric-line.interactive:hover {
    opacity: 0.95;
    text-shadow:
      0 0 12px rgba(255, 255, 255, 0.5),
      0 2px 4px #000000;
  }

  /* Bouton Flottant de Synchronisation (Mode Défilement Libre) */
  .sync-pill-container {
    position: absolute;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 70;
    pointer-events: auto;
    animation: syncSlideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes syncSlideUp {
    from {
      opacity: 0;
      transform: translate(-50%, 10px) scale(0.92);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0) scale(1);
    }
  }

  .sync-pill-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    background: rgba(15, 23, 42, 0.88);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(56, 189, 248, 0.5);
    color: #38bdf8;
    padding: 5px 13px;
    border-radius: 9999px;
    font-size: 0.76rem;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.6), 0 0 12px rgba(56, 189, 248, 0.3);
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .sync-pill-btn:hover {
    background: rgba(56, 189, 248, 0.25);
    border-color: #38bdf8;
    color: #ffffff;
    box-shadow: 0 6px 24px rgba(56, 189, 248, 0.5);
    transform: scale(1.04);
  }

  .sync-pill-btn:active {
    transform: scale(0.97);
  }

  .sync-pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: #22c55e;
    box-shadow: 0 0 8px #22c55e;
    animation: pulse 1.5s infinite;
    flex-shrink: 0;
  }

  .sync-label {
    white-space: nowrap;
    letter-spacing: 0.02em;
  }

  .sync-hotkey-badge {
    font-size: 0.68em;
    font-weight: 600;
    padding: 1px 4px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.75);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .badge-sync-btn {
    background: rgba(56, 189, 248, 0.22);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.45);
    cursor: pointer;
    font-weight: 700;
    transition: all 0.15s ease;
    animation: pulse 2s infinite;
  }

  .badge-sync-btn:hover {
    background: rgba(56, 189, 248, 0.4);
    color: #ffffff;
    border-color: #38bdf8;
    transform: scale(1.05);
  }

  .mini-dock-sync-btn {
    background: rgba(56, 189, 248, 0.25);
    color: #38bdf8;
    border-color: rgba(56, 189, 248, 0.5);
    animation: pulse 2s infinite;
  }

  .mini-dock-sync-btn:hover {
    background: rgba(56, 189, 248, 0.45);
    color: #ffffff;
    border-color: #38bdf8;
  }
</style>
