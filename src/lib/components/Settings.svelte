<script lang="ts">
  import type { AppSettings } from "../types";

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

  function adjustOffset(amount: number) {
    settings.timeOffsetMs += amount;
  }

  function resetOffset() {
    settings.timeOffsetMs = 0;
  }
</script>

<div class="settings-page">
  <header class="settings-header">
    <h2>⚙️ Paramètres GhostLyrics</h2>
    <p>Personnalisez l'affichage, les raccourcis et la synchronisation de l'overlay.</p>
  </header>

  <div class="settings-grid">
    <!-- Section Affichage & Styles -->
    <div class="card">
      <h3>Apparence de l'overlay</h3>

      <div class="field">
        <label for="fontSize">Taille de la police : <strong>{settings.fontSize}px</strong></label>
        <input id="fontSize" type="range" min="14" max="36" bind:value={settings.fontSize} />
      </div>

      <div class="field">
        <label for="opacity">Opacité de l'arrière-plan : <strong>{Math.round(settings.opacity * 100)}%</strong></label>
        <input id="opacity" type="range" min="0.1" max="1" step="0.05" bind:value={settings.opacity} />
      </div>

      <div class="field-row">
        <div class="field">
          <label for="activeColor">Couleur active</label>
          <input id="activeColor" type="color" bind:value={settings.activeColor} />
        </div>
        <div class="field">
          <label for="bgColor">Fond</label>
          <input id="bgColor" type="color" value="#0f0f14" />
        </div>
      </div>
    </div>

    <!-- Section Synchronisation & Offset -->
    <div class="card">
      <h3>Synchronisation temporelle</h3>
      <p class="description">Ajustez le décalage si les paroles sont légèrement en avance ou en retard.</p>

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
      <h3>Mode Fantôme (Click-Through)</h3>
      <p class="description">Permet à vos clics de souris de traverser l'overlay pour ne pas perturber vos jeux.</p>

      <div class="field">
        <label for="hotkey">Raccourci global :</label>
        <input id="hotkey" type="text" bind:value={settings.hotkey} readonly />
        <small>Appuyez sur ce raccourci n'importe quand pour verrouiller/déverrouiller l'overlay.</small>
      </div>
    </div>

    <!-- Section Fichiers Locaux -->
    <div class="card">
      <h3>Paroles locales (.lrc)</h3>
      <p class="description">Si une chanson n'est pas sur LRCLIB, vous pouvez charger manuellement un fichier .lrc.</p>
      <button class="primary-btn">Importer un fichier .lrc</button>
    </div>
  </div>
</div>

<style>
  .settings-page {
    padding: 24px;
    background: #0f172a;
    color: #f8fafc;
    min-height: 100vh;
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

  small {
    color: #64748b;
    font-size: 0.75rem;
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
  }

  .primary-btn:hover {
    background: #0369a1;
  }
</style>
