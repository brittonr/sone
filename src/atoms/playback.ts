import { atom } from "jotai";
import { atomWithStorage } from "jotai/utils";
import type { Track, StreamInfo } from "../types";

export const isPlayingAtom = atom(false);
export const currentTrackAtom = atom<Track | null>(null);
export const volumeAtom = atomWithStorage("sone.volume.v1", 1.0);
export const queueAtom = atom<Track[]>([]);
export const historyAtom = atom<Track[]>([]);
export const streamInfoAtom = atom<StreamInfo | null>(null);
export const preMuteVolumeAtom = atom<number>(0);
export const autoplayAtom = atomWithStorage("sone.autoplay.v1", false);

/** true = use track replay gain (shuffle/mixed queue), false = use album replay gain (album in order) */
export const useTrackGainAtom = atom(true);
