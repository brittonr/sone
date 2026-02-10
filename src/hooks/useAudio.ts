import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

// Helper to convert Tidal cover UUID to image URL
export function getTidalImageUrl(
  coverUuid: string | undefined,
  size: number = 320
): string {
  if (!coverUuid) return "";
  // Tidal cover UUIDs need to be converted: uuid with dashes -> path with slashes
  const path = coverUuid.replace(/-/g, "/");
  // Use standard Tidal sizes: 160, 320, 640, 1280
  // If an invalid size is requested, snap to the nearest supported size
  let validSize = 320;
  if (size <= 160) validSize = 160;
  else if (size <= 320) validSize = 320;
  else if (size <= 640) validSize = 640;
  else validSize = 1280;

  return `https://resources.tidal.com/images/${path}/${validSize}x${validSize}.jpg`;
}

export interface Track {
  id: number;
  title: string;
  artist?: { id: number; name: string };
  album?: { id: number; title: string; cover?: string };
  duration: number;
  audioQuality?: string;
  trackNumber?: number;
}

export interface AlbumDetail {
  id: number;
  title: string;
  cover?: string;
  artist?: { id: number; name: string };
  numberOfTracks?: number;
  duration?: number;
  releaseDate?: string;
}

export interface PaginatedTracks {
  items: Track[];
  totalNumberOfItems: number;
  offset: number;
  limit: number;
}

export type AppView =
  | { type: "home" }
  | {
      type: "album";
      albumId: number;
      albumInfo?: { title: string; cover?: string; artistName?: string };
    }
  | {
      type: "playlist";
      playlistId: string;
      playlistInfo?: {
        title: string;
        image?: string;
        description?: string;
        creatorName?: string;
        numberOfTracks?: number;
      };
    }
  | { type: "favorites" };

export interface Playlist {
  uuid: string;
  title: string;
  description?: string;
  image?: string;
  numberOfTracks?: number;
  creator?: { id: number; name?: string };
}

export interface DeviceCode {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  verificationUriComplete: string;
  expiresIn: number;
  interval: number;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
  user_id?: number;
}

interface PlaybackSnapshot {
  currentTrack: Track | null;
  queue: Track[];
  history: Track[];
}

const PLAYBACK_STATE_KEY = "tide-vibe.playback-state.v1";
const VOLUME_STATE_KEY = "tide-vibe.volume.v1";

export function useAudio() {
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTrack, setCurrentTrack] = useState<Track | null>(null);
  const [volume, setVolumeState] = useState(1.0);
  const [queue, setQueue] = useState<Track[]>([]);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [userPlaylists, setUserPlaylists] = useState<Playlist[]>([]);
  const [authTokens, setAuthTokens] = useState<AuthTokens | null>(null);
  const [currentView, setCurrentView] = useState<AppView>({ type: "home" });
  const [history, setHistory] = useState<Track[]>([]);
  const currentTrackRef = useRef<Track | null>(null);
  const hasRestoredPlaybackRef = useRef(false);

  // Keep ref in sync so callbacks always see latest value
  useEffect(() => {
    currentTrackRef.current = currentTrack;
  }, [currentTrack]);

  // Restore last playback session (track + queue + history + volume)
  useEffect(() => {
    try {
      const raw = localStorage.getItem(PLAYBACK_STATE_KEY);
      if (!raw) {
        return;
      }

      const parsed = JSON.parse(raw) as Partial<PlaybackSnapshot>;

      if (parsed.currentTrack && typeof parsed.currentTrack.id === "number") {
        setCurrentTrack(parsed.currentTrack as Track);
      }

      if (Array.isArray(parsed.queue)) {
        setQueue(
          parsed.queue.filter(
            (track): track is Track => !!track && typeof track.id === "number"
          )
        );
      }

      if (Array.isArray(parsed.history)) {
        setHistory(
          parsed.history.filter(
            (track): track is Track => !!track && typeof track.id === "number"
          )
        );
      }

      // Backward compatibility: old snapshots included volume.
      if (typeof (parsed as { volume?: number }).volume === "number") {
        setVolumeState(
          Math.min(1, Math.max(0, (parsed as { volume?: number }).volume!))
        );
      }

      try {
        const rawVolume = localStorage.getItem(VOLUME_STATE_KEY);
        if (rawVolume != null) {
          const parsedVolume = Number(rawVolume);
          if (!Number.isNaN(parsedVolume)) {
            setVolumeState(Math.min(1, Math.max(0, parsedVolume)));
          }
        }
      } catch (err) {
        console.error("Failed to restore volume state:", err);
      }
    } catch (err) {
      console.error("Failed to restore playback state:", err);
    } finally {
      hasRestoredPlaybackRef.current = true;
    }
  }, []);

  // Load saved auth on mount
  useEffect(() => {
    const loadAuth = async () => {
      try {
        console.log("Loading saved auth...");
        const tokens = await invoke<AuthTokens | null>("load_saved_auth");
        console.log("Loaded tokens:", tokens);

        if (tokens) {
          // Get the user ID from session if not in tokens
          let userId = tokens.user_id;
          if (!userId) {
            try {
              userId = await invoke<number>("get_session_user_id");
              console.log("Got user ID from session:", userId);
            } catch (e) {
              console.error("Failed to get user ID:", e);
            }
          }

          let activeTokens = { ...tokens, user_id: userId };
          setAuthTokens(activeTokens);
          setIsAuthenticated(true);

          // Load playlists inline to avoid closure issues
          if (userId) {
            try {
              console.log("Loading playlists for user:", userId);
              const playlists = await invoke<Playlist[]>("get_user_playlists", {
                userId: userId,
              });
              console.log("Loaded playlists:", playlists?.length);
              setUserPlaylists(playlists || []);
            } catch (playlistErr: any) {
              const errStr = String(playlistErr);
              console.error("Failed to load playlists:", playlistErr);

              // Auto-refresh token on 401/expired errors
              if (errStr.includes("401") || errStr.includes("expired")) {
                try {
                  console.log("Token expired, attempting refresh...");
                  const refreshedTokens = await invoke<AuthTokens>(
                    "refresh_tidal_auth"
                  );
                  console.log("Token refreshed successfully");

                  activeTokens = {
                    ...refreshedTokens,
                    user_id: userId ?? refreshedTokens.user_id,
                  };
                  setAuthTokens(activeTokens);

                  // Retry loading playlists with refreshed token
                  const playlists = await invoke<Playlist[]>(
                    "get_user_playlists",
                    {
                      userId: userId,
                    }
                  );
                  console.log(
                    "Loaded playlists after refresh:",
                    playlists?.length
                  );
                  setUserPlaylists(playlists || []);
                } catch (refreshErr) {
                  console.error("Token refresh failed:", refreshErr);
                  // Refresh failed - force re-login
                  setIsAuthenticated(false);
                  setAuthTokens(null);
                  setUserPlaylists([]);
                }
              } else {
                setUserPlaylists([]);
              }
            }
          }
        }
      } catch (err) {
        console.error("Failed to load saved auth:", err);
      }
    };

    loadAuth();
  }, []);

  // Persist now-playing state and queue across app relaunches.
  // Keep this separate from volume persistence to avoid serializing large
  // queue/history payloads on every volume slider movement.
  useEffect(() => {
    if (!hasRestoredPlaybackRef.current) {
      return;
    }

    const snapshot: PlaybackSnapshot = {
      currentTrack,
      queue,
      history,
    };

    try {
      localStorage.setItem(PLAYBACK_STATE_KEY, JSON.stringify(snapshot));
    } catch (err) {
      console.error("Failed to persist playback state:", err);
    }
  }, [currentTrack, queue, history]);

  // Persist volume separately as a small scalar value.
  useEffect(() => {
    if (!hasRestoredPlaybackRef.current) {
      return;
    }

    try {
      localStorage.setItem(VOLUME_STATE_KEY, String(volume));
    } catch (err) {
      console.error("Failed to persist volume state:", err);
    }
  }, [volume]);

  // Auto-play next track when current finishes
  useEffect(() => {
    if (!isPlaying || !currentTrack) return;

    const checkInterval = setInterval(async () => {
      try {
        const isFinished = await invoke<boolean>("is_track_finished");
        if (isFinished && queue.length > 0) {
          playNext();
        }
      } catch (err) {
        console.error("Failed to check track status:", err);
      }
    }, 1000);

    return () => clearInterval(checkInterval);
  }, [isPlaying, currentTrack, queue]);

  const startAuth = async (): Promise<DeviceCode> => {
    try {
      return await invoke("start_tidal_auth");
    } catch (error) {
      console.error("Failed to start auth:", error);
      throw error;
    }
  };

  const pollAuth = async (deviceCode: string): Promise<AuthTokens> => {
    try {
      const tokens = await invoke<AuthTokens>("poll_tidal_auth", {
        deviceCode: deviceCode,
      });

      // Get the user ID from session
      let userId = tokens.user_id;
      if (!userId) {
        try {
          userId = await invoke<number>("get_session_user_id");
        } catch (e) {
          console.error("Failed to get user ID:", e);
        }
      }

      const updatedTokens = { ...tokens, user_id: userId };
      setAuthTokens(updatedTokens);
      setIsAuthenticated(true);
      return updatedTokens;
    } catch (error) {
      throw error;
    }
  };

  const logout = async () => {
    try {
      await invoke("logout");
      setAuthTokens(null);
      setIsAuthenticated(false);
      setUserPlaylists([]);
      setCurrentTrack(null);
      setIsPlaying(false);
      setQueue([]);
      setHistory([]);
      try {
        localStorage.removeItem(PLAYBACK_STATE_KEY);
        localStorage.removeItem(VOLUME_STATE_KEY);
      } catch (err) {
        console.error("Failed to clear playback state:", err);
      }
    } catch (error) {
      console.error("Failed to logout:", error);
    }
  };

  const getUserPlaylists = async (userId: number): Promise<Playlist[]> => {
    try {
      const playlists = await invoke<Playlist[]>("get_user_playlists", {
        userId: userId,
      });
      setUserPlaylists(playlists);
      return playlists;
    } catch (error) {
      console.error("Failed to get playlists:", error);
      return [];
    }
  };

  const getPlaylistTracks = useCallback(
    async (playlistId: string): Promise<Track[]> => {
      try {
        console.log("Getting playlist tracks for:", playlistId);
        const tracks = await invoke<Track[]>("get_playlist_tracks", {
          playlistId: playlistId,
        });
        console.log("Got tracks:", tracks?.length);
        return tracks || [];
      } catch (error: any) {
        console.error("Failed to get playlist tracks:", error);
        alert(`Failed to get tracks: ${error?.message || error}`);
        return [];
      }
    },
    []
  );

  const playTrack = async (track: Track) => {
    try {
      // Push current track to history before switching
      if (currentTrackRef.current) {
        setHistory((h) => [...h, currentTrackRef.current!]);
      }
      await invoke("play_tidal_track", { trackId: track.id });
      setCurrentTrack(track);
      setIsPlaying(true);
    } catch (error: any) {
      console.error("Failed to play track:", error);
    }
  };

  const pauseTrack = async () => {
    try {
      await invoke("pause_track");
      setIsPlaying(false);
    } catch (error) {
      console.error("Failed to pause track:", error);
    }
  };

  const resumeTrack = async () => {
    try {
      const track = currentTrackRef.current;
      if (!track) {
        return;
      }

      // After app relaunch we restore currentTrack in UI, but the backend has
      // no decoded audio loaded yet. If the backend is effectively "empty",
      // re-dispatch the current track instead of sending a no-op resume.
      const isFinished = await invoke<boolean>("is_track_finished");
      if (isFinished) {
        await invoke("play_tidal_track", { trackId: track.id });
      } else {
        await invoke("resume_track");
      }
      setIsPlaying(true);
    } catch (error) {
      console.error("Failed to resume track:", error);
    }
  };

  const setVolume = async (level: number) => {
    try {
      await invoke("set_volume", { level });
      setVolumeState(level);
    } catch (error) {
      console.error("Failed to set volume:", error);
    }
  };

  const getPlaybackPosition = async (): Promise<number> => {
    try {
      return await invoke<number>("get_playback_position");
    } catch (error) {
      console.error("Failed to get playback position:", error);
      return 0;
    }
  };

  const seekTo = async (positionSecs: number) => {
    try {
      await invoke("seek_track", { positionSecs });
    } catch (error) {
      console.error("Failed to seek:", error);
    }
  };

  const addToQueue = (track: Track) => {
    setQueue((prev) => [...prev, track]);
  };

  const setQueueTracks = (tracks: Track[]) => {
    setQueue(tracks);
  };

  const getAlbumDetail = useCallback(
    async (albumId: number): Promise<AlbumDetail> => {
      try {
        return await invoke<AlbumDetail>("get_album_detail", { albumId });
      } catch (error: any) {
        console.error("Failed to get album detail:", error);
        throw error;
      }
    },
    []
  );

  const getAlbumTracks = useCallback(
    async (
      albumId: number,
      offset: number = 0,
      limit: number = 50
    ): Promise<PaginatedTracks> => {
      try {
        return await invoke<PaginatedTracks>("get_album_tracks", {
          albumId,
          offset,
          limit,
        });
      } catch (error: any) {
        console.error("Failed to get album tracks:", error);
        throw error;
      }
    },
    []
  );

  const navigateToAlbum = (
    albumId: number,
    albumInfo?: { title: string; cover?: string; artistName?: string }
  ) => {
    setCurrentView({ type: "album", albumId, albumInfo });
  };

  const navigateToPlaylist = (
    playlistId: string,
    playlistInfo?: {
      title: string;
      image?: string;
      description?: string;
      creatorName?: string;
      numberOfTracks?: number;
    }
  ) => {
    setCurrentView({ type: "playlist", playlistId, playlistInfo });
  };

  const getFavoriteTracks = useCallback(
    async (
      offset: number = 0,
      limit: number = 50
    ): Promise<PaginatedTracks> => {
      if (!authTokens?.user_id) throw new Error("Not authenticated");
      try {
        return await invoke<PaginatedTracks>("get_favorite_tracks", {
          userId: authTokens.user_id,
          offset,
          limit,
        });
      } catch (error: any) {
        console.error("Failed to get favorite tracks:", error);
        throw error;
      }
    },
    [authTokens?.user_id]
  );

  const isTrackFavorited = async (trackId: number): Promise<boolean> => {
    if (!authTokens?.user_id) throw new Error("Not authenticated");
    try {
      return await invoke<boolean>("is_track_favorited", {
        userId: authTokens.user_id,
        trackId,
      });
    } catch (error: any) {
      console.error("Failed to check track favorite status:", error);
      throw error;
    }
  };

  const addFavoriteTrack = async (trackId: number): Promise<void> => {
    if (!authTokens?.user_id) throw new Error("Not authenticated");
    try {
      await invoke("add_favorite_track", {
        userId: authTokens.user_id,
        trackId,
      });
    } catch (error: any) {
      console.error("Failed to favorite track:", error);
      throw error;
    }
  };

  const removeFavoriteTrack = async (trackId: number): Promise<void> => {
    if (!authTokens?.user_id) throw new Error("Not authenticated");
    try {
      await invoke("remove_favorite_track", {
        userId: authTokens.user_id,
        trackId,
      });
    } catch (error: any) {
      console.error("Failed to remove favorite track:", error);
      throw error;
    }
  };

  const isAlbumFavorited = async (albumId: number): Promise<boolean> => {
    if (!authTokens?.user_id) throw new Error("Not authenticated");
    try {
      return await invoke<boolean>("is_album_favorited", {
        userId: authTokens.user_id,
        albumId,
      });
    } catch (error: any) {
      console.error("Failed to check album favorite status:", error);
      throw error;
    }
  };

  const addFavoriteAlbum = async (albumId: number): Promise<void> => {
    if (!authTokens?.user_id) throw new Error("Not authenticated");
    try {
      await invoke("add_favorite_album", {
        userId: authTokens.user_id,
        albumId,
      });
    } catch (error: any) {
      console.error("Failed to favorite album:", error);
      throw error;
    }
  };

  const removeFavoriteAlbum = async (albumId: number): Promise<void> => {
    if (!authTokens?.user_id) throw new Error("Not authenticated");
    try {
      await invoke("remove_favorite_album", {
        userId: authTokens.user_id,
        albumId,
      });
    } catch (error: any) {
      console.error("Failed to remove favorite album:", error);
      throw error;
    }
  };

  const navigateToFavorites = () => {
    setCurrentView({ type: "favorites" });
  };

  const navigateHome = () => {
    setCurrentView({ type: "home" });
  };

  const playNext = useCallback(async () => {
    if (queue.length > 0) {
      const [nextTrack, ...rest] = queue;
      setQueue(rest);
      await playTrack(nextTrack);
    } else {
      setIsPlaying(false);
    }
  }, [queue]);

  const playPrevious = useCallback(async () => {
    // If more than 3 seconds in, restart the current track
    try {
      const pos = await getPlaybackPosition();
      if (pos > 3) {
        await seekTo(0);
        return;
      }
    } catch {
      // ignore position errors
    }

    // Go to previous track from history
    if (history.length > 0) {
      const newHistory = [...history];
      const prevTrack = newHistory.pop()!;
      setHistory(newHistory);

      // Put current track back at front of queue
      if (currentTrackRef.current) {
        const curr = currentTrackRef.current;
        setQueue((prev) => [curr, ...prev]);
      }

      // Play previous track directly (playTrack would push to history again)
      try {
        await invoke("play_tidal_track", { trackId: prevTrack.id });
        setCurrentTrack(prevTrack);
        setIsPlaying(true);
      } catch (error: any) {
        console.error("Failed to play previous track:", error);
      }
    } else if (currentTrackRef.current) {
      // No history, just restart current track
      await seekTo(0);
    }
  }, [history]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      switch (e.code) {
        case "Space":
          e.preventDefault();
          if (isPlaying) {
            pauseTrack();
          } else {
            resumeTrack();
          }
          break;
        case "ArrowLeft":
          e.preventDefault();
          playPrevious();
          break;
        case "ArrowRight":
          e.preventDefault();
          playNext();
          break;
        case "ArrowUp":
          e.preventDefault();
          setVolume(Math.min(1.0, volume + 0.1));
          break;
        case "ArrowDown":
          e.preventDefault();
          setVolume(Math.max(0.0, volume - 0.1));
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isPlaying, volume, playNext, playPrevious]);

  return {
    isPlaying,
    currentTrack,
    volume,
    queue,
    isAuthenticated,
    userPlaylists,
    authTokens,
    currentView,
    playTrack,
    pauseTrack,
    resumeTrack,
    setVolume,
    seekTo,
    getPlaybackPosition,
    addToQueue,
    setQueueTracks,
    playNext,
    playPrevious,
    startAuth,
    pollAuth,
    logout,
    getUserPlaylists,
    getPlaylistTracks,
    getAlbumDetail,
    getAlbumTracks,
    getFavoriteTracks,
    isTrackFavorited,
    addFavoriteTrack,
    removeFavoriteTrack,
    isAlbumFavorited,
    addFavoriteAlbum,
    removeFavoriteAlbum,
    navigateToAlbum,
    navigateToPlaylist,
    navigateToFavorites,
    navigateHome,
  };
}
