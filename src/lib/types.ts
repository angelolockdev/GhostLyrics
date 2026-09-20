export interface LyricWord {
  text: string;
  startTimeMs: number;
  endTimeMs: number;
}

export interface LyricLine {
  startTimeMs: number;
  endTimeMs?: number;
  text: string;
  words?: LyricWord[];
}

export interface SongMetadata {
  title: string;
  artist: string;
  album?: string;
  durationMs: number;
  sourceApp?: string;
}

export interface CurrentMediaState {
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  positionMs: number;
  isPlaying: boolean;
  playbackRate: number;
  lastUpdatedMs: number;
  sourceApp: string;
}

export interface PlaybackState {
  isPlaying: boolean;
  positionMs: number;
  lastUpdatedMs: number;
  playbackRate: number;
}

export interface LyricsData {
  id?: number;
  trackName: string;
  artistName: string;
  albumName?: string;
  duration?: number;
  instrumental: boolean;
  plainLyrics?: string;
  syncedLyrics?: string;
  lines: LyricLine[];
  source?: string;
  isSynced?: boolean;
}

export type DisplayMode = "standard" | "glass" | "ghost" | "hud";
export type AuroraMode = "fluid" | "eco" | "off";

export interface AppSettings {
  displayMode: DisplayMode;
  fontSize: number;
  opacity: number;
  textColor: string;
  activeColor: string;
  backgroundColor: string;
  timeOffsetMs: number;
  hotkey: string;
  hudHotkey?: string;
  clickThrough: boolean;
  autoCheckUpdates?: boolean;
  auroraMode?: AuroraMode;
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  hasUpdate: boolean;
  releaseName: string;
  releaseNotes: string;
  downloadUrl: string;
  publishedAt: string;
}

export interface DownloadProgress {
  downloadedBytes: number;
  totalBytes: number;
  percent: number;
}
