<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen } from "@tauri-apps/api/event";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { register, unregisterAll } from "@tauri-apps/plugin-global-shortcut";
  import type { LyricLine, LyricWord, SongMetadata, PlaybackState, AppSettings, CurrentMediaState } from "../types";

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
    {
      startTimeMs: 0,
      endTimeMs: 4000,
      text: "GhostLyrics — Prêt à afficher vos paroles",
      words: [
        { text: "GhostLyrics", startTimeMs: 0, endTimeMs: 1400 },
        { text: "—", startTimeMs: 1400, endTimeMs: 1800 },
        { text: "Prêt", startTimeMs: 1800, endTimeMs: 2400 },
        { text: "à", startTimeMs: 2400, endTimeMs: 2700 },
        { text: "afficher", startTimeMs: 2700, endTimeMs: 3400 },
        { text: "vos", startTimeMs: 3400, endTimeMs: 3700 },
        { text: "paroles", startTimeMs: 3700, endTimeMs: 4000 },
      ],
    },
    {
      startTimeMs: 4000,
      endTimeMs: 8000,
      text: "Lancez Spotify ou YouTube pour démarrer la synchronisation",
      words: [
        { text: "Lancez", startTimeMs: 4000, endTimeMs: 4600 },
        { text: "Spotify", startTimeMs: 4600, endTimeMs: 5400 },
        { text: "ou", startTimeMs: 5400, endTimeMs: 5700 },
        { text: "YouTube", startTimeMs: 5700, endTimeMs: 6500 },
        { text: "pour", startTimeMs: 6500, endTimeMs: 6900 },
        { text: "démarrer", startTimeMs: 6900, endTimeMs: 7500 },
        { text: "la", startTimeMs: 7500, endTimeMs: 7700 },
        { text: "synchronisation", startTimeMs: 7700, endTimeMs: 8000 },
      ],
    },
    {
      startTimeMs: 8000,
      endTimeMs: 12000,
      text: "Ou testez la démo instantanée dans les Paramètres (⚙️)",
      words: [
        { text: "Ou", startTimeMs: 8000, endTimeMs: 8300 },
        { text: "testez", startTimeMs: 8300, endTimeMs: 8900 },
        { text: "la", startTimeMs: 8900, endTimeMs: 9100 },
        { text: "démo", startTimeMs: 9100, endTimeMs: 9800 },
        { text: "instantanée", startTimeMs: 9800, endTimeMs: 10800 },
        { text: "dans", startTimeMs: 10800, endTimeMs: 11100 },
        { text: "les", startTimeMs: 11100, endTimeMs: 11300 },
        { text: "Paramètres", startTimeMs: 11300, endTimeMs: 12000 },
      ],
    },
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
    hudHotkey: "Ctrl+Shift+H",
    clickThrough: false,
    auroraMode: "fluid",
  });

  let auroraColors = $state<{ c1: string; c2: string; c3: string }>({
    c1: "rgba(56, 189, 248, 0.25)",
    c2: "rgba(129, 140, 248, 0.22)",
    c3: "rgba(236, 72, 153, 0.18)",
  });

  function updateAuroraPalette(title: string, artist: string) {
    let hash = 0;
    const str = `${title}-${artist}`;
    for (let i = 0; i < str.length; i++) {
      hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const hue1 = Math.abs(hash) % 360;
    const hue2 = (hue1 + 45) % 360;
    const hue3 = (hue1 + 120) % 360;

    auroraColors = {
      c1: `hsla(${hue1}, 85%, 55%, 0.26)`,
      c2: `hsla(${hue2}, 80%, 50%, 0.22)`,
      c3: `hsla(${hue3}, 75%, 48%, 0.18)`,
    };
  }

  function getWordStatus(word: { startTimeMs: number; endTimeMs: number }, currentPosMs: number) {
    if (currentPosMs >= word.endTimeMs) {
      return { state: "completed", progress: 100 };
    }
    if (currentPosMs < word.startTimeMs) {
      return { state: "upcoming", progress: 0 };
    }
    const duration = Math.max(1, word.endTimeMs - word.startTimeMs);
    const progress = Math.min(100, Math.max(0, ((currentPosMs - word.startTimeMs) / duration) * 100));
    return { state: "singing", progress };
  }

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
        : settings.displayMode === "hud"
          ? "💊 HUD"
          : "👻 Fantôme"
  );

  async function notifySettingsChanged() {
    try {
      const snap = $state.snapshot(settings);
      localStorage.setItem("ghost_lyrics_settings", JSON.stringify(snap));
      localStorage.setItem("ghost_lyrics_header_hidden", JSON.stringify(isHeaderHidden));
      await emit("settings_changed", snap);
    } catch (e) {}
  }

  async function applyDisplayMode(newMode: "standard" | "glass" | "ghost" | "hud") {
    const prevMode = settings.displayMode;
    settings.displayMode = newMode;

    try {
      const appWin = getCurrentWindow();
      if (newMode === "hud") {
        settings.opacity = 0.90;
        isHeaderHidden = true;
        const curSize = await appWin.innerSize();
        if (prevMode !== "hud") {
          localStorage.setItem("ghost_lyrics_prev_size", JSON.stringify({ width: curSize.width, height: curSize.height }));
        }
        await appWin.setSize(new LogicalSize(480, 84));
      } else {
        if (prevMode === "hud") {
          const savedSizeStr = localStorage.getItem("ghost_lyrics_prev_size");
          if (savedSizeStr) {
            try {
              const saved = JSON.parse(savedSizeStr);
              await appWin.setSize(new LogicalSize(saved.width || 800, saved.height || 220));
            } catch (e) {
              await appWin.setSize(new LogicalSize(800, 220));
            }
          } else {
            await appWin.setSize(new LogicalSize(800, 220));
          }
        }

        if (newMode === "standard") {
          settings.opacity = 0.88;
          isHeaderHidden = false;
        } else if (newMode === "glass") {
          settings.opacity = 0.28;
          isHeaderHidden = false;
        } else if (newMode === "ghost") {
          settings.opacity = 0.0;
          isHeaderHidden = true;
        }
      }
    } catch (e) {}

    await notifySettingsChanged();
  }

  async function cycleDisplayMode() {
    const modes: ("standard" | "glass" | "ghost" | "hud")[] = ["standard", "glass", "ghost", "hud"];
    const nextIdx = (modes.indexOf(settings.displayMode) + 1) % modes.length;
    await applyDisplayMode(modes[nextIdx]);
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
  let handleKeyDown: ((e: KeyboardEvent) => void) | null = null;

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

          if (!playback.isPlaying || Math.abs(drift) > 200) {
            // Recalage direct si pause, seek ou écart significatif (> 200ms)
            playback.positionMs = media.positionMs;
            playback.lastUpdatedMs = now;
            lastInterpolatedPositionMs = media.positionMs + settings.timeOffsetMs;
          } else if (Math.abs(drift) > 15) {
            // Dérive : correction dynamique vive (85% du drift pour coller immédiatement au chant)
            const adjusted = currentEstimated + (drift * 0.85);
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
        const isPodcastOrVideo = title.toLowerCase().includes("podcast") || artist.toLowerCase().includes("podcast") || title.toLowerCase().includes("interview");
        if (isPodcastOrVideo) {
          lyrics = [
            { startTimeMs: 0, text: `Transcription anglaise non disponible pour "${title}"` },
            { startTimeMs: 4000, text: "Ce podcast ou vidéo ne dispose pas encore de sous-titres" },
          ];
        } else {
          lyrics = [
            { startTimeMs: 0, text: `Aucune parole trouvée pour "${title}"` },
            { startTimeMs: 4000, text: "Vérifiez le titre ou lancez un autre morceau" },
          ];
        }
        currentLyricsSource = "";
      }
    } catch (err) {
      console.error("Erreur recherche paroles/transcription:", err);
      const isPodcastOrVideo = title.toLowerCase().includes("podcast") || artist.toLowerCase().includes("podcast");
      if (isPodcastOrVideo) {
        lyrics = [{ startTimeMs: 0, text: "Transcription non disponible pour cet épisode" }];
      } else {
        lyrics = [{ startTimeMs: 0, text: "Paroles ou transcription introuvables" }];
      }
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
      // Anticipation dynamique de 60ms pour compenser le buffer audio Windows/Bluetooth
      currentPos = playback.positionMs + (elapsed * playback.playbackRate) + settings.timeOffsetMs + 60;
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

    if (settings.clickThrough) {
      try {
        await invoke("set_overlay_click_through", { enable: true });
      } catch (e) {}
    }

    // 2. Démarrage de la boucle d'interpolation et du polling à 100ms pour une réactivité instantanée
    animationFrameId = requestAnimationFrame(updateInterpolation);
    pollIntervalId = window.setInterval(pollMedia, 100);
    pollMedia();

    // 3. Écouteurs d'événements Tauri
    try {
      unlistenSettings = await listen<AppSettings>("settings_changed", async (event) => {
        const prevMode = settings.displayMode;
        Object.assign(settings, event.payload);
        if (event.payload.displayMode !== prevMode) {
          await applyDisplayMode(event.payload.displayMode);
        } else if (event.payload.displayMode === "ghost") {
          isHeaderHidden = true;
        }
      });

      // Enregistrement des raccourcis globaux
      try {
        await unregisterAll();
        await register(settings.hotkey || "Ctrl+Shift+L", async (event) => {
          if (event.state === "Pressed") {
            settings.clickThrough = !settings.clickThrough;
            await invoke("set_overlay_click_through", { enable: settings.clickThrough });
            await notifySettingsChanged();
          }
        });
        await register(settings.hudHotkey || "Ctrl+Shift+H", async (event) => {
          if (event.state === "Pressed") {
            const next = settings.displayMode === "hud" ? "standard" : "hud";
            await applyDisplayMode(next);
          }
        });
      } catch (errShortcuts) {
        console.warn("Raccourcis globaux non enregistrés:", errShortcuts);
      }

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
        updateAuroraPalette(event.payload.title, event.payload.artist);
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
        updateAuroraPalette(event.payload.title, event.payload.artist);
        await loadLyricsForSong(event.payload.title, event.payload.artist);
      });
    } catch (e) {
      console.warn("Écouteurs d'événements non activés:", e);
    }

    // 4. Navigation clavier et touche Échap pour synchroniser
    handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && (e.key === "H" || e.key === "h")) {
        e.preventDefault();
        const next = settings.displayMode === "hud" ? "standard" : "hud";
        applyDisplayMode(next);
      } else if (e.ctrlKey && (e.key === "," || e.key === "p" || e.key === "P")) {
        e.preventDefault();
        handleOpenSettings();
      } else if (e.key === "Escape" && isManualScrolling) {
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
    if (handleKeyDown) window.removeEventListener("keydown", handleKeyDown);
    try {
      unregisterAll();
    } catch (e) {}
    if (unlistenDemo) unlistenDemo();
    if (unlistenManual) unlistenManual();
    if (unlistenSettings) unlistenSettings();
  });
</script>

<div
  class="overlay-container mode-{settings.displayMode} {isHeaderHidden ? 'header-hidden' : ''}"
  style="
    --bg-opacity: {settings.opacity};
    --active-color: {settings.activeColor};
    --text-color: {settings.textColor};
    --active-color-glow: {settings.activeColor}66;
    font-size: {settings.fontSize}px;
  "
  data-tauri-drag-region
>
  {#if settings.clickThrough}
    <div class="click-through-indicator" aria-hidden="true" title="Mode clics traversants actif ({settings.hotkey || 'Ctrl+Shift+L'} pour déverrouiller)">
      <span class="indicator-icon">🛡️</span>
      <span class="indicator-text">Traversant ({settings.hotkey || 'Ctrl+Shift+L'})</span>
    </div>
  {/if}

  <!-- Halo Aurora Glow Réactif (Fluid Mesh Gradient) -->
  {#if settings.auroraMode !== "off"}
    <div
      class="aurora-container {settings.auroraMode === 'eco' ? 'eco' : 'fluid'}"
      aria-hidden="true"
      style="
        --aurora-c1: {auroraColors.c1};
        --aurora-c2: {auroraColors.c2};
        --aurora-c3: {auroraColors.c3};
      "
    >
      <div class="aurora-blob blob-1"></div>
      <div class="aurora-blob blob-2"></div>
      <div class="aurora-blob blob-3"></div>
    </div>
  {/if}

  {#if settings.displayMode === "hud"}
    <!-- Mode HUD Compact (Dynamic Island Capsule) -->
    <div class="hud-capsule-layout" data-tauri-drag-region>
      <div class="hud-top-meta" data-tauri-drag-region>
        <div class="hud-source-badge">
          <span class="hud-live-dot {playback.isPlaying ? 'pulsing' : ''}"></span>
          <span class="hud-source-name">{currentSong.title} — {currentSong.artist}</span>
        </div>
        <div class="hud-actions">
          <button
            class="hud-action-btn"
            onclick={() => applyDisplayMode("standard")}
            title="Agrandir l'overlay (Ctrl+Shift+H)"
            aria-label="Agrandir"
          >
            ⤢
          </button>
          <button
            class="hud-action-btn"
            onclick={handleOpenSettings}
            title="Paramètres (Ctrl+,)"
            aria-label="Paramètres"
          >
            ⚙️
          </button>
          <button
            class="hud-action-btn"
            onclick={handleMinimize}
            title="Réduire"
            aria-label="Réduire"
          >
            —
          </button>
          <button
            class="hud-action-btn hud-close-btn"
            onclick={handleCloseApp}
            title="Quitter GhostLyrics"
            aria-label="Quitter"
          >
            ✕
          </button>
        </div>
      </div>

      <!-- Vers Actif en Karaoké Mot-à-Mot -->
      {#if currentLineIndex >= 0 && lyrics[currentLineIndex]}
        {@const activeLine = lyrics[currentLineIndex]}
        {@const hasWords = activeLine.words && activeLine.words.length > 0}
        <div class="hud-active-verse {hasWords ? 'has-karaoke' : ''}" style="color: {settings.activeColor};" data-tauri-drag-region>
          {#if hasWords}
            <span class="karaoke-words-container">
              {#each activeLine.words as word, wIdx}
                {@const status = getWordStatus(word, interpolatedPositionMs)}
                <span
                  class="karaoke-word {status.state}"
                  style="--word-progress: {status.progress}%;"
                >{word.text}{wIdx < activeLine.words.length - 1 ? ' ' : ''}</span>
              {/each}
            </span>
          {:else}
            {activeLine.text}
          {/if}
        </div>
      {:else}
        <div class="hud-active-verse" style="color: {settings.activeColor};" data-tauri-drag-region>
          <span class="hud-idle-text">🎵 En attente des paroles...</span>
        </div>
      {/if}

      <!-- Vers Suivant (Teaser atténué) -->
      {#if currentLineIndex >= 0 && currentLineIndex + 1 < lyrics.length}
        <div class="hud-next-verse" style="color: {settings.textColor};" data-tauri-drag-region>
          ↳ {lyrics[currentLineIndex + 1].text}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Mode Standard, Verre ou Fantôme -->
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
            title="Style d'affichage : Standard / Verre / Fantôme / HUD (clic pour basculer)"
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
            {@const isPodcast = currentLyricsSource.includes("Podcast")}
            {@const isVideo = currentLyricsSource.includes("YouTube")}
            <span
              class="badge {isPodcast ? 'badge-podcast' : isVideo ? 'badge-video' : isCurrentSynced ? 'badge-synced' : 'badge-paced'}"
              title={isPodcast ? `Transcription anglaise de podcast (${currentLyricsSource})` : isVideo ? `Sous-titres anglais synchronisés (${currentLyricsSource})` : isCurrentSynced ? `Synchronisé (.lrc) via ${currentLyricsSource}` : `Défilement temporel estimé via ${currentLyricsSource}`}
            >
              {#if isPodcast}
                🎙️ Podcast
              {:else if isVideo}
                📺 Vidéo EN
              {:else if isCurrentSynced}
                🟢 LRC
              {:else}
                🟡 Texte
              {/if}
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
            <!-- Bouton Mode HUD Compact -->
            <button
              class="ctrl-btn btn-hud"
              onclick={() => applyDisplayMode("hud")}
              title="Passer en mode HUD Compact / Dynamic Island (Ctrl+Shift+H)"
              aria-label="Mode HUD"
            >
              💊
            </button>

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
          onclick={() => applyDisplayMode("hud")}
          title="Mode HUD Compact (Ctrl+Shift+H)"
          aria-label="Mode HUD"
        >
          💊
        </button>
        <button
          class="mini-dock-btn"
          onclick={handleOpenSettings}
          title="Ouvrir les paramètres (Ctrl+,)"
          aria-label="Paramètres"
        >
          ⚙️
        </button>
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
        <button
          class="mini-dock-btn"
          onclick={handleMinimize}
          title="Réduire dans la zone de notification"
          aria-label="Réduire"
        >
          —
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
          {@const dist = currentLineIndex >= 0 ? Math.abs(index - currentLineIndex) : 1}
          {@const isActive = index === currentLineIndex}
          {@const hasWords = isActive && line.words && line.words.length > 0}
          <div
            bind:this={lineElements[index]}
            class="lyric-line {isActive ? 'active' : ''} {isManualScrolling ? 'interactive' : ''} dist-{Math.min(dist, 3)} {hasWords ? 'has-karaoke' : ''}"
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
              color: {isActive ? settings.activeColor : settings.textColor};
            "
          >
            {#if hasWords}
              <span class="karaoke-words-container">
                {#each line.words as word, wIdx}
                  {@const status = getWordStatus(word, interpolatedPositionMs)}
                  <span
                    class="karaoke-word {status.state}"
                    style="--word-progress: {status.progress}%;"
                  >{word.text}{wIdx < line.words.length - 1 ? ' ' : ''}</span>
                {/each}
              </span>
            {:else}
              {line.text}
            {/if}
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
  {/if}
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
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-rendering: optimizeLegibility;
    transition: background 0.25s ease, border 0.25s ease, box-shadow 0.25s ease;
  }

  .overlay-container.header-hidden {
    padding-top: 4px;
  }

  /* Badge flottant indicatif du mode Clics Traversants */
  .click-through-indicator {
    position: absolute;
    top: 6px;
    left: 10px;
    z-index: 85;
    display: flex;
    align-items: center;
    gap: 5px;
    background: rgba(15, 23, 42, 0.82);
    border: 1px solid rgba(56, 189, 248, 0.45);
    border-radius: 9999px;
    padding: 2px 9px;
    font-size: 10.5px;
    font-weight: 700;
    color: #38bdf8;
    pointer-events: none;
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.55), 0 0 10px rgba(56, 189, 248, 0.2);
    animation: fadeIn 0.2s ease;
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

  /* En mode Fantôme : l'Aurora Glow devient un halo nébuleux éthéré très diffus sous les paroles */
  .mode-ghost .aurora-container {
    opacity: 0.65;
    pointer-events: none;
  }

  .mode-ghost .aurora-blob {
    opacity: 0.6;
    filter: blur(72px);
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

  .badge-podcast {
    background: rgba(168, 85, 247, 0.25);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.4);
  }

  .badge-video {
    background: rgba(239, 68, 68, 0.22);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.4);
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
    mask-image: linear-gradient(to bottom, transparent 0%, rgba(0, 0, 0, 0.85) 10%, black 22%, black 78%, rgba(0, 0, 0, 0.85) 90%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, rgba(0, 0, 0, 0.85) 10%, black 22%, black 78%, rgba(0, 0, 0, 0.85) 90%, transparent 100%);
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
    font-weight: 600;
    line-height: 1.45;
    word-break: break-word;
    overflow-wrap: break-word;
    max-width: 94%;
    margin: 0 auto;
    position: relative;
    transition: transform 0.4s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.35s ease, color 0.3s ease;
    will-change: transform, opacity;
    /* Ombre nette, fine et soignée : lisibilité parfaite sur fond blanc ou noir sans taches noires */
    text-shadow:
      0 1px 2px rgba(0, 0, 0, 0.75),
      0 0 1px rgba(0, 0, 0, 0.85);
  }

  /* 🌊 Ligne active : Effet de vague cinétique lors du passage */
  .lyric-line.dist-0,
  .lyric-line.active {
    font-weight: 750;
    transform: scale(1.05);
    opacity: 1;
    animation: line-wave-pass 0.48s cubic-bezier(0.22, 1, 0.36, 1) forwards;
  }

  /* Balayage de vague lumineuse fluide à l'activation d'un vers */
  .lyric-line.active::after {
    content: "";
    position: absolute;
    inset: -2px -24px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      var(--active-color-glow, rgba(56, 189, 248, 0.2)) 25%,
      rgba(255, 255, 255, 0.65) 50%,
      var(--active-color-glow, rgba(56, 189, 248, 0.2)) 75%,
      transparent 100%
    );
    transform: translateX(-100%);
    animation: wave-pass-sheen 0.75s cubic-bezier(0.2, 0.8, 0.2, 1) forwards;
    pointer-events: none;
    border-radius: 9999px;
    filter: blur(4px);
  }

  @keyframes line-wave-pass {
    0% {
      transform: translateY(8px) scale(0.97);
      opacity: 0.65;
    }
    60% {
      transform: translateY(-2px) scale(1.07);
      opacity: 1;
    }
    100% {
      transform: translateY(0) scale(1.05);
      opacity: 1;
    }
  }

  @keyframes wave-pass-sheen {
    0% {
      transform: translateX(-100%);
      opacity: 0;
    }
    30% {
      opacity: 0.9;
    }
    100% {
      transform: translateX(100%);
      opacity: 0;
    }
  }

  /* Ligne active classique (sans karaoké mot-à-mot) */
  .lyric-line.active:not(.has-karaoke) {
    text-shadow:
      0 0 16px var(--active-color-glow, rgba(56, 189, 248, 0.65)),
      0 1px 3px rgba(0, 0, 0, 0.85);
  }

  /* Ligne active avec karaoké mot-à-mot : text-shadow désactivé sur le parent pour propreté totale */
  .lyric-line.active.has-karaoke {
    text-shadow: none;
  }

  /* Échelonnage doux sans aucun flou (élimine toute bavure sombre sur fond clair) */
  .lyric-line.dist-1 {
    opacity: 0.65;
    transform: scale(0.96);
  }

  .lyric-line.dist-2 {
    opacity: 0.35;
    transform: scale(0.92);
  }

  .lyric-line.dist-3 {
    opacity: 0.18;
    transform: scale(0.88);
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

  /* ========================================================================= */
  /* 🌌 1. Halo Aurora Glow Réactif (Arrière-plan dynamique GPU)                */
  /* ========================================================================= */
  .aurora-container {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
    z-index: 0;
    border-radius: inherit;
  }

  .aurora-blob {
    position: absolute;
    border-radius: 50%;
    filter: blur(52px);
    will-change: transform;
    opacity: 0.85;
    transform: translate3d(0, 0, 0);
  }

  .blob-1 {
    top: -15%;
    left: -10%;
    width: 65%;
    height: 65%;
    background: radial-gradient(circle, var(--aurora-c1), transparent 70%);
  }

  .blob-2 {
    bottom: -15%;
    right: -10%;
    width: 70%;
    height: 70%;
    background: radial-gradient(circle, var(--aurora-c2), transparent 70%);
  }

  .blob-3 {
    top: 25%;
    left: 25%;
    width: 55%;
    height: 55%;
    background: radial-gradient(circle, var(--aurora-c3), transparent 70%);
  }

  .aurora-container.fluid .blob-1 {
    animation: aurora-float-1 18s ease-in-out infinite alternate;
  }

  .aurora-container.fluid .blob-2 {
    animation: aurora-float-2 22s ease-in-out infinite alternate;
  }

  .aurora-container.fluid .blob-3 {
    animation: aurora-float-3 15s ease-in-out infinite alternate;
  }

  .aurora-container.eco .blob-1,
  .aurora-container.eco .blob-2,
  .aurora-container.eco .blob-3 {
    animation: none;
  }

  @keyframes aurora-float-1 {
    0% { transform: translate3d(0, 0, 0) scale(1); }
    50% { transform: translate3d(25px, 20px, 0) scale(1.12); }
    100% { transform: translate3d(-15px, 35px, 0) scale(0.95); }
  }

  @keyframes aurora-float-2 {
    0% { transform: translate3d(0, 0, 0) scale(1); }
    50% { transform: translate3d(-30px, -25px, 0) scale(1.15); }
    100% { transform: translate3d(20px, -15px, 0) scale(0.92); }
  }

  @keyframes aurora-float-3 {
    0% { transform: translate3d(0, 0, 0) scale(0.9); }
    50% { transform: translate3d(-20px, 25px, 0) scale(1.18); }
    100% { transform: translate3d(25px, -20px, 0) scale(1); }
  }

  /* ========================================================================= */
  /* 🎤 2. Karaoké Mot-à-Mot « Apple Music Sing »                              */
  /* ========================================================================= */
  .karaoke-words-container {
    display: inline;
    line-height: inherit;
  }

  .karaoke-word {
    display: inline;
    white-space: pre-wrap;
    position: relative;
    transition: filter 0.16s ease, color 0.16s ease;
  }

  .karaoke-word.completed {
    color: var(--active-color, #38bdf8);
    text-shadow:
      0 0 12px var(--active-color-glow, rgba(56, 189, 248, 0.6)),
      0 1px 2px rgba(0, 0, 0, 0.85);
  }

  .karaoke-word.singing {
    background: linear-gradient(
      90deg,
      var(--active-color, #38bdf8) 0%,
      var(--active-color, #38bdf8) calc(var(--word-progress, 0%) - 2%),
      #ffffff var(--word-progress, 0%),
      rgba(255, 255, 255, 0.75) calc(var(--word-progress, 0%) + 2%),
      rgba(255, 255, 255, 0.75) 100%
    );
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    filter: drop-shadow(0 0 10px var(--active-color-glow, rgba(56, 189, 248, 0.75))) drop-shadow(0 1px 2px rgba(0, 0, 0, 0.85));
  }

  .karaoke-word.upcoming {
    color: rgba(255, 255, 255, 0.72);
    opacity: 1;
    text-shadow:
      0 1px 2px rgba(0, 0, 0, 0.8),
      0 0 1px rgba(0, 0, 0, 0.9);
  }

  /* ========================================================================= */
  /* 💊 3. Mode HUD Compact (Dynamic Island Capsule)                           */
  /* ========================================================================= */
  .mode-hud {
    background: rgba(15, 15, 20, 0.92) !important;
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid rgba(255, 255, 255, 0.16) !important;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.65) !important;
    border-radius: 24px !important;
    padding: 6px 14px !important;
    justify-content: center;
  }

  .hud-capsule-layout {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    justify-content: center;
    gap: 3px;
    position: relative;
    z-index: 10;
  }

  .hud-top-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
  }

  .hud-source-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.6);
    font-weight: 500;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    max-width: 80%;
  }

  .hud-live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #64748b;
    flex-shrink: 0;
  }

  .hud-live-dot.pulsing {
    background: #38bdf8;
    box-shadow: 0 0 8px #38bdf8;
    animation: hud-dot-pulse 2s infinite ease-in-out;
  }

  @keyframes hud-dot-pulse {
    0%, 100% { opacity: 0.7; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.3); }
  }

  .hud-actions {
    display: flex;
    align-items: center;
    gap: 3px;
  }

  .hud-action-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .hud-action-btn:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.15);
  }

  .hud-close-btn:hover {
    color: #ffffff !important;
    background: #ef4444 !important;
  }

  .hud-active-verse {
    font-size: 13.5px;
    font-weight: 700;
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    mask-image: linear-gradient(to right, black 92%, transparent 100%);
    -webkit-mask-image: linear-gradient(to right, black 92%, transparent 100%);
    text-shadow: 0 0 10px rgba(56, 189, 248, 0.4), 0 1px 3px #000;
  }

  .hud-active-verse.has-karaoke {
    text-shadow: none;
  }

  .hud-next-verse {
    font-size: 10.5px;
    opacity: 0.5;
    font-style: italic;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-shadow: 0 1px 2px #000;
  }

  .hud-idle-text {
    opacity: 0.6;
    font-size: 12px;
  }

  .btn-hud:hover {
    background: rgba(56, 189, 248, 0.25);
    color: #38bdf8;
  }
</style>
