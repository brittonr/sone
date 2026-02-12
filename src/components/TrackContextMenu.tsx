import {
  ListEnd,
  ListPlus,
  Heart,
  Radio,
  Trash2,
  ListMusic,
} from "lucide-react";
import { useState, useRef, useEffect, useCallback } from "react";
import { useAudioContext } from "../contexts/AudioContext";
import { type Track } from "../hooks/useAudio";
import AddToPlaylistMenu from "./AddToPlaylistMenu";

interface TrackContextMenuProps {
  track: Track;
  index: number;
  anchorRef: React.RefObject<HTMLButtonElement | null>;
  /** When provided (right-click), the menu opens at the cursor position */
  cursorPosition?: { x: number; y: number };
  onClose: () => void;
  /** If set, shows "Remove from playlist" option */
  playlistId?: string;
  isUserPlaylist?: boolean;
  onTrackRemoved?: (index: number) => void;
}

export default function TrackContextMenu({
  track,
  index,
  anchorRef,
  cursorPosition,
  onClose,
  playlistId,
  isUserPlaylist,
  onTrackRemoved,
}: TrackContextMenuProps) {
  const {
    addToQueue,
    playNextInQueue,
    favoriteTrackIds,
    addFavoriteTrack,
    removeFavoriteTrack,
    navigateToTrackRadio,
    removeTrackFromPlaylist,
  } = useAudioContext();

  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<{ top: number; left: number }>({
    top: -9999,
    left: -9999,
  });
  const [isPositioned, setIsPositioned] = useState(false);
  const [showPlaylistSubmenu, setShowPlaylistSubmenu] = useState(false);

  // Fake anchor ref for AddToPlaylistMenu positioning — we'll use the menu itself
  const playlistBtnRef = useRef<HTMLButtonElement | null>(null);

  const isFav = favoriteTrackIds.has(track.id);
  const canRemoveFromPlaylist = !!playlistId && !!isUserPlaylist;

  // Position the menu: measure actual size, clamp to viewport.
  // In Tauri's WebKit, getBoundingClientRect() returns CSS-pixel values while
  // mouse clientX/clientY are in viewport (zoomed) coordinates.  Only cursor
  // positions need zoom compensation; rect values & viewport bounds do not.
  useEffect(() => {
    const raf = requestAnimationFrame(() => {
      const menu = menuRef.current;
      if (!menu) return;

      const menuRect = menu.getBoundingClientRect();
      const menuWidth = menuRect.width || 240;
      const menuHeight = menuRect.height || 300;
      const viewW = window.innerWidth;
      const viewH = window.innerHeight;
      const pad = 8;

      let top: number;
      let left: number;

      if (cursorPosition) {
        // Right-click: clientX/Y are in viewport (zoomed) coords — convert to CSS px
        const zoom = parseFloat(document.documentElement.style.zoom || "1");
        top = cursorPosition.y / zoom;
        left = cursorPosition.x / zoom;
      } else if (anchorRef.current) {
        // Dots button: getBoundingClientRect already returns CSS px
        const rect = anchorRef.current.getBoundingClientRect();
        top = rect.bottom + 4;
        left = rect.right - menuWidth;
      } else {
        return;
      }

      // Clamp horizontally
      if (left < pad) left = pad;
      if (left + menuWidth > viewW - pad) {
        left = viewW - menuWidth - pad;
      }

      // Clamp vertically: flip upward if it would overflow
      if (top + menuHeight > viewH - pad) {
        if (cursorPosition) {
          const zoom = parseFloat(document.documentElement.style.zoom || "1");
          top = cursorPosition.y / zoom - menuHeight;
        } else if (anchorRef.current) {
          const rect = anchorRef.current.getBoundingClientRect();
          top = rect.top - menuHeight - 4;
        }
      }
      if (top < pad) top = pad;

      setPosition({ top, left });
      setIsPositioned(true);
    });

    return () => cancelAnimationFrame(raf);
  }, [anchorRef, cursorPosition, canRemoveFromPlaylist]);

  // Close on click outside
  useEffect(() => {
    if (showPlaylistSubmenu) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (
        menuRef.current &&
        !menuRef.current.contains(e.target as Node) &&
        anchorRef.current &&
        !anchorRef.current.contains(e.target as Node)
      ) {
        onClose();
      }
    };

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose, anchorRef, showPlaylistSubmenu]);

  const handlePlayNext = useCallback(() => {
    playNextInQueue(track);
    onClose();
  }, [track, playNextInQueue, onClose]);

  const handleAddToQueue = useCallback(() => {
    addToQueue(track);
    onClose();
  }, [track, addToQueue, onClose]);

  const handleToggleFavorite = useCallback(async () => {
    try {
      if (isFav) {
        await removeFavoriteTrack(track.id);
      } else {
        await addFavoriteTrack(track.id);
      }
    } catch (err) {
      console.error("Failed to toggle favorite:", err);
    }
    onClose();
  }, [track.id, isFav, addFavoriteTrack, removeFavoriteTrack, onClose]);

  const handleGoToTrackRadio = useCallback(() => {
    navigateToTrackRadio(track.id, {
      title: track.title,
      artistName: track.artist?.name,
      cover: track.album?.cover,
    });
    onClose();
  }, [track, navigateToTrackRadio, onClose]);

  const handleRemoveFromPlaylist = useCallback(async () => {
    if (!playlistId) return;
    try {
      await removeTrackFromPlaylist(playlistId, index);
      onTrackRemoved?.(index);
    } catch (err) {
      console.error("Failed to remove track from playlist:", err);
    }
    onClose();
  }, [playlistId, index, removeTrackFromPlaylist, onTrackRemoved, onClose]);

  const menuItemClass =
    "w-full flex items-center gap-3 px-4 py-2.5 hover:bg-[#ffffff0a] transition-colors text-left text-[14px] text-[#e0e0e0] hover:text-white";

  return (
    <>
      <div
        ref={menuRef}
        className="fixed z-9999 w-[240px] bg-[#1a1a1a] rounded-xl shadow-2xl overflow-hidden flex flex-col py-1"
        style={{
          top: position.top,
          left: position.left,
          opacity: isPositioned ? 1 : 0,
          animation: isPositioned ? "fadeIn 0.12s ease-out" : undefined,
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Play next */}
        <button className={menuItemClass} onClick={handlePlayNext}>
          <ListEnd size={18} className="shrink-0 text-[#a6a6a6]" />
          <span>Play next</span>
        </button>

        {/* Add to queue */}
        <button className={menuItemClass} onClick={handleAddToQueue}>
          <ListPlus size={18} className="shrink-0 text-[#a6a6a6]" />
          <span>Add to play queue</span>
        </button>

        {/* Divider */}
        <div className="my-1 border-t border-[#2a2a2a]" />

        {/* Add to playlist */}
        <button
          ref={playlistBtnRef}
          className={menuItemClass}
          onClick={() => setShowPlaylistSubmenu(true)}
        >
          <ListMusic size={18} className="shrink-0 text-[#a6a6a6]" />
          <span>Add to playlist</span>
        </button>

        {/* Add to / Remove from Loved tracks */}
        <button className={menuItemClass} onClick={handleToggleFavorite}>
          <Heart
            size={18}
            className={`shrink-0 ${isFav ? "text-[#00FFFF]" : "text-[#a6a6a6]"}`}
            fill={isFav ? "currentColor" : "none"}
          />
          <span>{isFav ? "Remove from Loved tracks" : "Add to Loved tracks"}</span>
        </button>

        {/* Divider */}
        <div className="my-1 border-t border-[#2a2a2a]" />

        {/* Go to track radio */}
        <button className={menuItemClass} onClick={handleGoToTrackRadio}>
          <Radio size={18} className="shrink-0 text-[#a6a6a6]" />
          <span>Go to track radio</span>
        </button>

        {/* Remove from playlist (only for user's own playlist) */}
        {canRemoveFromPlaylist && (
          <>
            <div className="my-1 border-t border-[#2a2a2a]" />
            <button
              className="w-full flex items-center gap-3 px-4 py-2.5 hover:bg-[#ffffff0a] transition-colors text-left text-[14px] text-[#ff6666] hover:text-[#ff4444]"
              onClick={handleRemoveFromPlaylist}
            >
              <Trash2 size={18} className="shrink-0" />
              <span>Remove from playlist</span>
            </button>
          </>
        )}
      </div>

      {/* Add to playlist submenu */}
      {showPlaylistSubmenu && (
        <AddToPlaylistMenu
          trackId={track.id}
          anchorRef={playlistBtnRef}
          onClose={() => {
            setShowPlaylistSubmenu(false);
            onClose();
          }}
        />
      )}
    </>
  );
}
