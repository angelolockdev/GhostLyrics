<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { emit, listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import type { AppSettings, UpdateInfo, DownloadProgress } from "../types";

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
    autoCheckUpdates: true,
  });

  let demoStatus = $state<string>("");
  let manualTitle = $state<string>("Bohemian Rhapsody");
  let manualArtist = $state<string>("Queen");
  let manualStatus = $state<string>("");

  // État des Mises à jour
  let updateInfo = $state<UpdateInfo | null>(null);
  let isCheckingUpdate = $state<boolean>(false);
  let updateStatusMessage = $state<string>("");
  let isDownloadingUpdate = $state<boolean>(false);
  let downloadPercent = $state<number>(0);
  let downloadDetails = $state<string>("");
  let unlistenProgress: (() => void) | null = null;
  let unlistenRestart: (() => void) | null = null;

  onMount(async () => {
    const saved = localStorage.getItem("ghost_lyrics_settings");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        Object.assign(settings, parsed);
      } catch (e) {}
    }

    try {
      unlistenProgress = await listen<DownloadProgress>("update_download_progress", (event) => {
        downloadPercent = event.payload.percent;
        const mbDl = (event.payload.downloadedBytes / (1024 * 1024)).toFixed(1);
        const mbTot = event.payload.totalBytes > 0 ? (event.payload.totalBytes / (1024 * 1024)).toFixed(1) : "?";
        downloadDetails = `${mbDl} Mo / ${mbTot} Mo (${event.payload.percent}%)`;
      });

      unlistenRestart = await listen("update_ready_to_restart", () => {
        downloadDetails = "Téléchargement terminé ! Lancement de l'installeur...";
      });

      // Si activé, vérifier en arrière-plan au chargement
      if (settings.autoCheckUpdates !== false) {
        checkForUpdates(false);
      }
    } catch (e) {
      console.warn("Écouteurs de mise à jour non disponibles:", e);
    }
  });

  onDestroy(() => {
    if (unlistenProgress) unlistenProgress();
    if (unlistenRestart) unlistenRestart();
  });

  async function checkForUpdates(manual: boolean = true) {
    isCheckingUpdate = true;
    if (manual) updateStatusMessage = "Recherche en cours sur GitHub Releases...";
    try {
      const res = await invoke<UpdateInfo>("check_for_updates");
      updateInfo = res;
      if (res.hasUpdate) {
        updateStatusMessage = `✨ Nouvelle version disponible : v${res.latestVersion} !`;
      } else {
        if (manual) updateStatusMessage = `✅ Vous disposez de la version la plus récente (v${res.currentVersion}).`;
      }
    } catch (e) {
      if (manual) updateStatusMessage = `❌ Impossible de contacter GitHub : ${String(e)}`;
    } finally {
      isCheckingUpdate = false;
    }
  }

  async function startUpdate() {
    if (!updateInfo || !updateInfo.downloadUrl) return;
    isDownloadingUpdate = true;
    downloadPercent = 0;
    downloadDetails = "Initialisation du téléchargement...";
    try {
      await invoke("download_and_install_update", { downloadUrl: updateInfo.downloadUrl });
    } catch (e) {
      isDownloadingUpdate = false;
      updateStatusMessage = `❌ Erreur lors de l'installation : ${String(e)}`;
    }
  }

  async function notifySettingsChanged() {
    try {
      const snap = $state.snapshot(settings);
      localStorage.setItem("ghost_lyrics_settings", JSON.stringify(snap));
      await emit("settings_changed", snap);
    } catch (e) {
      console.error("Erreur sync settings:", e);
    }
  }

  function setDisplayMode(mode: "standard" | "glass" | "ghost") {
    settings.displayMode = mode;
    if (mode === "standard") {
      settings.opacity = 0.88;
    } else if (mode === "glass") {
      settings.opacity = 0.28;
    } else if (mode === "ghost") {
      settings.opacity = 0.0;
    }
    notifySettingsChanged();
  }

  function adjustOffset(amount: number) {
    settings.timeOffsetMs += amount;
    notifySettingsChanged();
  }

  function resetOffset() {
    settings.timeOffsetMs = 0;
    notifySettingsChanged();
  }

  async function launchDemo() {
    demoStatus = "Lancement de la démo...";
    try {
      await emit("play_demo_song", {
        title: "Bohemian Rhapsody",
        artist: "Queen",
        durationMs: 354000,
      });
      demoStatus = "✅ Morceau démo envoyé sur l'overlay !";
      setTimeout(() => { demoStatus = ""; }, 4000);
    } catch (e) {
      demoStatus = "❌ Erreur : " + String(e);
    }
  }

  async function searchManualSong() {
    if (!manualTitle.trim()) return;
    manualStatus = "Recherche en cours...";
    try {
      await emit("play_manual_song", {
        title: manualTitle.trim(),
        artist: manualArtist.trim(),
      });
      manualStatus = "✅ Morceau envoyé à l'overlay !";
      setTimeout(() => { manualStatus = ""; }, 4000);
    } catch (e) {
      manualStatus = "❌ Erreur : " + String(e);
    }
  }
</script>

<div class="settings-page">
  <header class="settings-header">
    <h2>⚙️ Paramètres GhostLyrics</h2>
    <p>Personnalisez l'affichage, les raccourcis et testez la synchronisation des paroles.</p>
  </header>

  <div class="settings-grid">
    <!-- Section Modes d'Affichage -->
    <div class="card card-modes">
      <h3>🎭 Modes d'Affichage</h3>
      <p class="description">
        Choisissez le niveau d'incrustation sur votre écran. Le texte reste toujours 100% net et contrasté.
      </p>

      <div class="mode-cards">
        <button
          type="button"
          class="mode-card {settings.displayMode === 'standard' ? 'mode-active' : ''}"
          onclick={() => setDisplayMode('standard')}
        >
          <span class="mode-icon">🎴</span>
          <div class="mode-info">
            <strong>Standard</strong>
            <small>Fond sombre dépoli classique pour un confort de lecture optimal.</small>
          </div>
        </button>

        <button
          type="button"
          class="mode-card {settings.displayMode === 'glass' ? 'mode-active' : ''}"
          onclick={() => setDisplayMode('glass')}
        >
          <span class="mode-icon">🪟</span>
          <div class="mode-info">
            <strong>Verre Discret</strong>
            <small>Translucide et léger, laisse entrevoir vos fenêtres sans gêner.</small>
          </div>
        </button>

        <button
          type="button"
          class="mode-card {settings.displayMode === 'ghost' ? 'mode-active' : ''}"
          onclick={() => setDisplayMode('ghost')}
        >
          <span class="mode-icon">👻</span>
          <div class="mode-info">
            <strong>Fantôme Pur</strong>
            <small>Zéro boîte, zéro fond. Seules les paroles flottent sur votre écran.</small>
          </div>
        </button>
      </div>
    </div>

    <!-- Section Test & Diagnostic -->
    <div class="card card-highlight">
      <h3>🧪 Test & Démonstration immédiate</h3>
      <p class="description">
        Vérifiez instantanément le fonctionnement de l'overlay et de la recherche LRCLIB sans attendre Spotify.
      </p>
      <div class="demo-actions">
        <button class="primary-btn" onclick={launchDemo}>
          🎵 Lancer un morceau test (Queen - Bohemian Rhapsody)
        </button>
        {#if demoStatus}
          <span class="status-msg">{demoStatus}</span>
        {/if}
      </div>
    </div>

    <!-- Section Recherche Manuelle -->
    <div class="card">
      <h3>🔍 Recherche manuelle de paroles</h3>
      <p class="description">
        Affichez les paroles d'une chanson spécifique même si elle n'est pas détectée automatiquement.
      </p>
      <div class="manual-form">
        <div class="field-row">
          <div class="field" style="flex: 2;">
            <label for="mTitle">Titre de la chanson</label>
            <input id="mTitle" type="text" bind:value={manualTitle} placeholder="Ex: Bohemian Rhapsody" />
          </div>
          <div class="field" style="flex: 1;">
            <label for="mArtist">Artiste</label>
            <input id="mArtist" type="text" bind:value={manualArtist} placeholder="Ex: Queen" />
          </div>
        </div>
        <button class="secondary-btn" onclick={searchManualSong}>
          Afficher sur l'overlay
        </button>
        {#if manualStatus}
          <span class="status-msg">{manualStatus}</span>
        {/if}
      </div>
    </div>

    <!-- Section Affichage & Styles -->
    <div class="card">
      <h3>Ajustements fins & Styles</h3>

      <div class="field">
        <label for="fontSize">Taille de la police : <strong>{settings.fontSize}px</strong></label>
        <input
          id="fontSize"
          type="range"
          min="14"
          max="36"
          bind:value={settings.fontSize}
          oninput={notifySettingsChanged}
        />
      </div>

      <div class="field">
        <div class="field-header">
          <label for="opacity">Opacité de l'arrière-plan : <strong>{Math.round(settings.opacity * 100)}%</strong></label>
          <span class="pill-guarantee">Texte 100% lisible</span>
        </div>
        <input
          id="opacity"
          type="range"
          min="0"
          max="1"
          step="0.05"
          bind:value={settings.opacity}
          oninput={notifySettingsChanged}
        />
        <small class="field-hint">Ce curseur n'atténue que la boîte de fond. Les paroles restent nettes avec ombres portées.</small>
      </div>

      <div class="field-row">
        <div class="field">
          <label for="activeColor">Couleur active</label>
          <input
            id="activeColor"
            type="color"
            bind:value={settings.activeColor}
            oninput={notifySettingsChanged}
          />
        </div>
        <div class="field">
          <label for="bgColor">Fond</label>
          <input
            id="bgColor"
            type="color"
            value="#0f0f14"
            oninput={(e) => {
              settings.backgroundColor = `rgba(15, 15, 20, ${settings.opacity})`;
              notifySettingsChanged();
            }}
          />
        </div>
      </div>
    </div>

    <!-- Section Synchronisation & Offset -->
    <div class="card">
      <h3>Synchronisation temporelle</h3>
      <p class="description">Ajustez le décalage si les paroles sont légèrement en avance ou en retard. Vos changements sont immédiatement appliqués sur l'overlay.</p>

      <div class="offset-controls">
        <div class="offset-display">
          <span>Décalage actuel :</span>
          <strong class="{settings.timeOffsetMs > 0 ? 'text-green' : settings.timeOffsetMs < 0 ? 'text-red' : ''}">
            {settings.timeOffsetMs > 0 ? `+${settings.timeOffsetMs}` : settings.timeOffsetMs} ms
          </strong>
        </div>

        <div class="button-group">
          <button onclick={() => adjustOffset(-200)}>-200 ms</button>
          <button onclick={() => adjustOffset(-50)}>-50 ms</button>
          <button onclick={resetOffset}>Réinitialiser</button>
          <button onclick={() => adjustOffset(+50)}>+50 ms</button>
          <button onclick={() => adjustOffset(+200)}>+200 ms</button>
        </div>
      </div>
    </div>

    <!-- Section Raccourcis & Mode Fantôme -->
    <div class="card">
      <h3>Mode Fantôme (Click-Through) & Zone de notification</h3>
      <p class="description">
        L'overlay se contrôle facilement depuis l'en-tête et depuis la barre des tâches Windows :
      </p>

      <div class="info-list">
        <div class="info-item">
          <strong>🖱️ Contrôles d'en-tête :</strong>
          <span>Utilisez <code>⚙️</code> pour les paramètres, <code>—</code> pour réduire dans la barre d'icônes cachées, et <code>✕</code> pour quitter.</span>
        </div>
        <div class="info-item">
          <strong>📥 Barre d'icônes cachées (System Tray) :</strong>
          <span>GhostLyrics se loge près de l'horloge Windows. Cliquez dessus pour afficher ou masquer l'overlay à tout instant.</span>
        </div>
        <div class="info-item">
          <strong>👻 Raccourci global :</strong>
          <span><code>Ctrl + Shift + L</code> verrouille l'overlay en mode transparent aux clics.</span>
        </div>
      </div>
    </div>

    <!-- Section Mises à jour du logiciel -->
    <div class="card card-update">
      <div class="card-title-row">
        <h3>🔄 Mises à jour du logiciel</h3>
        <span class="version-pill">v{updateInfo?.currentVersion || "0.1.4"}</span>
      </div>
      <p class="description">
        GhostLyrics vérifie directement les versions publiées sur GitHub Releases et installe automatiquement les nouvelles fonctionnalités.
      </p>

      <div class="update-controls">
        <div class="update-action-row">
          <button
            class="btn-primary"
            onclick={() => checkForUpdates(true)}
            disabled={isCheckingUpdate || isDownloadingUpdate}
          >
            {#if isCheckingUpdate}
              ⏳ Recherche en cours...
            {:else}
              🔍 Rechercher une mise à jour
            {/if}
          </button>

          {#if updateStatusMessage}
            <span class="update-status-msg">{updateStatusMessage}</span>
          {/if}
        </div>

        {#if updateInfo && updateInfo.hasUpdate}
          <div class="update-banner">
            <div class="update-banner-header">
              <span class="update-tag">🎉 Version {updateInfo.latestVersion} disponible</span>
              {#if updateInfo.publishedAt}
                <span class="update-date">{new Date(updateInfo.publishedAt).toLocaleDateString('fr-FR')}</span>
              {/if}
            </div>

            {#if updateInfo.releaseNotes}
              <div class="release-notes-box">
                <strong>Notes de version :</strong>
                <pre class="release-notes-text">{updateInfo.releaseNotes}</pre>
              </div>
            {/if}

            <div class="update-buttons-row">
              <button
                class="btn-install-update"
                onclick={startUpdate}
                disabled={isDownloadingUpdate}
              >
                {#if isDownloadingUpdate}
                  ⏳ Téléchargement en cours...
                {:else}
                  ⚡ Mettre à jour automatiquement
                {/if}
              </button>

              <a
                href="https://github.com/angelolockdev/GhostLyrics/releases/latest"
                target="_blank"
                rel="noreferrer"
                class="btn-github-link"
              >
                🌐 Voir la release sur GitHub
              </a>
            </div>

            {#if isDownloadingUpdate}
              <div class="progress-section">
                <div class="progress-bar-bg">
                  <div class="progress-bar-fill" style="width: {downloadPercent}%;"></div>
                </div>
                <div class="progress-labels">
                  <span>{downloadDetails}</span>
                  <span>{downloadPercent}%</span>
                </div>
              </div>
            {/if}
          </div>
        {/if}

        <div class="auto-check-toggle">
          <label class="checkbox-label">
            <input
              type="checkbox"
              bind:checked={settings.autoCheckUpdates}
              onchange={notifySettingsChanged}
            />
            <span>Vérifier automatiquement les mises à jour au démarrage</span>
          </label>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .settings-page {
    height: 100vh;
    max-height: 100vh;
    overflow-y: auto;
    overflow-x: hidden;
    user-select: text;
    -webkit-user-select: text;
    padding: 24px;
    background: #0f172a;
    color: #f8fafc;
    box-sizing: border-box;
    font-family: system-ui, -apple-system, sans-serif;
  }

  .settings-header {
    margin-bottom: 24px;
  }

  .settings-header h2 {
    margin: 0 0 6px 0;
    font-size: 1.5rem;
    font-weight: 700;
  }

  .settings-header p {
    margin: 0;
    color: #94a3b8;
    font-size: 0.9rem;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 16px 20px;
  }

  .card h3 {
    margin: 0 0 8px 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .description {
    margin: 0 0 14px 0;
    font-size: 0.85rem;
    color: #94a3b8;
  }

  .field {
    margin-bottom: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field label {
    font-size: 0.85rem;
    color: #cbd5e1;
  }

  .field-row {
    display: flex;
    gap: 16px;
  }

  input[type="range"] {
    width: 100%;
    accent-color: #38bdf8;
  }

  input[type="text"] {
    background: #0f172a;
    border: 1px solid #475569;
    border-radius: 6px;
    padding: 8px 12px;
    color: white;
    font-size: 0.9rem;
  }


  .offset-controls {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .offset-display {
    font-size: 0.9rem;
    color: #cbd5e1;
  }

  .offset-display strong {
    font-size: 1.1rem;
    margin-left: 8px;
  }

  .text-green {
    color: #4ade80;
  }

  .text-red {
    color: #f87171;
  }

  .button-group {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  button {
    background: #334155;
    color: #f8fafc;
    border: none;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.2s ease;
  }

  button:hover {
    background: #475569;
  }

  .primary-btn {
    background: #0284c7;
    font-weight: 600;
  }

  .primary-btn:hover {
    background: #0369a1;
  }

  .secondary-btn {
    background: #475569;
    font-weight: 600;
    align-self: flex-start;
  }

  .secondary-btn:hover {
    background: #64748b;
  }

  .card-highlight {
    border-color: #0284c7;
    background: linear-gradient(180deg, #1e293b 0%, #172554 100%);
  }

  .demo-actions, .manual-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .status-msg {
    font-size: 0.85rem;
    font-weight: 600;
    color: #38bdf8;
  }

  .mode-cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .mode-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 14px;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 8px;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .mode-card:hover {
    border-color: #475569;
    background: #1e293b;
  }

  .mode-card.mode-active {
    border-color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    box-shadow: 0 0 12px rgba(56, 189, 248, 0.25);
  }

  .mode-icon {
    font-size: 1.5rem;
  }

  .mode-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .mode-info strong {
    font-size: 0.9rem;
    color: #f8fafc;
  }

  .mode-info small {
    font-size: 0.75rem;
    color: #94a3b8;
    line-height: 1.35;
  }

  .field-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .pill-guarantee {
    font-size: 0.7rem;
    padding: 2px 8px;
    border-radius: 9999px;
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.3);
    font-weight: 600;
  }

  .field-hint {
    font-size: 0.75rem;
    color: #64748b;
  }

  .info-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .info-item {
    font-size: 0.85rem;
    line-height: 1.5;
    color: #cbd5e1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  code {
    background: #0f172a;
    padding: 2px 6px;
    border-radius: 4px;
    color: #38bdf8;
    font-family: monospace;
    font-size: 0.9em;
  }

  /* Section Mises à jour */
  .card-update {
    border-color: rgba(56, 189, 248, 0.25);
    background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%);
  }

  .card-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .version-pill {
    font-size: 0.75rem;
    font-weight: 700;
    padding: 3px 10px;
    border-radius: 9999px;
    background: rgba(56, 189, 248, 0.18);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.35);
  }

  .update-controls {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-top: 14px;
  }

  .update-action-row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }

  .btn-primary {
    background: #0284c7;
    color: #ffffff;
    border: none;
    padding: 9px 16px;
    border-radius: 8px;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-primary:hover:not(:disabled) {
    background: #0369a1;
    transform: translateY(-1px);
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .update-status-msg {
    font-size: 0.85rem;
    color: #94a3b8;
    font-weight: 500;
  }

  .update-banner {
    background: rgba(30, 41, 59, 0.7);
    border: 1px solid rgba(56, 189, 248, 0.3);
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .update-banner-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .update-tag {
    font-size: 0.95rem;
    font-weight: 700;
    color: #38bdf8;
  }

  .update-date {
    font-size: 0.8rem;
    color: #64748b;
  }

  .release-notes-box {
    background: rgba(15, 23, 42, 0.8);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 10px 12px;
    max-height: 120px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .release-notes-box strong {
    font-size: 0.8rem;
    color: #cbd5e1;
  }

  .release-notes-text {
    margin: 0;
    font-size: 0.78rem;
    color: #94a3b8;
    font-family: inherit;
    white-space: pre-wrap;
    line-height: 1.4;
  }

  .update-buttons-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .btn-install-update {
    background: #16a34a;
    color: #ffffff;
    border: none;
    padding: 9px 18px;
    border-radius: 8px;
    font-size: 0.88rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-install-update:hover:not(:disabled) {
    background: #15803d;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(22, 163, 74, 0.35);
  }

  .btn-install-update:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-github-link {
    background: rgba(255, 255, 255, 0.08);
    color: #cbd5e1;
    text-decoration: none;
    padding: 9px 14px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    border: 1px solid rgba(255, 255, 255, 0.1);
    transition: all 0.2s ease;
  }

  .btn-github-link:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #ffffff;
  }

  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }

  .progress-bar-bg {
    width: 100%;
    height: 8px;
    background: rgba(15, 23, 42, 0.9);
    border-radius: 9999px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #38bdf8, #22c55e);
    border-radius: 9999px;
    transition: width 0.2s ease;
  }

  .progress-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: #94a3b8;
  }

  .auto-check-toggle {
    margin-top: 6px;
    padding-top: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .checkbox-label {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 0.82rem;
    color: #cbd5e1;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    cursor: pointer;
    accent-color: #0284c7;
  }
</style>
