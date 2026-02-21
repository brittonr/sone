import { LogOut, Palette, RefreshCw, User, Keyboard, X, MonitorDown, Volume2, Infinity as InfinityIcon, Headphones, Shield } from "lucide-react";
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAtom } from "jotai";
import { useAuth } from "../hooks/useAuth";
import { clearAllCache } from "../api/tidal";
import { autoplayAtom } from "../atoms/playback";
import ThemeEditor from "./ThemeEditor";

const SHORTCUTS = [
  { keys: "Space", desc: "Play / Pause" },
  { keys: "Ctrl + →", desc: "Next track" },
  { keys: "Ctrl + ←", desc: "Previous track" },
  { keys: "↑", desc: "Volume up" },
  { keys: "↓", desc: "Volume down" },
  { keys: "M", desc: "Mute / Unmute" },
  { keys: "L", desc: "Like / Unlike current track" },
  { keys: "Ctrl + S", desc: "Focus search bar" },
  { keys: "Ctrl + R", desc: "Refresh app data" },
  { keys: "Esc", desc: "Close now-playing drawer" },
  { keys: "Ctrl + +", desc: "Zoom in" },
  { keys: "Ctrl + -", desc: "Zoom out" },
  { keys: "Ctrl + 0", desc: "Reset zoom to 100%" },
  { keys: "?", desc: "Show keyboard shortcuts" },
] as const;

export default function UserMenu() {
  const { userName, logout } = useAuth();
  const [open, setOpen] = useState(false);
  const [themeOpen, setThemeOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [minimizeToTray, setMinimizeToTray] = useState(false);
  const [volumeNormalization, setVolumeNormalization] = useState(false);
  const [exclusiveMode, setExclusiveMode] = useState(false);
  const [bitPerfect, setBitPerfect] = useState(false);
  const [exclusiveDevice, setExclusiveDevice] = useState<string | null>(null);
  const [audioDevices, setAudioDevices] = useState<Array<{ id: string; name: string }>>([]);
  const [autoplay, setAutoplay] = useAtom(autoplayAtom);
  const menuRef = useRef<HTMLDivElement>(null);

  // Load preferences
  useEffect(() => {
    invoke<boolean>("get_minimize_to_tray").then(setMinimizeToTray).catch(() => {});
    invoke<boolean>("get_volume_normalization").then(setVolumeNormalization).catch(() => {});
    invoke<boolean>("get_exclusive_mode").then(setExclusiveMode).catch(() => {});
    invoke<boolean>("get_bit_perfect").then(setBitPerfect).catch(() => {});
    invoke<string | null>("get_exclusive_device").then(setExclusiveDevice).catch(() => {});
  }, []);

  // Toggle shortcuts modal from ? key
  useEffect(() => {
    const handler = () => setShortcutsOpen((prev) => !prev);
    window.addEventListener("toggle-shortcuts", handler);
    return () => window.removeEventListener("toggle-shortcuts", handler);
  }, []);

  // Load audio devices when exclusive mode is enabled
  useEffect(() => {
    if (exclusiveMode) {
      invoke<Array<{ id: string; name: string }>>("list_audio_devices")
        .then((devices) => {
          setAudioDevices(devices);
          if (!exclusiveDevice && devices.length > 0) {
            setExclusiveDevice(devices[0].id);
            invoke("set_exclusive_device", { device: devices[0].id }).catch(() => {});
          }
        })
        .catch(() => {});
    }
  }, [exclusiveMode]);

  // Close on click outside
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div ref={menuRef} className="relative">
      <button
        onClick={() => setOpen((prev) => !prev)}
        className="w-8 h-8 rounded-full bg-th-button hover:bg-th-button-hover flex items-center justify-center transition-colors"
        title="Account"
      >
        <User size={16} className="text-th-text-secondary" />
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-2 w-72 bg-th-surface rounded-lg shadow-2xl shadow-black/60 border border-th-border-subtle z-50 py-1 animate-fadeIn">
          {/* User info */}
          <div className="px-4 py-3 border-b border-th-border-subtle">
            <div className="flex items-center gap-3">
              <div className="w-9 h-9 rounded-full bg-th-button flex items-center justify-center shrink-0">
                <User size={16} className="text-th-text-muted" />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-[13px] font-medium text-white truncate">
                  {userName}
                </p>
              </div>
            </div>
          </div>

          {/* Theme */}
          <button
            onClick={() => {
              setOpen(false);
              setThemeOpen(true);
            }}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-th-text-secondary hover:text-white hover:bg-th-border-subtle transition-colors"
          >
            <Palette size={16} />
            Theme
          </button>

          {/* Refresh */}
          <button
            onClick={async () => {
              await clearAllCache();
              setOpen(false);
              window.location.reload();
            }}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-th-text-secondary hover:text-white hover:bg-th-border-subtle transition-colors"
          >
            <RefreshCw size={16} />
            Refresh App
          </button>

          {/* Shortcuts */}
          <button
            onClick={() => {
              setOpen(false);
              setShortcutsOpen(true);
            }}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-th-text-secondary hover:text-white hover:bg-th-border-subtle transition-colors"
          >
            <Keyboard size={16} />
            Shortcuts
          </button>

          {/* Volume normalization */}
          <button
            onClick={() => {
              if (bitPerfect) return;
              const next = !volumeNormalization;
              setVolumeNormalization(next);
              invoke("set_volume_normalization", { enabled: next }).catch(() => {});
            }}
            className={`w-full flex items-center gap-3 px-4 py-2.5 text-[13px] transition-colors ${
              bitPerfect
                ? "text-th-text-muted cursor-not-allowed opacity-50"
                : "text-th-text-secondary hover:text-white hover:bg-th-border-subtle"
            }`}
          >
            <Volume2 size={16} />
            <span className="flex-1 text-left">Normalize volume</span>
            <div
              className={`w-8 h-[18px] rounded-full transition-colors ${
                volumeNormalization ? "bg-th-accent" : "bg-th-border-subtle"
              }`}
            >
              <div
                className={`w-3.5 h-3.5 rounded-full bg-white mt-[2px] transition-transform ${
                  volumeNormalization ? "translate-x-[16px]" : "translate-x-[2px]"
                }`}
              />
            </div>
          </button>

          {/* Exclusive output */}
          <button
            onClick={() => {
              const next = !exclusiveMode;
              setExclusiveMode(next);
              if (!next) {
                setBitPerfect(false);
              }
              invoke("set_exclusive_mode", { enabled: next }).catch(() => {});
            }}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-th-text-secondary hover:text-white hover:bg-th-border-subtle transition-colors"
          >
            <Headphones size={16} />
            <span className="flex-1 text-left">Exclusive output</span>
            <div
              className={`w-8 h-[18px] rounded-full transition-colors ${
                exclusiveMode ? "bg-th-accent" : "bg-th-border-subtle"
              }`}
            >
              <div
                className={`w-3.5 h-3.5 rounded-full bg-white mt-[2px] transition-transform ${
                  exclusiveMode ? "translate-x-[16px]" : "translate-x-[2px]"
                }`}
              />
            </div>
          </button>

          {/* Device selector (visible when exclusive on) */}
          {exclusiveMode && audioDevices.length > 0 && (
            <div className="px-4 py-2 flex items-center gap-3">
              <div className="w-4" />
              <select
                value={exclusiveDevice || ""}
                onChange={(e) => {
                  setExclusiveDevice(e.target.value);
                  invoke("set_exclusive_device", { device: e.target.value }).catch(() => {});
                }}
                className="flex-1 bg-th-inset text-[12px] text-th-text-secondary rounded-md px-2 py-1.5 border border-th-border-subtle outline-none focus:border-th-accent"
              >
                {audioDevices.map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.name} ({d.id})
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Bit-perfect mode (visible when exclusive on) */}
          {exclusiveMode && (
            <button
              onClick={() => {
                const next = !bitPerfect;
                setBitPerfect(next);
                invoke("set_bit_perfect", { enabled: next }).catch(() => {});
              }}
              className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-th-text-secondary hover:text-white hover:bg-th-border-subtle transition-colors"
            >
              <Shield size={16} />
              <span className="flex-1 text-left">Bit-perfect</span>
              <div
                className={`w-8 h-[18px] rounded-full transition-colors ${
                  bitPerfect ? "bg-th-accent" : "bg-th-border-subtle"
                }`}
              >
                <div
                  className={`w-3.5 h-3.5 rounded-full bg-white mt-[2px] transition-transform ${
                    bitPerfect ? "translate-x-[16px]" : "translate-x-[2px]"
                  }`}
                />
              </div>
            </button>
          )}

          {/* Autoplay */}
          <button
            onClick={() => setAutoplay(!autoplay)}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-th-text-secondary hover:text-white hover:bg-th-border-subtle transition-colors"
          >
            <InfinityIcon size={16} />
            <span className="flex-1 text-left">Autoplay</span>
            <div
              className={`w-8 h-[18px] rounded-full transition-colors ${
                autoplay ? "bg-th-accent" : "bg-th-border-subtle"
              }`}
            >
              <div
                className={`w-3.5 h-3.5 rounded-full bg-white mt-[2px] transition-transform ${
                  autoplay ? "translate-x-[16px]" : "translate-x-[2px]"
                }`}
              />
            </div>
          </button>

          {/* Close to tray */}
          <button
            onClick={() => {
              const next = !minimizeToTray;
              setMinimizeToTray(next);
              invoke("set_minimize_to_tray", { enabled: next }).catch(() => {});
            }}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-th-text-secondary hover:text-white hover:bg-th-border-subtle transition-colors"
          >
            <MonitorDown size={16} />
            <span className="flex-1 text-left">Close to tray</span>
            <div
              className={`w-8 h-[18px] rounded-full transition-colors ${
                minimizeToTray ? "bg-th-accent" : "bg-th-border-subtle"
              }`}
            >
              <div
                className={`w-3.5 h-3.5 rounded-full bg-white mt-[2px] transition-transform ${
                  minimizeToTray ? "translate-x-[16px]" : "translate-x-[2px]"
                }`}
              />
            </div>
          </button>

          {/* Logout */}
          <button
            onClick={() => {
              setOpen(false);
              logout();
            }}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-[13px] text-red-400 hover:bg-th-border-subtle transition-colors"
          >
            <LogOut size={16} />
            Log out
          </button>
        </div>
      )}

      <ThemeEditor open={themeOpen} onClose={() => setThemeOpen(false)} />

      {/* Shortcuts modal */}
      {shortcutsOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
          onClick={() => setShortcutsOpen(false)}
        >
          <div
            className="bg-th-elevated rounded-xl shadow-2xl w-[420px] max-h-[80vh] flex flex-col overflow-hidden"
            onClick={(e) => e.stopPropagation()}
            style={{ animation: "slideUp 0.2s ease-out" }}
          >
            {/* Header */}
            <div className="flex items-center justify-between px-5 pt-5 pb-3">
              <h2 className="text-[16px] font-bold text-white">Keyboard Shortcuts</h2>
              <button
                onClick={() => setShortcutsOpen(false)}
                className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-th-inset transition-colors text-th-text-muted hover:text-white"
              >
                <X size={18} />
              </button>
            </div>

            {/* Shortcut list */}
            <div className="px-5 pb-5 flex flex-col gap-1">
              {SHORTCUTS.map((s) => (
                <div
                  key={s.keys}
                  className="flex items-center justify-between py-2 px-1"
                >
                  <span className="text-[13px] text-th-text-secondary">{s.desc}</span>
                  <kbd className="text-[12px] font-mono text-th-text-muted bg-th-surface px-2.5 py-1 rounded-md border border-th-border-subtle">
                    {s.keys}
                  </kbd>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
