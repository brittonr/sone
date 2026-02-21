use gstreamer as gst;
use gst::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use tauri::Emitter;

type Reply<T> = mpsc::Sender<T>;

#[derive(Debug, Clone, Serialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}

enum AudioCommand {
    PlayUrl { uri: String, reply: Reply<Result<(), String>> },
    Pause { reply: Reply<Result<(), String>> },
    Resume { reply: Reply<Result<(), String>> },
    Stop { reply: Reply<Result<(), String>> },
    SetVolume { level: f32, reply: Reply<Result<(), String>> },
    SetNormalizationGain { gain: f64, reply: Reply<Result<(), String>> },
    Seek { position_secs: f32, reply: Reply<Result<(), String>> },
    GetPosition { reply: Reply<Result<f32, String>> },
    IsFinished { reply: Reply<Result<bool, String>> },
    SetExclusiveMode { enabled: bool, device: Option<String>, reply: Reply<Result<(), String>> },
    SetBitPerfect { enabled: bool, reply: Reply<Result<(), String>> },
    ListDevices { reply: Reply<Result<Vec<AudioDevice>, String>> },
}

pub struct AudioPlayer {
    cmd_tx: mpsc::Sender<AudioCommand>,
}

impl AudioPlayer {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<AudioCommand>();

        std::thread::spawn(move || {
            // GStreamer plugin path setup
            if std::env::var("GST_PLUGIN_PATH_1_0").is_ok()
                || std::env::var("APPDIR").is_ok()
            {
                if let Ok(path) = std::env::var("GST_PLUGIN_PATH_1_0") {
                    std::env::set_var("GST_PLUGIN_PATH", &path);
                }
            } else if std::env::var("GST_PLUGIN_PATH").is_err() {
                for dir in [
                    "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
                    "/usr/lib64/gstreamer-1.0",
                    "/usr/lib/gstreamer-1.0",
                ] {
                    if std::path::Path::new(dir).is_dir() {
                        std::env::set_var("GST_PLUGIN_PATH", dir);
                        break;
                    }
                }
            }

            gst::init().expect("Failed to initialize GStreamer");

            let mut pipeline: Option<gst::Pipeline> = None;
            let mut user_volume_el: Option<gst::Element> = None;
            let mut norm_volume_el: Option<gst::Element> = None;
            let eos = Arc::new(AtomicBool::new(false));
            let tearing_down = Arc::new(AtomicBool::new(false));
            let has_uri = AtomicBool::new(false);

            let mut exclusive = false;
            let mut bit_perfect = false;
            let mut device: Option<String> = None;
            // What mode the *running* pipeline was built with (for teardown).
            // Mode toggles take effect on next PlayUrl, so teardown must use
            // the old pipeline's mode, not the current toggle state.
            let mut pipeline_exclusive = false;

            let mut current_volume: f64 = 1.0;
            let mut current_norm_gain: f64 = 1.0;

            for cmd in cmd_rx {
                match cmd {
                    AudioCommand::PlayUrl { uri, reply } => {
                        let result = (|| -> Result<(), String> {
                            if let Some(old) = pipeline.take() {
                                tearing_down.store(true, Ordering::SeqCst);

                                // Fade volume to silence before teardown
                                if let Some(ref vol) = user_volume_el {
                                    for i in (0..10).rev() {
                                        vol.set_property("volume", current_volume * (i as f64 / 10.0));
                                        std::thread::sleep(std::time::Duration::from_millis(10));
                                    }
                                    if pipeline_exclusive {
                                        // ALSA ring buffer needs extra time for silence to propagate
                                        std::thread::sleep(std::time::Duration::from_millis(150));
                                    }
                                }

                                // EOS drain: flush buffers gracefully
                                old.send_event(gst::event::Eos::new());
                                let drain_timeout = if pipeline_exclusive { 1000 } else { 500 };
                                let start = std::time::Instant::now();
                                while !eos.load(Ordering::SeqCst)
                                    && start.elapsed() < std::time::Duration::from_millis(drain_timeout)
                                {
                                    std::thread::sleep(std::time::Duration::from_millis(10));
                                }
                                log::debug!("[audio] teardown: EOS drain took {:?}", start.elapsed());

                                old.set_state(gst::State::Null)
                                    .map_err(|e| format!("Failed to stop old pipeline: {e}"))?;
                                let _ = old.state(gst::ClockTime::from_mseconds(500));
                                drop(old);
                                if pipeline_exclusive {
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                }
                                tearing_down.store(false, Ordering::SeqCst);
                                log::debug!("[audio] teardown: complete");
                            }
                            user_volume_el = None;
                            norm_volume_el = None;
                            eos.store(false, Ordering::SeqCst);
                            has_uri.store(true, Ordering::SeqCst);

                            let pipe = gst::Pipeline::new();
                            let uridecodebin = gst::ElementFactory::make("uridecodebin")
                                .property("uri", &uri)
                                .build()
                                .map_err(|e| format!("Failed to create uridecodebin: {e}"))?;
                            let audioconvert = gst::ElementFactory::make("audioconvert")
                                .build()
                                .map_err(|e| format!("Failed to create audioconvert: {e}"))?;

                            log::debug!("[audio] building pipeline: exclusive={exclusive} bit_perfect={bit_perfect}");
                            let (u_vol, n_vol) = if bit_perfect {
                                // Disable dithering and noise shaping so audioconvert
                                // only does byte-layout conversion (endianness/interleaving)
                                audioconvert.set_property("dither", 0i32);
                                audioconvert.set_property_from_str("noise-shaping", "none");

                                let capsfilter = gst::ElementFactory::make("capsfilter")
                                    .build()
                                    .map_err(|e| format!("Failed to create capsfilter: {e}"))?;
                                let dev = device.as_deref()
                                    .ok_or_else(|| "No audio device selected for exclusive mode".to_string())?;
                                let sink = gst::ElementFactory::make("alsasink")
                                    .property("device", dev)
                                    .property("buffer-time", 500_000i64)
                                    .property("latency-time", 50_000i64)
                                    .build()
                                    .map_err(|e| format!("Failed to create alsasink: {e}"))?;

                                pipe.add_many([&uridecodebin, &audioconvert, &capsfilter, &sink])
                                    .map_err(|e| format!("Failed to add elements: {e}"))?;
                                gst::Element::link_many([&audioconvert, &capsfilter, &sink])
                                    .map_err(|e| format!("Failed to link bit-perfect chain: {e}"))?;

                                (None, None)
                            } else if exclusive {
                                // Exclusive (non-bit-perfect): lock output caps so alsasink
                                // never sees a format change mid-stream, and audiorate fills
                                // gaps with silence instead of XRUN on network stalls.
                                let audioresample = gst::ElementFactory::make("audioresample")
                                    .build()
                                    .map_err(|e| format!("Failed to create audioresample: {e}"))?;
                                let audiorate = gst::ElementFactory::make("audiorate")
                                    .build()
                                    .map_err(|e| format!("Failed to create audiorate: {e}"))?;
                                let norm_vol = gst::ElementFactory::make("volume")
                                    .property("volume", current_norm_gain)
                                    .build()
                                    .map_err(|e| format!("Failed to create norm volume: {e}"))?;
                                let user_vol = gst::ElementFactory::make("volume")
                                    .property("volume", current_volume)
                                    .build()
                                    .map_err(|e| format!("Failed to create user volume: {e}"))?;
                                let capsfilter = gst::ElementFactory::make("capsfilter")
                                    .property("caps", gst::Caps::builder("audio/x-raw")
                                        .field("format", "S32LE")
                                        .field("rate", 48000i32)
                                        .field("channels", 2i32)
                                        .build())
                                    .build()
                                    .map_err(|e| format!("Failed to create capsfilter: {e}"))?;
                                let dev = device.as_deref()
                                    .ok_or_else(|| "No audio device selected for exclusive mode".to_string())?;
                                let sink = gst::ElementFactory::make("alsasink")
                                    .property("device", dev)
                                    .property("buffer-time", 500_000i64)
                                    .property("latency-time", 50_000i64)
                                    .build()
                                    .map_err(|e| format!("Failed to create alsasink: {e}"))?;

                                pipe.add_many([&uridecodebin, &audioconvert, &audioresample, &audiorate, &norm_vol, &user_vol, &capsfilter, &sink])
                                    .map_err(|e| format!("Failed to add elements: {e}"))?;
                                gst::Element::link_many([&audioconvert, &audioresample, &audiorate, &norm_vol, &user_vol, &capsfilter, &sink])
                                    .map_err(|e| format!("Failed to link exclusive chain: {e}"))?;

                                (Some(user_vol), Some(norm_vol))
                            } else {
                                // Normal playback: autoaudiosink through PipeWire
                                let audioresample = gst::ElementFactory::make("audioresample")
                                    .build()
                                    .map_err(|e| format!("Failed to create audioresample: {e}"))?;
                                let norm_vol = gst::ElementFactory::make("volume")
                                    .property("volume", current_norm_gain)
                                    .build()
                                    .map_err(|e| format!("Failed to create norm volume: {e}"))?;
                                let user_vol = gst::ElementFactory::make("volume")
                                    .property("volume", current_volume)
                                    .build()
                                    .map_err(|e| format!("Failed to create user volume: {e}"))?;
                                let sink = gst::ElementFactory::make("autoaudiosink")
                                    .build()
                                    .map_err(|e| format!("Failed to create autoaudiosink: {e}"))?;

                                pipe.add_many([&uridecodebin, &audioconvert, &audioresample, &norm_vol, &user_vol, &sink])
                                    .map_err(|e| format!("Failed to add elements: {e}"))?;
                                gst::Element::link_many([&audioconvert, &audioresample, &norm_vol, &user_vol, &sink])
                                    .map_err(|e| format!("Failed to link chain: {e}"))?;

                                (Some(user_vol), Some(norm_vol))
                            };

                            // Grab a weak ref to the capsfilter for bit-perfect cap locking
                            let capsfilter_weak: Option<gst::glib::WeakRef<gst::Element>> = if bit_perfect {
                                audioconvert.static_pad("src")
                                    .and_then(|p| p.peer())
                                    .and_then(|p| p.parent_element())
                                    .map(|el| el.downgrade())
                            } else {
                                None
                            };

                            // Connect uridecodebin's dynamic pad to audioconvert
                            let convert_weak = audioconvert.downgrade();
                            uridecodebin.connect_pad_added(move |_src, src_pad| {
                                let Some(convert) = convert_weak.upgrade() else { return };
                                let Some(sink_pad) = convert.static_pad("sink") else { return };
                                if sink_pad.is_linked() { return; }

                                if let Some(caps) = src_pad.current_caps() {
                                    if let Some(s) = caps.structure(0) {
                                        if !s.name().as_str().starts_with("audio/") {
                                            return;
                                        }
                                    }
                                }

                                if let Err(e) = src_pad.link(&sink_pad) {
                                    log::error!("Failed to link uridecodebin pad: {e:?}");
                                }

                                // Bit-perfect: lock capsfilter to decoded format so
                                // audioconvert cannot silently convert bit depth
                                if let Some(ref cf_weak) = capsfilter_weak {
                                    if let Some(cf) = cf_weak.upgrade() {
                                        let caps = src_pad.current_caps()
                                            .or_else(|| {
                                                let query = src_pad.query_caps(None);
                                                if query.is_fixed() { Some(query) } else { None }
                                            });
                                        if let Some(caps) = caps {
                                            if let Some(s) = caps.structure(0) {
                                                if let (Ok(rate), Ok(channels), Ok(format)) = (
                                                    s.get::<i32>("rate"),
                                                    s.get::<i32>("channels"),
                                                    s.get::<&str>("format"),
                                                ) {
                                                    let locked = if format.starts_with("S24") {
                                                        // 24-bit: allow byte-alignment variants
                                                        // (S24LE packed, S32LE padded, S24_32LE container)
                                                        // Hard-blocks truncation to S16LE
                                                        gst::Caps::builder("audio/x-raw")
                                                            .field("format", gst::List::new(["S24LE", "S32LE", "S24_32LE"]))
                                                            .field("rate", rate)
                                                            .field("channels", channels)
                                                            .build()
                                                    } else {
                                                        // Non-24-bit: lock to exact format
                                                        gst::Caps::builder("audio/x-raw")
                                                            .field("format", format)
                                                            .field("rate", rate)
                                                            .field("channels", channels)
                                                            .build()
                                                    };
                                                    log::info!("[audio] bit-perfect: locking capsfilter to {locked}");
                                                    cf.set_property("caps", &locked);
                                                }
                                            }
                                        }
                                    }
                                }
                            });

                            // Start pipeline — staged transition for hardware sinks
                            if exclusive || bit_perfect {
                                // Single attempt: ALSA device opens during async NULL→READY.
                                // If PipeWire holds the device (EBUSY), we return "device_busy"
                                // immediately so the frontend can retry with UI feedback.
                                pipe.set_state(gst::State::Paused)
                                    .map_err(|e| format!("Failed to start pipeline: {e:?}"))?;
                                let (state_result, _, _) = pipe.state(gst::ClockTime::from_seconds(10));
                                if state_result.is_err() {
                                    // Check bus for EBUSY vs other failures
                                    let mut is_busy = false;
                                    if let Some(bus) = pipe.bus() {
                                        while let Some(msg) = bus.pop() {
                                            if let gst::MessageView::Error(err) = msg.view() {
                                                let err_msg = err.error().to_string();
                                                let debug_str = err.debug().map(|s| s.to_string()).unwrap_or_default();
                                                if err_msg.contains("busy") || debug_str.contains("busy")
                                                    || err_msg.contains("EBUSY") || debug_str.contains("EBUSY") {
                                                    is_busy = true;
                                                }
                                            }
                                        }
                                    }
                                    pipe.set_state(gst::State::Null).ok();
                                    return Err(if is_busy { "device_busy" } else { "Failed to start playback" }.into());
                                }
                                pipe.set_state(gst::State::Playing)
                                    .map_err(|e| format!("Failed to start playback: {e}"))?;
                            } else {
                                pipe.set_state(gst::State::Playing)
                                    .map_err(|e| format!("Failed to start playback: {e}"))?;
                            }
                            user_volume_el = u_vol;
                            norm_volume_el = n_vol;

                            // Bus watcher — detects EOS/Error + ALSA EBUSY
                            let eos_flag = Arc::clone(&eos);
                            let tearing_down_flag = Arc::clone(&tearing_down);
                            let app_handle_clone = app_handle.clone();
                            if let Some(bus) = pipe.bus() {
                                std::thread::spawn(move || {
                                    for msg in bus.iter_timed(gst::ClockTime::NONE) {
                                        match msg.view() {
                                            gst::MessageView::Eos(..) => {
                                                eos_flag.store(true, Ordering::SeqCst);
                                                if !tearing_down_flag.load(Ordering::SeqCst) {
                                                    app_handle_clone.emit("track-finished", ()).ok();
                                                }
                                                break;
                                            }
                                            gst::MessageView::Error(err) => {
                                                let err_msg = err.error().to_string();
                                                let debug_str = err.debug()
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_default();
                                                log::error!(
                                                    "GStreamer error: {} (debug: {})",
                                                    err_msg, debug_str
                                                );

                                                let is_busy = err_msg.contains("busy") || debug_str.contains("busy")
                                                    || err_msg.contains("EBUSY") || debug_str.contains("EBUSY");
                                                let kind = if is_busy { "device_busy" } else { "playback_error" };
                                                app_handle_clone.emit("audio-error",
                                                    serde_json::json!({ "kind": kind, "message": err_msg })
                                                ).ok();

                                                eos_flag.store(true, Ordering::SeqCst);
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                });
                            }

                            pipeline = Some(pipe);
                            pipeline_exclusive = exclusive || bit_perfect;
                            Ok(())
                        })();
                        reply.send(result).ok();
                    }

                    AudioCommand::Pause { reply } => {
                        let result = match pipeline.as_ref() {
                            Some(p) => p.set_state(gst::State::Paused)
                                .map(|_| ()).map_err(|e| format!("Failed to pause: {e}")),
                            None => Err("No active pipeline".into()),
                        };
                        reply.send(result).ok();
                    }

                    AudioCommand::Resume { reply } => {
                        let result = match pipeline.as_ref() {
                            Some(p) => p.set_state(gst::State::Playing)
                                .map(|_| ()).map_err(|e| format!("Failed to resume: {e}")),
                            None => Err("No active pipeline".into()),
                        };
                        reply.send(result).ok();
                    }

                    AudioCommand::Stop { reply } => {
                        let result = match pipeline.as_ref() {
                            Some(p) => {
                                if pipeline_exclusive {
                                    if let Some(ref vol) = user_volume_el {
                                        for i in (0..10).rev() {
                                            vol.set_property("volume", current_volume * (i as f64 / 10.0));
                                            std::thread::sleep(std::time::Duration::from_millis(10));
                                        }
                                        std::thread::sleep(std::time::Duration::from_millis(150));
                                    }
                                }
                                p.set_state(gst::State::Null)
                                    .map(|_| {
                                        eos.store(false, Ordering::SeqCst);
                                        has_uri.store(false, Ordering::SeqCst);
                                    }).map_err(|e| format!("Failed to stop: {e}"))
                            }
                            None => Ok(()),
                        };
                        reply.send(result).ok();
                    }

                    AudioCommand::SetVolume { level, reply } => {
                        current_volume = level as f64;
                        if let Some(ref vol) = user_volume_el {
                            vol.set_property("volume", current_volume);
                        }
                        reply.send(Ok(())).ok();
                    }

                    AudioCommand::SetNormalizationGain { gain, reply } => {
                        current_norm_gain = gain;
                        if let Some(ref vol) = norm_volume_el {
                            vol.set_property("volume", gain);
                        }
                        reply.send(Ok(())).ok();
                    }

                    AudioCommand::Seek { position_secs, reply } => {
                        let result = match pipeline.as_ref() {
                            Some(p) => {
                                let pos = gst::ClockTime::from_nseconds(
                                    (position_secs as f64 * 1_000_000_000.0) as u64,
                                );
                                p.seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, pos)
                                    .map_err(|e| format!("Seek failed: {e}"))
                            }
                            None => Err("No active pipeline".into()),
                        };
                        reply.send(result).ok();
                    }

                    AudioCommand::GetPosition { reply } => {
                        let pos = pipeline.as_ref()
                            .and_then(|p| p.query_position::<gst::ClockTime>())
                            .map(|pos| pos.nseconds() as f32 / 1_000_000_000.0)
                            .unwrap_or(0.0);
                        reply.send(Ok(pos)).ok();
                    }

                    AudioCommand::IsFinished { reply } => {
                        let finished = eos.load(Ordering::SeqCst)
                            || !has_uri.load(Ordering::SeqCst);
                        reply.send(Ok(finished)).ok();
                    }

                    AudioCommand::SetExclusiveMode { enabled, device: dev, reply } => {
                        exclusive = enabled;
                        if let Some(d) = dev {
                            device = Some(d);
                        }
                        if !enabled {
                            bit_perfect = false;
                        }
                        reply.send(Ok(())).ok();
                    }

                    AudioCommand::SetBitPerfect { enabled, reply } => {
                        bit_perfect = enabled;
                        if enabled {
                            exclusive = true;
                        }
                        reply.send(Ok(())).ok();
                    }

                    AudioCommand::ListDevices { reply } => {
                        let result = list_alsa_devices_inner();
                        reply.send(result).ok();
                    }
                }
            }
        });

        Self { cmd_tx }
    }

    fn send_cmd<T>(&self, build: impl FnOnce(Reply<T>) -> AudioCommand) -> T {
        let (tx, rx) = mpsc::channel();
        let cmd = build(tx);
        self.cmd_tx.send(cmd).expect("Audio thread dead");
        rx.recv().expect("Audio thread dead")
    }

    pub fn play_url(&self, uri: &str) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::PlayUrl { uri: uri.to_string(), reply })
    }
    pub fn pause(&self) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::Pause { reply })
    }
    pub fn resume(&self) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::Resume { reply })
    }
    pub fn stop(&self) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::Stop { reply })
    }
    pub fn set_volume(&self, level: f32) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::SetVolume { level, reply })
    }
    pub fn set_normalization_gain(&self, gain: f64) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::SetNormalizationGain { gain, reply })
    }
    pub fn seek(&self, position_secs: f32) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::Seek { position_secs, reply })
    }
    pub fn get_position(&self) -> Result<f32, String> {
        self.send_cmd(|reply| AudioCommand::GetPosition { reply })
    }
    pub fn is_finished(&self) -> Result<bool, String> {
        self.send_cmd(|reply| AudioCommand::IsFinished { reply })
    }
    pub fn set_exclusive_mode(&self, enabled: bool, device: Option<String>) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::SetExclusiveMode { enabled, device, reply })
    }
    pub fn set_bit_perfect(&self, enabled: bool) -> Result<(), String> {
        self.send_cmd(|reply| AudioCommand::SetBitPerfect { enabled, reply })
    }
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>, String> {
        self.send_cmd(|reply| AudioCommand::ListDevices { reply })
    }
}

/// Enumerate ALSA hardware devices. Does NOT use the audio pipeline,
/// so it is safe to call from any thread.
pub fn list_alsa_devices() -> Result<Vec<AudioDevice>, String> {
    list_alsa_devices_inner()
}

fn list_alsa_devices_inner() -> Result<Vec<AudioDevice>, String> {
    let monitor = gst::DeviceMonitor::new();
    let caps = gst::Caps::new_empty_simple("audio/x-raw");
    monitor.add_filter(Some("Audio/Sink"), Some(&caps));
    monitor.start().map_err(|e| format!("Failed to start device monitor: {e}"))?;
    let devices = monitor.devices();
    monitor.stop();

    log::debug!("[list_alsa_devices] DeviceMonitor found {} devices", devices.len());

    let mut result = Vec::new();
    for dev in &devices {
        let Some(props) = dev.properties() else { continue };

        let api = props.get::<String>("device.api").unwrap_or_default();
        if api != "alsa" {
            continue;
        }

        // PipeWire exposes alsa.card and alsa.device as strings, not i32.
        // Use api.alsa.path directly (e.g. "hw:5,0") when available,
        // otherwise construct from string card+device properties.
        let path = props.get::<String>("api.alsa.path").ok()
            .or_else(|| {
                let card = props.get::<String>("alsa.card").ok()?;
                let dev_num = props.get::<String>("alsa.device").ok()?;
                Some(format!("hw:{card},{dev_num}"))
            });

        if let Some(path) = path {
            let name = dev.display_name().to_string();
            log::debug!("[list_alsa_devices] found: '{}' -> {}", name, path);
            result.push(AudioDevice { id: path, name });
        }
    }

    log::debug!("[list_alsa_devices] returning {} devices", result.len());
    Ok(result)
}
