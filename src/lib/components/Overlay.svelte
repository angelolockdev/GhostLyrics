<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { LyricLine, SongMetadata, PlaybackState, AppSettings } from "../types";

  // State using Svelte 5 Runes
  let currentSong = $state<SongMetadata>({
    title: "En attente de lecture...",
    artist: "Lancez une musique sur Spotify ou YouTube",
    durationMs: 0,
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
    opacity: 0.85,
    textColor: "rgba(255, 255, 255, 0.45)",
    activeColor: "#ffffff",
    backgroundColor: "rgba(15, 15, 20, 0.75)",
    timeOffsetMs: 0,
    hotkey: "Ctrl+Shift+L",
    clickThrough: false,
  });

  let currentLineIndex = $state<number>(0);
  let interpolatedPositionMs = $state<number>(0);
  let animationFrameId: number;
  let pollIntervalId: number;
  let lastFetchedTitle = "";

  async function pollMedia() {
    try {
      const media = await invoke<any>("get_media_state");
      if (media && media.title) {
        playback.isPlaying = media.isPlaying;
        playback.positionMs = media.positionMs;
        playback.lastUpdatedMs = media.lastUpdatedMs;
        playback.playbackRate = media.playbackRate || 1.0;

        if (media.title !== lastFetchedTitle) {
          lastFetchedTitle = media.title;
          currentSong = {
            title: media.title,
            artist: media.artist,
            album: media.album,
            durationMs: media.durationMs,
          };

          lyrics = [{ startTimeMs: 0, text: `Recherche des paroles pour ${media.title}...` }];

          try {
            const data = await invoke<any>("fetch_song_lyrics", {
              title: media.title,
              artist: media.artist,
              album: media.album || null,
              durationSec: media.durationMs > 0 ? media.durationMs / 1000 : null,
            });

            if (data && data.lines && data.lines.length > 0) {
              lyrics = data.lines;
            } else if (data && data.instrumental) {
              lyrics = [{ startTimeMs: 0, text: "🎵 Morceau instrumental" }];
            } else {
              lyrics = [{ startTimeMs: 0, text: "Aucune parole synchronisée trouvée" }];
            }
          } catch (err) {
            console.error("Erreur lors de la récupération des paroles:", err);
            lyrics = [{ startTimeMs: 0, text: "Paroles non disponibles" }];
          }
        }
      }
    } catch (e) {
      // Ignorer si en dehors de Tauri (navigateur)
    }
  }

  async function toggleClickThrough() {
    settings.clickThrough = !settings.clickThrough;
    try {
      await invoke("set_overlay_click_through", { enabled: settings.clickThrough });
    } catch (e) {
      console.error("Erreur click-through:", e);
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

  onMount(() => {
    animationFrameId = requestAnimationFrame(updateInterpolation);
    pollIntervalId = window.setInterval(pollMedia, 1000);
    pollMedia();
  });

  onDestroy(() => {
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
    if (pollIntervalId) clearInterval(pollIntervalId);
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
  <!-- Drag Handle Header (active when not in click-through mode) -->
  <div class="header-drag-zone" data-tauri-drag-region>
    <div class="song-info">
      <span class="music-icon">🎵</span>
      <span class="track-title">{currentSong.title}</span>
      <span class="track-artist">— {currentSong.artist}</span>
    </div>
    <div class="status-badges">
      <span class="badge {settings.clickThrough ? 'badge-active' : ''}">
        {settings.clickThrough ? "👻 Mode Fantôme (Verrouillé)" : "🔓 Déplaçable"}
      </span>
    </div>
  </div>

  <!-- Lyrics Display Area -->
  <div class="lyrics-viewport">
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
  </div>
</div>

<style>
  .overlay-container {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    padding: 12px 18px;
    border-radius: 12px;
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.37);
    overflow: hidden;
    transition: background 0.3s ease;
  }

  .header-drag-zone {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .song-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .music-icon {
    opacity: 0.8;
  }

  .track-title {
    font-weight: 600;
    color: #f1f5f9;
  }

  .track-artist {
    color: #94a3b8;
  }

  .badge {
    font-size: 0.65em;
    padding: 2px 8px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.1);
    color: #cbd5e1;
    font-weight: 500;
  }

  .badge-active {
    background: rgba(56, 189, 248, 0.2);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
  }

  .lyrics-viewport {
    position: relative;
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: center;
    mask-image: linear-gradient(to bottom, transparent 0%, black 25%, black 75%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 25%, black 75%, transparent 100%);
  }

  .lyrics-list {
    transition: transform 0.35s cubic-bezier(0.25, 1, 0.5, 1);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .lyric-line {
    text-align: center;
    padding: 6px 12px;
    transition: all 0.3s ease;
    font-weight: 500;
    line-height: 1.4;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.6);
  }

  .lyric-line.active {
    font-weight: 700;
    transform: scale(1.04);
    text-shadow: 0 0 12px rgba(255, 255, 255, 0.35);
  }
</style>
