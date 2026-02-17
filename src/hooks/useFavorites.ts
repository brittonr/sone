import { useCallback } from "react";
import { useAtom, useAtomValue } from "jotai";
import { invoke } from "@tauri-apps/api/core";
import { favoriteTrackIdsAtom, favoriteAlbumIdsAtom } from "../atoms/favorites";
import { authTokensAtom } from "../atoms/auth";
import {
  invalidateCache,
  addTrackToFavoritesCache,
  removeTrackFromFavoritesCache,
  addAlbumToFavoritesCache,
  removeAlbumFromFavoritesCache,
} from "../api/tidal";
import type { Track, AlbumDetail } from "../types";

export function useFavorites() {
  const [favoriteTrackIds, setFavoriteTrackIds] = useAtom(favoriteTrackIdsAtom);
  const [favoriteAlbumIds, setFavoriteAlbumIds] = useAtom(favoriteAlbumIdsAtom);
  const authTokens = useAtomValue(authTokensAtom);

  // NOTE: Initial loading of favorite track IDs has been moved to
  // AppInitializer to avoid firing once per component that calls useFavorites().

  const addFavoriteTrack = useCallback(
    async (trackId: number, track?: Track): Promise<void> => {
      if (!authTokens?.user_id) throw new Error("Not authenticated");
      // Optimistic update — reflect in UI immediately
      setFavoriteTrackIds((prev: Set<number>) => new Set([...prev, trackId]));
      if (track) addTrackToFavoritesCache(authTokens.user_id, track);
      try {
        await invoke("add_favorite_track", {
          userId: authTokens.user_id,
          trackId,
        });
      } catch (error: any) {
        // Revert on failure
        setFavoriteTrackIds((prev: Set<number>) => {
          const next = new Set(prev);
          next.delete(trackId);
          return next;
        });
        if (track) removeTrackFromFavoritesCache(authTokens.user_id, trackId);
        console.error("Failed to favorite track:", error);
        throw error;
      }
    },
    [authTokens?.user_id, setFavoriteTrackIds]
  );

  const removeFavoriteTrack = useCallback(
    async (trackId: number): Promise<void> => {
      if (!authTokens?.user_id) throw new Error("Not authenticated");
      // Optimistic update — reflect in UI immediately
      setFavoriteTrackIds((prev: Set<number>) => {
        const next = new Set(prev);
        next.delete(trackId);
        return next;
      });
      removeTrackFromFavoritesCache(authTokens.user_id, trackId);
      try {
        await invoke("remove_favorite_track", {
          userId: authTokens.user_id,
          trackId,
        });
      } catch (error: any) {
        // Revert on failure
        setFavoriteTrackIds((prev: Set<number>) => new Set([...prev, trackId]));
        console.error("Failed to remove favorite track:", error);
        throw error;
      }
    },
    [authTokens?.user_id, setFavoriteTrackIds]
  );

  const addFavoriteAlbum = useCallback(
    async (albumId: number, album?: AlbumDetail): Promise<void> => {
      if (!authTokens?.user_id) throw new Error("Not authenticated");
      // Optimistic update
      setFavoriteAlbumIds((prev: Set<number>) => new Set([...prev, albumId]));
      if (album) addAlbumToFavoritesCache(authTokens.user_id, album);
      try {
        await invoke("add_favorite_album", {
          userId: authTokens.user_id,
          albumId,
        });
      } catch (error: any) {
        // Revert on failure
        setFavoriteAlbumIds((prev: Set<number>) => {
          const next = new Set(prev);
          next.delete(albumId);
          return next;
        });
        if (album) removeAlbumFromFavoritesCache(authTokens.user_id, albumId);
        console.error("Failed to favorite album:", error);
        throw error;
      }
    },
    [authTokens?.user_id, setFavoriteAlbumIds]
  );

  const removeFavoriteAlbum = useCallback(
    async (albumId: number): Promise<void> => {
      if (!authTokens?.user_id) throw new Error("Not authenticated");
      // Optimistic update
      setFavoriteAlbumIds((prev: Set<number>) => {
        const next = new Set(prev);
        next.delete(albumId);
        return next;
      });
      removeAlbumFromFavoritesCache(authTokens.user_id, albumId);
      try {
        await invoke("remove_favorite_album", {
          userId: authTokens.user_id,
          albumId,
        });
      } catch (error: any) {
        // Revert on failure
        setFavoriteAlbumIds((prev: Set<number>) => new Set([...prev, albumId]));
        console.error("Failed to remove favorite album:", error);
        throw error;
      }
    },
    [authTokens?.user_id, setFavoriteAlbumIds]
  );

  const addFavoritePlaylist = useCallback(
    async (playlistUuid: string): Promise<void> => {
      if (!authTokens?.user_id) throw new Error("Not authenticated");
      try {
        await invoke("add_favorite_playlist", {
          userId: authTokens.user_id,
          playlistUuid,
        });
        invalidateCache("fav-");
      } catch (error: any) {
        console.error("Failed to favorite playlist:", error);
        throw error;
      }
    },
    [authTokens?.user_id]
  );

  const removeFavoritePlaylist = useCallback(
    async (playlistUuid: string): Promise<void> => {
      if (!authTokens?.user_id) throw new Error("Not authenticated");
      try {
        await invoke("remove_favorite_playlist", {
          userId: authTokens.user_id,
          playlistUuid,
        });
        invalidateCache("fav-");
      } catch (error: any) {
        console.error("Failed to remove favorite playlist:", error);
        throw error;
      }
    },
    [authTokens?.user_id]
  );

  return {
    favoriteTrackIds,
    addFavoriteTrack,
    removeFavoriteTrack,
    favoriteAlbumIds,
    addFavoriteAlbum,
    removeFavoriteAlbum,
    addFavoritePlaylist,
    removeFavoritePlaylist,
  };
}
