# GhostLyrics — Spécification de Conception : Moteur Karaoké « Apple Music Sing », Aurora Glow & HUD Compact

> **Statut :** Validé après session de Brainstorming  
> **Date :** 2026-09-20  
> **Version cible :** GhostLyrics v0.2.0  
> **Auteurs :** Équipe GhostLyrics & Pair Programmer  

---

## 1. Résumé de Compréhension (Understanding Summary)

* **Ce qui est conçu :**
  1. **Moteur Karaoké « Apple Music Sing » (Syllabes & Mot-à-Mot) :** Balayage lumineux fluide mot par mot. Prise en charge native des timestamps explicites (Enhanced LRC `<mm:ss.xx>`) et estimateur heuristique syllabique pour transformer 100% des fichiers LRC standards en paroles progressives ultra-fluides.
  2. **Halo Aurora Glow Réactif (Fluid Mesh Gradient) :** Arrière-plan néon liquide flouté dérivé de la palette chromatique de la pochette de l'album en cours, avec préréglages de performance (Fluide 60 FPS, Éco statique, Désactivé) et mise en pause automatique en jeu plein écran exclusif.
  3. **4ème Mode d'Affichage « HUD Compact / Dynamic Island » :** Capsule horizontale flottante discrète (440×58 px) affichant le vers actif et le vers suivant, basculable via raccourci global (`Ctrl + Shift + H`) ou bouton d'action rapide.
* **Pourquoi cela existe :** Propulser GhostLyrics au niveau des expériences visuelles les plus raffinées de l'industrie (Apple Music Sing, Spotify Carplay) tout en maintenant ses fondations : logiciel libre, gratuit, zéro compte requis et légèreté absolue.
* **Pour qui :** Utilisateurs Windows 10/11 et joueurs sur PC recherchant un karaoké immersif ou un affichage discret sans aucun impact sur leurs performances de jeu (FPS).
* **Contraintes clés :**
  * Empreinte mémoire résiduelle cible : **< 35–40 Mo de RAM**.
  * Utilisation CPU : **< 0.8%** en défilement et balayage continu.
  * Utilisation GPU : **< 1.5%** pour le halo lumineux animé.
  * Maintien inconditionnel du mode Fantôme (*Click-Through* Win32 `WS_EX_TRANSPARENT`).

---

## 2. Hypothèses Retenues (Assumptions)

1. **Extraction de palette ultra-légère :** L'extraction de couleur se fait dès réception de la vignette média GSMTC via un micro-échantillonnage 16×16 pixels sans impact CPU.
2. **Raccourci global HUD :** `Ctrl + Shift + H` est le raccourci par défaut pour permuter entre l'overlay complet et la pilule compacte (configurable dans les paramètres).
3. **Pondération syllabique robuste :** L'algorithme d'estimation pour les lignes sans tags de mots alloue la durée de la ligne selon la présence des voyelles et diphtongues avec une marge respiratoire de 150 ms en fin de vers.

---

## 3. Architecture Technique Détaillée

### 3.1. Structure des Données & Moteur Karaoké Mot-à-Mot

#### Modèle de données (Rust & TypeScript)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LyricWord {
    pub text: String,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_time_ms: i64,
    pub end_time_ms: Option<i64>,
    pub text: String,
    pub words: Vec<LyricWord>, // Découpage temporel mot/syllabe
}
```

#### Algorithme de synchronisation hybride
1. **Tags explicites (Enhanced LRC) :** Extraction directe des timestamps `<mm:ss.xx>` dans `parser.rs`.
2. **Estimation syllabique heuristique (si `words` est vide) :**
   $$\text{Poids du mot } w = (\text{voyelles}(w) \times 1.5) + (\text{consonnes}(w) \times 0.5) + \text{bonus\_ponctuation}(w)$$
   $$\text{Durée}(w) = (\text{Durée totale du vers} - 150\text{ms}) \times \frac{\text{Poids}(w)}{\sum \text{Poids}}$$
3. **Rendu dynamique Svelte 5 :**  
   Mise à jour dans `requestAnimationFrame` du ratio d'avancement par mot :
   $$\text{progression} = \text{clamp}\left(\frac{\text{position\_actuelle} - \text{start\_ms}}{\text{end\_ms} - \text{start\_ms}}, 0, 1\right) \times 100\%$$
   Rendu CSS via `background: linear-gradient(90deg, var(--active-color) var(--prog), var(--text-color) var(--prog)); background-clip: text;`.

---

### 3.2. Halo Aurora Glow Réactif (Fluid Mesh Gradient)

* **Extraction chromatique :** Échantillonnage HSL miniature extrayant 3 teintes complémentaires ($C_1$ vibrante, $C_2$ accent, $C_3$ fond équilibré).
* **Rendu GPU :** 3 calques vectoriels `radial-gradient` indépendants superposés avec `filter: blur(48px)` et animés via `transform: translate3d(...)` (0 recalcul de layout).
* **Gestion d'énergie (3 préréglages) :**
  * *Fluide (60/144 FPS) :* Animation fluide continue.
  * *Éco (Statique) :* Teinte d'ambiance floutée fixe par morceau (0% GPU).
  * *Désactivé :* Fond translucide standard sans halo.
* **Mise en veille automatique :** Coupure immédiate du calcul CSS si l'overlay est masqué ou en mode Fantôme pur.

---

### 3.3. Mode HUD Compact (Dynamic Island)

* **Format de capsule :** 440×58 px, angles arrondis `border-radius: 29px`.
* **Disposition :** Ligne supérieure avec vers actif chanté en mot-à-mot, ligne inférieure avec aperçu estompé du vers à venir.
* **Double mémorisation d'ancrage :** Sauvegarde séparée dans `localStorage` des positions d'écran pour le mode standard et le mode compact.
* **Permutation sans latence :** Déclenchable via raccourci global `Ctrl + Shift + H`, bouton d'en-tête ou System Tray.
* **Click-through :** État Win32 `WS_EX_TRANSPARENT` préservé pour ne jamais bloquer les clics en jeu.

---

## 4. Gestion des Cas Limites (Edge Cases)

* **Ponts instrumentaux (> 8s) :** Affichage d'une pulsation d'ondes d'attente s'estompant 2 secondes avant la reprise du chant.
* **Rythme effréné (Rap) :** Plancher minimal garanti de 90 ms par mot pour éliminer tout scintillement.
* **Pochette indisponible :** Bascule automatique sur une palette par défaut cyber-indigo (`#0ea5e9` / `#6366f1`).

---

## 5. Journal des Décisions (Decision Log)

* **DEC-001 (Périmètre prioritaire) :** Choix du pack "Expérience Visuelle & Karaoké Apple Music" face aux options Multilingue, Streamer et Multi-fournisseurs.
* **DEC-002 (Moteur hybride) :** Combinaison des balises mot-à-mot réelles avec une interpolation heuristique syllabique pour couvrir 100% de la bibliothèque musicale.
* **DEC-003 (Accélération GPU & Éco) :** Rendu CSS GPU hardware-accelerated sans shader WebGL lourd pour préserver le framerate en jeu et la mémoire (< 35 Mo).
* **DEC-004 (Format HUD) :** 4ème mode d'affichage pilule avec mémoires de position distinctes et raccourci global dédié `Ctrl + Shift + H`.
* **DEC-005 (Architecture) :** Validation unanime de l'Option 1 (Architecture Hybride Découplée Svelte 5 + Rust).
