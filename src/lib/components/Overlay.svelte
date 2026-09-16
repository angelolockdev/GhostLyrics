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
    { startTimeMs: 0, text: "GhostLyrics — En attente du lecteur" },
    { startTimeMs: 3000, text: "Détection automatique via Windows Media Controls" },
    { startTimeMs: 6000, text: "Prêt à afficher vos morceaux en temps réel" },
  ]);

  let settings = $state<AppSettings>({
    fontSize: 22,
    opacity: 0.88,
    textColor: "rgba(255, 255, 255, 0.45)",
    activeColor: "#38bdf8",
    backgroundColor: "rgba(15, 15, 20, 0.82)",
    timeOffsetMs: 0,
    hotkey: "Ctrl+Shift+L",
    clickThrough: false,
  });

  let hasDetectedPlayer = $state<boolean>(false);
  let isDemoMode = $state<boolean>(false);
  let isFetchingLyrics = $state<boolean>(false);
  let currentLineIndex = $state<number>(0);
  let interpolatedPositionMs = $state<number>(0);

  let animationFrameId: number;
  let pollIntervalId: number;
  let lastFetchedKey = "";
  let unlistenDemo: (() => void) | null = null;
  let unlistenManual: (() => void) | null = null;

  async function pollMedia() {
    if (isDemoMode) return;

    try {
      const media = await invoke<CurrentMediaState | null>("get_media_state");
      if (media && media.title) {
        hasDetectedPlayer = true;
        playback.isPlaying = media.isPlaying;
        playback.positionMs = media.positionMs;
        playback.lastUpdatedMs = media.lastUpdatedMs;
        playback.playbackRate = media.playbackRate || 1.0;

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
      } else {
        hasDetectedPlayer = false;
      }
    } catch (e) {
      // Ignorer si en dehors de Tauri (test web)
    }
  }

  async function loadLyricsForSong(title: string, artist: string, album?: string, durationMs?: number) {
    isFetchingLyrics = true;
    lyrics = [{ startTimeMs: 0, text: `Recherche des paroles pour "${title}"...` }];

    try {
      const data = await invoke<any>("fetch_song_lyrics", {
        title,
        artist,
        album: album || null,
        durationSec: durationMs && durationMs > 0 ? durationMs / 1000 : null,
      });

      if (data && data.lines && data.lines.length > 0) {
        lyrics = data.lines;
      } else if (data && data.instrumental) {
        lyrics = [{ startTimeMs: 0, text: "🎵 Morceau instrumental identifié" }];
      } else {
        lyrics = [
          { startTimeMs: 0, text: `Aucune parole synchronisée pour "${title}"` },
          { startTimeMs: 4000, text: "Vérifiez le titre ou importez un fichier .lrc dans les Paramètres" },
        ];
      }
    } catch (err) {
      console.error("Erreur LRCLIB:", err);
      lyrics = [{ startTimeMs: 0, text: "Paroles non trouvées sur LRCLIB" }];
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

    // Trouve la ligne de parole active
    let foundIndex = 0;
    for (let i = 0; i < lyrics.length; i++) {
      if (lyrics[i].startTimeMs <= interpolatedPositionMs) {
        foundIndex = i;
      } else {
        break;
      }
    }
    currentLineIndex = foundIndex;

    animationFrameId = requestAnimationFrame(updateInterpolation);
  }

  onMount(async () => {
    animationFrameId = requestAnimationFrame(updateInterpolation);
    pollIntervalId = window.setInterval(pollMedia, 1000);
    pollMedia();

    try {
      unlistenDemo = await listen<{ title: string; artist: string; durationMs: number }>("play_demo_song", async (event) => {
        isDemoMode = true;
        hasDetectedPlayer = true;
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
  });

  onDestroy(() => {
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
    if (pollIntervalId) clearInterval(pollIntervalId);
    if (unlistenDemo) unlistenDemo();
    if (unlistenManual) unlistenManual();
  });
</script>

<div
  class="overlay-container"
  style="
    background: {settings.backgroundColor};
    opacity: {settings.opacity};
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
        <span class="badge badge-loading">⏳ LRCLIB...</span>
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

  <!-- Lyrics Display Area -->
  <main class="lyrics-viewport">
    <div
      class="lyrics-list"
      style="transform: translateY(-{currentLineIndex * (settings.fontSize * 1.8)}px);"
    >
      {#each lyrics as line, index}
        <div
          class="lyric-line {index === currentLineIndex ? 'active' : ''}"
          style="
            color: {index === currentLineIndex ? settings.activeColor : settings.textColor};
            font-size: {index === currentLineIndex ? settings.fontSize * 1.15 : settings.fontSize}px;
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
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 10px 36px 0 rgba(0, 0, 0, 0.45);
    overflow: hidden;
    user-select: none;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  .overlay-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    min-height: 32px;
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
  }

  .track-artist {
    color: #94a3b8;
    font-size: 0.8em;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .player-status-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: rgba(0, 0, 0, 0.4);
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

  .window-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    background: rgba(0, 0, 0, 0.3);
    padding: 2px 4px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.06);
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
    display: flex;
    flex-direction: column;
    justify-content: center;
    mask-image: linear-gradient(to bottom, transparent 0%, black 22%, black 78%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 22%, black 78%, transparent 100%);
  }

  .lyrics-list {
    transition: transform 0.35s cubic-bezier(0.25, 1, 0.5, 1);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .lyric-line {
    text-align: center;
    padding: 6px 14px;
    transition: all 0.3s ease;
    font-weight: 500;
    line-height: 1.4;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.7);
  }

  .lyric-line.active {
    font-weight: 700;
    transform: scale(1.05);
    text-shadow: 0 0 16px rgba(56, 189, 248, 0.4);
  }
</style>
