# 👻 GhostLyrics

<p align="center">
  <strong>Alternative open source, gratuite et ultra-légère à Lyric Overlay.</strong><br>
  Affichez les paroles synchronisées (style karaoké) par-dessus vos fenêtres et jeux vidéo sous Windows, sans latence et sans impacter vos performances.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6?logo=windows" alt="Platform Windows" />
  <img src="https://img.shields.io/badge/Framework-Tauri%20v2-FFC131?logo=tauri" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/Backend-Rust-black?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Frontend-Svelte%205-FF3E00?logo=svelte" alt="Svelte 5" />
  <img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License" />
</p>

---

## ✨ Fonctionnalités clés

- ⚡ **Ultra-léger (< 35 Mo de RAM) :** Conçu avec Tauri v2 et Rust pour ne pas faire chuter vos FPS en jeu.
- 🎵 **Détection universelle de la musique :** Compatible immédiatement avec **Spotify, Apple Music, YouTube Music (PWA/navigateur), Deezer, VLC**, etc., via l'API native Windows WinRT GSMTC. Aucun compte ni clé API requis !
- 👻 **Mode Fantôme (*Click-Through*) :** Verrouillez l'overlay avec un raccourci global (`Ctrl + Shift + L`) pour cliquer librement au travers pendant vos sessions de jeu ou de travail.
- 🎤 **Paroles synchronisées en direct :** Intégration directe avec la base libre **LRCLIB** et algorithme d'interpolation fluide à 60/144 Hz.
- 📂 **Fallback de paroles personnalisées :** Prise en charge des fichiers `.lrc` locaux et réglage manuel du décalage temporel (+/- ms).
- ⚙️ **Gestion en arrière-plan (System Tray) :** Contrôle discret depuis la zone de notification Windows avec une fenêtre de personnalisation complète (polices, opacités, couleurs).

---

## 🛠️ Stack Technique

- **Moteur & Système :** [Tauri v2](https://v2.tauri.app/) & [Rust](https://www.rust-lang.org/) (Crates `windows-rs`, `reqwest`, `tokio`).
- **Interface Utilisateur :** [Svelte 5](https://svelte.dev/) + [Vite](https://vitejs.dev/) + [TypeScript](https://www.typescriptlang.org/).
- **Design & Styles :** [Tailwind CSS](https://tailwindcss.com/) pour des effets de flou et de translucidité (`backdrop-blur`).
- **Base de Paroles :** [LRCLIB](https://lrclib.net/) (REST API publique).

---

## 🚀 Démarrage en développement

### Prérequis
- **Rust & Cargo :** `rustup` (édition 2021+, Rust 1.80+)
- **Node.js :** v18+ (Node 20/24 recommandé)
- **pnpm :** `corepack enable` ou `npm install -g pnpm`
- **Windows C++ Build Tools :** inclus avec Visual Studio Community ou Build Tools for Visual Studio.

### Installation

```bash
# 1. Cloner le dépôt
git clone https://github.com/angelolockdev/GhostLyrics.git
cd GhostLyrics

# 2. Installer les dépendances frontend
pnpm install

# 3. Lancer l'application en mode développement
pnpm tauri dev
```

---

## 📂 Architecture du Projet

```text
GhostLyrics/
├── docs/
│   └── DESIGN.md            # Spécifications d'architecture détaillées
├── src-tauri/               # Backend Rust & intégration Windows
│   ├── Cargo.toml           # Dépendances Rust (windows-rs, tauri v2, reqwest)
│   ├── tauri.conf.json      # Configuration des fenêtres et permissions Tauri
│   └── src/
│       ├── main.rs          # Point d'entrée principal
│       ├── lib.rs           # Setup Tauri & enregistrement des commandes
│       ├── media/           # Détection WinRT GSMTC
│       ├── lyrics/          # Client LRCLIB, parseur .lrc et cache local
│       └── window/          # Gestionnaire de styles de fenêtre & click-through
├── src/                     # Frontend Svelte 5
│   ├── App.svelte           # Routeur / sélecteur de vue (Overlay vs Settings)
│   ├── lib/
│   │   ├── components/      # Composants Overlay & Paramètres
│   │   ├── stores/          # Stores réactifs d'état audio et paroles
│   │   └── types.ts         # Définitions TypeScript
│   └── main.ts              # Montage Svelte
├── package.json
└── vite.config.ts
```

---

## 📦 Releases Automatiques Windows

Le projet inclut un workflow GitHub Actions automatisé ([`.github/workflows/release-windows.yml`](.github/workflows/release-windows.yml)) qui compile l'application, génère les installeurs Windows (`.exe` NSIS et `.msi`) et publie la release GitHub.

### Créer une nouvelle version

```bash
# 1. Créer un tag de version (ex: v0.1.0)
git tag v0.1.0

# 2. Pousser le tag pour déclencher le build et la release
git push origin v0.1.0
```

Vous pouvez également déclencher manuellement la création d'une release depuis l'onglet **Actions** de votre dépôt GitHub via le bouton **Run workflow**.

---

## 📄 Licence

Ce projet est distribué sous licence libre **MIT**. Consultez le fichier [LICENSE](LICENSE) pour plus d'informations.
