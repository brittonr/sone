import { Play, Heart, MoreHorizontal, Plus, Loader2 } from "lucide-react";
import { type Track, getTidalImageUrl } from "../hooks/useAudio";
import TidalImage from "./TidalImage";
import { useAudioContext } from "../contexts/AudioContext";
import { useRef, useEffect } from "react";

interface TrackListProps {
  tracks: Track[];
  onPlay: (track: Track, index: number) => void;
  showDateAdded?: boolean;
  showAlbum?: boolean;
  showCover?: boolean;
  showArtist?: boolean;
  onLoadMore?: () => void;
  hasMore?: boolean;
  loadingMore?: boolean;
  context?: "album" | "playlist" | "favorites";
}

function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

function formatDate(dateString?: string): string {
  if (!dateString) return "";
  const date = new Date(dateString);
  const now = new Date();
  const diffTime = Math.abs(now.getTime() - date.getTime());
  const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));

  if (diffDays <= 7) return "This week";
  if (diffDays <= 14) return "Last week";
  if (diffDays <= 30) return "Last month";
  
  return date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

export default function TrackList({
  tracks,
  onPlay,
  showDateAdded = false,
  showAlbum = true,
  showCover = true,
  showArtist = true,
  onLoadMore,
  hasMore = false,
  loadingMore = false,
  context = "playlist",
}: TrackListProps) {
  const {
    currentTrack,
    isPlaying,
    navigateToAlbum,
    favoriteTrackIds,
    addFavoriteTrack,
    removeFavoriteTrack,
  } = useAudioContext();

  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  const toggleFavorite = async (e: React.MouseEvent, track: Track) => {
    e.stopPropagation();
    const isFav = favoriteTrackIds.has(track.id);
    
    try {
      if (isFav) {
        await removeFavoriteTrack(track.id);
      } else {
        await addFavoriteTrack(track.id);
      }
    } catch (err) {
      console.error("Failed to toggle favorite", err);
    }
  };

  useEffect(() => {
    if (!onLoadMore) return;

    if (observerRef.current) {
      observerRef.current.disconnect();
    }

    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMore) {
          onLoadMore();
        }
      },
      { threshold: 0.1 }
    );

    if (sentinelRef.current) {
      observerRef.current.observe(sentinelRef.current);
    }

    return () => observerRef.current?.disconnect();
  }, [hasMore, onLoadMore]);

  const isCurrentlyPlaying = (track: Track) => {
    return currentTrack?.id === track.id && isPlaying;
  };

  const isCurrentTrackRow = (track: Track) => {
    return currentTrack?.id === track.id;
  };

  // Build grid columns string
  const gridCols = [
    "36px",                        // #
    showCover ? "minmax(200px, 4fr)" : "minmax(200px, 4fr)",  // Title (with or without cover)
    ...(showArtist ? ["minmax(120px, 2fr)"] : []),
    ...(showAlbum ? ["minmax(120px, 2fr)"] : []),
    ...(showDateAdded ? ["minmax(100px, 1fr)"] : []),
    "72px",                        // Time
    "100px",                       // Actions (always present for + and heart)
  ].join(" ");

  return (
    <div className="flex flex-col w-full">
      {/* Header Row */}
      <div
        className="grid gap-4 px-4 py-3 border-b border-[#2a2a2a] text-[12px] text-[#a6a6a6] uppercase tracking-widest mb-2"
        style={{ gridTemplateColumns: gridCols }}
      >
        <span className="text-right">#</span>
        <span>Title</span>
        {showArtist && <span>Artist</span>}
        {showAlbum && <span>Album</span>}
        {showDateAdded && <span>Date Added</span>}
        <span className="text-right">Time</span>
        <span /> {/* Actions column header */}
      </div>

      {/* Track Rows */}
      <div className="flex flex-col">
        {tracks.map((track, index) => {
          const isActive = isCurrentTrackRow(track);
          const playing = isCurrentlyPlaying(track);
          const isFav = favoriteTrackIds.has(track.id);

          return (
            <div
              key={`${track.id}-${index}`}
              onClick={() => onPlay(track, index)}
              className={`grid gap-4 px-4 py-2.5 rounded-md cursor-pointer group transition-colors items-center ${
                isActive ? "bg-[#ffffff0a]" : "hover:bg-[#ffffff08]"
              }`}
              style={{ gridTemplateColumns: gridCols }}
            >
              {/* Track Number / Playing Indicator */}
              <div className="flex items-center justify-end">
                {playing ? (
                  <div className="flex items-end gap-[3px] h-4">
                    <span className="w-[3px] h-full bg-[#00FFFF] rounded-full playing-bar" />
                    <span className="w-[3px] h-full bg-[#00FFFF] rounded-full playing-bar" style={{ animationDelay: "0.2s" }} />
                    <span className="w-[3px] h-full bg-[#00FFFF] rounded-full playing-bar" style={{ animationDelay: "0.4s" }} />
                  </div>
                ) : (
                  <>
                    <span
                      className={`text-[15px] tabular-nums group-hover:hidden ${
                        isActive ? "text-[#00FFFF]" : "text-[#a6a6a6]"
                      }`}
                    >
                      {context === "album" ? (track.trackNumber ?? index + 1) : index + 1}
                    </span>
                    <Play
                      size={14}
                      fill="white"
                      className="text-white hidden group-hover:block"
                    />
                  </>
                )}
              </div>

              {/* Title + Thumbnail */}
              <div className="flex items-center gap-3 min-w-0">
                {showCover && (
                  <div className="relative w-10 h-10 shrink-0 rounded bg-[#282828] overflow-hidden">
                    <TidalImage
                      src={getTidalImageUrl(track.album?.cover, 160)}
                      alt={track.album?.title || track.title}
                      className="w-full h-full object-cover"
                    />
                  </div>
                )}
                <div className="flex flex-col justify-center min-w-0">
                  <span
                    className={`text-[15px] font-medium truncate leading-snug ${
                      isActive ? "text-[#00FFFF]" : "text-white"
                    }`}
                  >
                    {track.title}
                  </span>
                  {!showArtist && (
                     <span className="text-[13px] text-[#a6a6a6] truncate leading-snug group-hover:text-white transition-colors">
                       {track.artist?.name || "Unknown Artist"}
                     </span>
                  )}
                </div>
              </div>

              {/* Artist (Column) */}
              {showArtist && (
                <div className="flex items-center min-w-0">
                  <span className="text-[14px] text-[#a6a6a6] truncate group-hover:text-white transition-colors">
                    {track.artist?.name || "Unknown Artist"}
                  </span>
                </div>
              )}

              {/* Album */}
              {showAlbum && (
                <div className="flex items-center min-w-0">
                  <span
                    className="text-[14px] text-[#a6a6a6] truncate hover:text-white hover:underline transition-colors cursor-pointer"
                    onClick={(e) => {
                      e.stopPropagation();
                      if (track.album?.id) {
                        navigateToAlbum(track.album.id, {
                          title: track.album.title,
                          cover: track.album.cover,
                          artistName: track.artist?.name,
                        });
                      }
                    }}
                  >
                    {track.album?.title || ""}
                  </span>
                </div>
              )}

              {/* Date Added */}
              {showDateAdded && (
                <div className="flex items-center min-w-0">
                  <span className="text-[14px] text-[#a6a6a6] truncate">
                    {formatDate(track.dateAdded)}
                  </span>
                </div>
              )}

              {/* Duration */}
              <div className="flex items-center justify-end text-[14px] text-[#a6a6a6] tabular-nums">
                {formatDuration(track.duration)}
              </div>

              {/* Actions: three dots on hover only, + and heart always visible */}
              <div className="flex items-center justify-end gap-2">
                <button 
                  className="p-1.5 text-[#a6a6a6] hover:text-white rounded-full transition-colors opacity-0 group-hover:opacity-100"
                  title="More options"
                  onClick={(e) => e.stopPropagation()}
                >
                  <MoreHorizontal size={18} />
                </button>
                <button 
                  className="p-1.5 text-[#a6a6a6] hover:text-white rounded-full transition-colors"
                  title="Add to playlist"
                  onClick={(e) => e.stopPropagation()}
                >
                  <Plus size={18} />
                </button>
                <button 
                  className={`p-1.5 rounded-full transition-colors ${isFav ? 'text-[#00FFFF]' : 'text-[#a6a6a6] hover:text-white'}`}
                  title={isFav ? "Remove from favorites" : "Add to favorites"}
                  onClick={(e) => toggleFavorite(e, track)}
                >
                  <Heart size={18} fill={isFav ? "currentColor" : "none"} />
                </button>
              </div>
            </div>
          );
        })}
      </div>

      {/* Infinite Scroll Sentinel */}
      {hasMore && (
        <div
          ref={sentinelRef}
          className="flex items-center justify-center py-8"
        >
          {loadingMore ? (
            <div className="flex items-center gap-3 text-[#a6a6a6]">
              <Loader2 size={20} className="animate-spin" />
              <span className="text-sm">Loading more tracks...</span>
            </div>
          ) : (
            <div className="h-8" />
          )}
        </div>
      )}
    </div>
  );
}
