export interface LyricLine {
  startTimeMs: number;
  endTimeMs?: number;
  text: string;
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
}

export interface AppSettings {
  fontSize: number;
  opacity: number;
  textColor: string;
  activeColor: string;
  backgroundColor: string;
  timeOffsetMs: number;
  hotkey: string;
  clickThrough: boolean;
}
