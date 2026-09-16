<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
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

  async function cycleDisplayMode() {
    const modes: ("standard" | "glass" | "ghost")[] = ["standard", "glass", "ghost"];
    const nextIdx = (modes.indexOf(settings.displayMode) + 1) % modes.length;
    settings.displayMode = modes[nextIdx];
    if (settings.displayMode === "standard") settings.opacity = 0.88;
    else if (settings.displayMode === "glass") settings.opacity = 0.28;
    else if (settings.displayMode === "ghost") settings.opacity = 0.0;

    try {
      localStorage.setItem("ghost_lyrics_settings", JSON.stringify($state.snapshot(settings)));
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
  let currentScrollY = $state<number>(0);

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
          playback.lastUpdatedMs = media.lastUpdatedMs;

          // Si le décalage avec l'interpolation dépasse 400ms (ex: seek ou reprise), recalage immédiat
          const diff = Math.abs(interpolatedPositionMs - (media.positionMs + settings.timeOffsetMs));
          if (!playback.isPlaying || diff > 400) {
            playback.positionMs = media.positionMs;
          }

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
    if (playback.isPlaying) {
      const elapsed = Date.now() - playback.lastUpdatedMs;
      interpolatedPositionMs = playback.positionMs + (elapsed * playback.playbackRate) + settings.timeOffsetMs;
    } else {
      interpolatedPositionMs = playback.positionMs + settings.timeOffsetMs;
    }

    if (lyrics.length === 0) {
      currentLineIndex = -1;
    } else if (interpolatedPositionMs < lyrics[0].startTimeMs) {
      // Intro musicale avant la première phrase de paroles
      currentLineIndex = -1;
    } else {
      let foundIndex = 0;
      for (let i = 0; i < lyrics.length; i++) {
        if (lyrics[i].startTimeMs <= interpolatedPositionMs) {
          foundIndex = i;
        } else {
          break;
        }
      }
      currentLineIndex = foundIndex;
    }

    // Centrage automatique exact sur la ligne active via offsetTop
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

    animationFrameId = requestAnimationFrame(updateInterpolation);
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

    // 2. Démarrage de la boucle d'interpolation et du polling à 350ms
    animationFrameId = requestAnimationFrame(updateInterpolation);
    pollIntervalId = window.setInterval(pollMedia, 350);
    pollMedia();

    // 3. Écouteurs d'événements Tauri
    try {
      unlistenSettings = await listen<AppSettings>("settings_changed", (event) => {
        Object.assign(settings, event.payload);
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
        await loadLyricsForSong(event.payload.title, event.payload.artist);
      });
    } catch (e) {
      console.warn("Écouteurs d'événements non activés:", e);
    }

    // 4. Vérification silencieuse des mises à jour au démarrage
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
    if (unlistenDemo) unlistenDemo();
    if (unlistenManual) unlistenManual();
    if (unlistenSettings) unlistenSettings();
  });
</script>

<div
  class="overlay-container mode-{settings.displayMode}"
  style="
    --bg-opacity: {settings.opacity};
    font-size: {settings.fontSize}px;
  "
>
  <!-- Header Bar -->
  <header class="overlay-header">
    <!-- Zone Déplaçable (Drag region) -->
    <div class="drag-zone" data-tauri-drag-region>
      <span class="music-icon">🎵</span>
      <div class="song-meta">
        <span class="track-title">{currentSong.title}</span>
        <span class="track-artist">— {currentSong.artist}</span>
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
        {#if settings.displayMode === 'standard'}
          🎴 Standard
        {:else if settings.displayMode === 'glass'}
          🪟 Verre
        {:else}
          👻 Fantôme
        {/if}
      </button>

      <!-- Indicateur d'état du lecteur -->
      <div class="player-status-pill" title="Statut de la détection Windows Media Controls">
        {#if isDemoMode}
          <span class="status-dot dot-demo"></span>
          <span class="status-text">Mode Démo</span>
        {:else if !hasDetectedPlayer}
          <span class="status-dot dot-waiting"></span>
          <span class="status-text">En attente d'un lecteur</span>
        {:else if playback.isPlaying}
          <span class="status-dot dot-playing"></span>
          <span class="status-text">{currentSong.sourceApp || "Lecteur"} • En lecture</span>
        {:else}
          <span class="status-dot dot-paused"></span>
          <span class="status-text">{currentSong.sourceApp || "Lecteur"} • En pause</span>
        {/if}
      </div>

      {#if isFetchingLyrics}
        <span class="badge badge-loading">⏳ Recherche...</span>
      {:else if currentLyricsSource}
        <span
          class="badge {isCurrentSynced ? 'badge-synced' : 'badge-paced'}"
          title={isCurrentSynced ? "Paroles synchronisées à la milliseconde (.lrc)" : "Paroles textuelles défilantes avec rythme temporel estimé"}
        >
          {isCurrentSynced ? '🟢 ' : '🟡 '}{currentLyricsSource}
        </span>
      {/if}

      {#if availableUpdate}
        <button
          class="badge badge-update-alert"
          onclick={handleOpenSettings}
          title="Nouvelle version v{availableUpdate} disponible ! Cliquez pour ouvrir les paramètres et mettre à jour."
        >
          ✨ v{availableUpdate} dispo
        </button>
      {/if}

      <!-- Boutons de contrôle -->
      <div class="window-controls">
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

  <!-- Lyrics Display Area : Centrage vertical exact sur la ligne active -->
  <main class="lyrics-viewport">
    <div
      class="lyrics-list"
      style="transform: translateY(-{currentScrollY}px);"
    >
      {#each lyrics as line, index}
        <div
          bind:this={lineElements[index]}
          class="lyric-line {index === currentLineIndex ? 'active' : ''}"
          style="
            color: {index === currentLineIndex ? settings.activeColor : settings.textColor};
            font-size: {index === currentLineIndex ? settings.fontSize * 1.16 : settings.fontSize}px;
          "
        >
          {line.text}
        </div>
      {/each}
    </div>
  </main>
</div>

<style>
  .overlay-container {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    padding: 10px 16px;
    border-radius: 12px;
    overflow: hidden;
    user-select: none;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    transition: background 0.25s ease, border 0.25s ease, box-shadow 0.25s ease;
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

  /* En mode Fantôme, l'en-tête est très discret au repos pour un rendu épuré */
  .mode-ghost .overlay-header {
    opacity: 0.15;
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
    gap: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    min-height: 32px;
    z-index: 10;
  }

  .drag-zone {
    display: flex;
    align-items: center;
    gap: 8px;
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
    font-size: 0.9em;
    opacity: 0.9;
  }

  .song-meta {
    display: flex;
    align-items: baseline;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-title {
    font-weight: 600;
    color: #f8fafc;
    font-size: 0.85em;
    overflow: hidden;
    text-overflow: ellipsis;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
  }

  .track-artist {
    color: #cbd5e1;
    font-size: 0.8em;
    overflow: hidden;
    text-overflow: ellipsis;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .mode-pill-btn {
    background: rgba(0, 0, 0, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.14);
    color: #f1f5f9;
    font-size: 0.72em;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: 9999px;
    cursor: pointer;
    transition: all 0.15s ease;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .mode-pill-btn:hover {
    background: rgba(56, 189, 248, 0.25);
    border-color: rgba(56, 189, 248, 0.5);
    color: #38bdf8;
  }

  .player-status-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: rgba(0, 0, 0, 0.45);
    padding: 3px 10px;
    border-radius: 9999px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 0.72em;
    font-weight: 500;
    color: #e2e8f0;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    display: inline-block;
  }

  .dot-waiting {
    background-color: #ef4444;
    box-shadow: 0 0 8px rgba(239, 68, 68, 0.6);
  }

  .dot-playing {
    background-color: #22c55e;
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.7);
    animation: pulse 2s infinite;
  }

  .dot-paused {
    background-color: #eab308;
    box-shadow: 0 0 8px rgba(234, 179, 8, 0.6);
  }

  .dot-demo {
    background-color: #a855f7;
    box-shadow: 0 0 8px rgba(168, 85, 247, 0.7);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .badge {
    font-size: 0.7em;
    padding: 3px 8px;
    border-radius: 9999px;
    font-weight: 500;
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
    gap: 4px;
    background: rgba(0, 0, 0, 0.4);
    padding: 2px 4px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .ctrl-btn {
    background: transparent;
    border: none;
    color: #cbd5e1;
    cursor: pointer;
    font-size: 0.85em;
    width: 26px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 5px;
    transition: all 0.15s ease;
    padding: 0;
  }

  .ctrl-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #ffffff;
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
</style>
