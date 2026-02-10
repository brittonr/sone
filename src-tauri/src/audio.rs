use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

pub enum AudioCommand {
    Play { bytes: Vec<u8>, generation: u64 },
    PlayFailed { generation: u64 },
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Seek(f32),
    GetPosition(std::sync::mpsc::Sender<f32>),
    IsFinished(std::sync::mpsc::Sender<bool>),
}

pub struct AudioPlayer {
    sender: Sender<AudioCommand>,
    /// Monotonic counter incremented on every play request.
    /// Background download threads compare against this to detect if a newer
    /// play request has superseded them (avoids stale downloads overriding the
    /// latest track).
    play_generation: Arc<AtomicU64>,
    /// Whether playback should currently be running. This is updated immediately
    /// on pause/resume calls, so delayed downloads can honor the latest intent.
    desired_playing: Arc<AtomicBool>,
    /// Generation ID of an in-flight download, or 0 when no download is pending.
    pending_generation: Arc<AtomicU64>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (sender, receiver) = channel::<AudioCommand>();
        let desired_playing = Arc::new(AtomicBool::new(false));
        let pending_generation = Arc::new(AtomicU64::new(0));
        let desired_playing_for_thread = Arc::clone(&desired_playing);
        let pending_generation_for_thread = Arc::clone(&pending_generation);

        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
            let mut play_start: Option<Instant> = None;
            let mut accumulated_time: f32 = 0.0;
            let mut is_paused = false;
            let mut current_bytes: Option<Vec<u8>> = None;

            loop {
                match receiver.recv() {
                    Ok(AudioCommand::Play { bytes, generation }) => {
                        if generation != 0
                            && pending_generation_for_thread.load(Ordering::SeqCst) != generation
                        {
                            // A newer play request superseded this one.
                            continue;
                        }

                        sink.stop();
                        let cursor = Cursor::new(bytes.clone());
                        current_bytes = Some(bytes);
                        if let Ok(source) = Decoder::new(cursor) {
                            sink.append(source);
                            accumulated_time = 0.0;

                            if desired_playing_for_thread.load(Ordering::SeqCst) {
                                sink.play();
                                play_start = Some(Instant::now());
                                is_paused = false;
                            } else {
                                sink.pause();
                                play_start = None;
                                is_paused = true;
                            }
                        }

                        if generation != 0 {
                            let _ = pending_generation_for_thread.compare_exchange(
                                generation,
                                0,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            );
                        }
                    }
                    Ok(AudioCommand::PlayFailed { generation }) => {
                        let _ = pending_generation_for_thread.compare_exchange(
                            generation,
                            0,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        );
                    }
                    Ok(AudioCommand::Pause) => {
                        if let Some(start) = play_start {
                            accumulated_time += start.elapsed().as_secs_f32();
                        }
                        sink.pause();
                        is_paused = true;
                        play_start = None;
                    }
                    Ok(AudioCommand::Resume) => {
                        sink.play();
                        play_start = Some(Instant::now());
                        is_paused = false;
                    }
                    Ok(AudioCommand::Stop) => {
                        sink.stop();
                        play_start = None;
                        accumulated_time = 0.0;
                        current_bytes = None;
                    }
                    Ok(AudioCommand::SetVolume(level)) => {
                        sink.set_volume(level);
                    }
                    Ok(AudioCommand::Seek(position_secs)) => {
                        let pos = std::time::Duration::from_secs_f32(position_secs);
                        let seek_ok = sink.try_seek(pos).is_ok();
                        if seek_ok {
                            accumulated_time = position_secs;
                            if !is_paused {
                                play_start = Some(Instant::now());
                            } else {
                                play_start = None;
                            }
                        } else if let Some(ref bytes) = current_bytes {
                            // Fallback for codecs that don't support seeking (e.g. FLAC)
                            // Re-decode from stored bytes and skip to the target position
                            let vol = sink.volume();
                            sink.stop();
                            let cursor = Cursor::new(bytes.clone());
                            if let Ok(source) = Decoder::new(cursor) {
                                sink.append(source.skip_duration(pos));
                                sink.set_volume(vol);
                                if is_paused {
                                    sink.pause();
                                }
                                accumulated_time = position_secs;
                                if !is_paused {
                                    play_start = Some(Instant::now());
                                } else {
                                    play_start = None;
                                }
                            }
                        }
                    }
                    Ok(AudioCommand::GetPosition(reply)) => {
                        let pos = if is_paused {
                            accumulated_time
                        } else if let Some(start) = play_start {
                            accumulated_time + start.elapsed().as_secs_f32()
                        } else {
                            0.0
                        };
                        let _ = reply.send(pos);
                    }
                    Ok(AudioCommand::IsFinished(reply)) => {
                        let has_pending_play =
                            pending_generation_for_thread.load(Ordering::SeqCst) != 0;
                        let _ = reply.send(!has_pending_play && sink.empty());
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            sender,
            play_generation: Arc::new(AtomicU64::new(0)),
            desired_playing,
            pending_generation,
        }
    }

    #[allow(dead_code)]
    pub fn play(&self, bytes: Vec<u8>) -> Result<(), String> {
        self.play_generation.fetch_add(1, Ordering::SeqCst);
        self.desired_playing.store(true, Ordering::SeqCst);
        self.pending_generation.store(0, Ordering::SeqCst);
        self.sender
            .send(AudioCommand::Play {
                bytes,
                generation: 0,
            })
            .map_err(|e| e.to_string())
    }

    /// Start playing a track from a URL. The download happens on a background
    /// thread so this method returns almost immediately. A generation counter
    /// ensures that if a newer play request arrives while the download is
    /// in-flight the stale download is discarded.
    pub fn play_url(&self, url: String) -> Result<(), String> {
        let gen = self.play_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = Arc::clone(&self.play_generation);
        let pending_generation = Arc::clone(&self.pending_generation);
        let sender = self.sender.clone();
        self.desired_playing.store(true, Ordering::SeqCst);
        self.pending_generation.store(gen, Ordering::SeqCst);

        // Stop current playback right away so the user hears silence instead
        // of the tail of the old track while the new one downloads.
        sender.send(AudioCommand::Stop).ok();

        thread::spawn(move || {
            let response = match reqwest::blocking::get(&url) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Failed to fetch stream from '{}': {}", url, e);
                    if generation.load(Ordering::SeqCst) == gen {
                        let _ = sender.send(AudioCommand::PlayFailed { generation: gen });
                    }
                    return;
                }
            };

            let bytes = match response.bytes() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Failed to read stream bytes: {}", e);
                    if generation.load(Ordering::SeqCst) == gen {
                        let _ = sender.send(AudioCommand::PlayFailed { generation: gen });
                    }
                    return;
                }
            };

            // Only send Play if no newer request has been made in the meantime.
            if generation.load(Ordering::SeqCst) != gen {
                return;
            }

            if let Err(e) = sender.send(AudioCommand::Play {
                bytes: bytes.to_vec(),
                generation: gen,
            }) {
                eprintln!("Failed to send Play command: {}", e);
                // Best-effort cleanup if the player thread is unavailable.
                let _ = pending_generation.compare_exchange(
                    gen,
                    0,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
            }
        });

        Ok(())
    }

    pub fn pause(&self) -> Result<(), String> {
        self.desired_playing.store(false, Ordering::SeqCst);
        self.sender
            .send(AudioCommand::Pause)
            .map_err(|e| e.to_string())
    }

    pub fn resume(&self) -> Result<(), String> {
        self.desired_playing.store(true, Ordering::SeqCst);
        self.sender
            .send(AudioCommand::Resume)
            .map_err(|e| e.to_string())
    }

    pub fn stop(&self) -> Result<(), String> {
        // Invalidate pending downloads so stale Play commands are ignored.
        self.play_generation.fetch_add(1, Ordering::SeqCst);
        self.pending_generation.store(0, Ordering::SeqCst);
        self.desired_playing.store(false, Ordering::SeqCst);
        self.sender
            .send(AudioCommand::Stop)
            .map_err(|e| e.to_string())
    }

    pub fn set_volume(&self, level: f32) -> Result<(), String> {
        self.sender
            .send(AudioCommand::SetVolume(level))
            .map_err(|e| e.to_string())
    }

    pub fn seek(&self, position_secs: f32) -> Result<(), String> {
        self.sender
            .send(AudioCommand::Seek(position_secs))
            .map_err(|e| e.to_string())
    }

    pub fn get_position(&self) -> Result<f32, String> {
        let (reply_tx, reply_rx) = channel();
        self.sender
            .send(AudioCommand::GetPosition(reply_tx))
            .map_err(|e| e.to_string())?;
        reply_rx.recv().map_err(|e| e.to_string())
    }

    pub fn is_finished(&self) -> Result<bool, String> {
        let (reply_tx, reply_rx) = channel();
        self.sender
            .send(AudioCommand::IsFinished(reply_tx))
            .map_err(|e| e.to_string())?;
        reply_rx.recv().map_err(|e| e.to_string())
    }
}
