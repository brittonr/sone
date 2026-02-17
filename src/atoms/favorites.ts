import { atom } from "jotai";

export const favoriteTrackIdsAtom = atom<Set<number>>(new Set<number>());
export const favoriteAlbumIdsAtom = atom<Set<number>>(new Set<number>());
