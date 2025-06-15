use crate::{
    Smooth, Strerr,
    log::{Log, log},
};
use sdl2::audio::AudioCallback;
use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    codecs::{CODEC_TYPE_NULL, Decoder},
    formats::{FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    units::Time,
};

pub type Sample = f32;

pub struct Playback {
    decoder: Box<dyn Decoder>,
    format: Box<dyn FormatReader>,
    buffer: Box<[Sample]>,
    track_id: u32,
    pub sample_rate: u32,
    pub channels: usize,
    offset: usize,
    position: f64,
    total_frames: u64,
    pub speed: Smooth,
    pub volume: Smooth,
    pub end: bool,
}

impl Playback {
    const SKIP_SECS: f64 = 5.0;

    pub fn new<P>(path: P, buffer: Option<Box<[Sample]>>) -> Result<Self, String>
    where
        P: AsRef<std::path::Path>,
    {
        let src = std::fs::File::open(path).strerr()?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let probed = symphonia::default::get_probe()
            .format(
                &Default::default(),
                mss,
                &Default::default(),
                &Default::default(),
            )
            .strerr()?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("no supported audio tracks")?;

        let sample_rate = track.codec_params.sample_rate.ok_or("no sample rate")?;
        let channels = track.codec_params.channels.ok_or("no channels")?.count();
        let total_frames = track.codec_params.n_frames.ok_or("no frames")?;

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &Default::default())
            .strerr()?;

        let track_id = track.id;

        Ok(Playback {
            decoder,
            format,
            buffer: match buffer {
                Some(buffer) => buffer,
                None => Box::new([]),
            },
            track_id,
            sample_rate,
            channels,
            offset: 0,
            position: 0.0,
            total_frames,
            speed: Smooth::new(1.0),
            volume: Smooth::new(0.25),
            end: false,
        })
    }

    fn get(&mut self, index: usize) -> Result<Sample, String> {
        if self.needs_refill(index) {
            self.refill()?;
            self.get(index)
        } else if index < self.offset {
            Err("index < offset").strerr()
        } else {
            self.buffer
                .get(index - self.offset)
                .copied()
                .ok_or("unreachable")
                .strerr()
        }
    }

    fn needs_refill(&self, index: usize) -> bool {
        self.offset + self.buffer.len() <= index
    }

    fn refill(&mut self) -> Result<(), String> {
        type SymErr = symphonia::core::errors::Error;

        loop {
            let packet = self.format.next_packet().strerr()?;

            while !self.format.metadata().is_latest() {
                self.format.metadata().pop();
            }

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let frames = decoded.frames();
                    let channels = spec.channels.count();
                    let samples = frames * channels;

                    let cow = match decoded {
                        AudioBufferRef::F32(cow) => cow,
                        _ => return Err("shit").strerr(),
                    };

                    self.offset += self.buffer.len();

                    if self.buffer.len() != samples {
                        log(
                            Log::Info,
                            format!("buffer size changed ({} -> {})", self.buffer.len(), samples),
                        );

                        self.buffer = (0..samples)
                            .map(|_| 0.0)
                            .collect::<Vec<Sample>>()
                            .into_boxed_slice();
                    }

                    for frame in 0..frames {
                        for channel in 0..channels {
                            self.buffer[frame * channels + channel] = cow.chan(channel)[frame];
                        }
                    }

                    return Ok(());
                }
                Err(SymErr::IoError(e)) => log(Log::Warning, format!("IoError says \"{e}\"")),
                Err(SymErr::DecodeError(e)) => {
                    log(Log::Warning, format!("DecodeError says \"{e}\""))
                }
                Err(e) => {
                    return Err(e).strerr();
                }
            }
        }
    }

    pub const fn progress(&self) -> f32 {
        (self.position / self.total_frames as f64) as f32
    }

    pub fn take_buffer(self) -> Box<[Sample]> {
        self.buffer
    }

    fn skip_impl(&mut self, seconds: f64) -> Result<(), String> {
        self.position = (self.position + seconds * self.sample_rate as f64).max(0.0);
        self.offset = self.position as usize * self.channels;
        let seconds = (self.position / self.sample_rate as f64) as u64;

        self.format
            .seek(
                SeekMode::Coarse,
                SeekTo::Time {
                    time: Time::new(seconds, 0.0),
                    track_id: Some(self.track_id),
                },
            )
            .strerr()?;

        self.decoder.reset();

        Ok(())
    }

    pub fn skip(&mut self, mult: i32) -> Result<(), String> {
        self.skip_impl(Self::SKIP_SECS * mult as f64)
    }
}

impl AudioCallback for Playback {
    type Channel = Sample;

    fn callback(&mut self, out: &mut [Self::Channel]) {
        let channels = self.channels;
        let frames = out.len() / channels;

        if out.len() % channels != 0 {
            log(Log::Warning, "nasty ahh buffer length");
        }

        for frame in 0..frames {
            let index = (self.position as usize) * channels;
            let t = frame as f32 / frames as f32;
            self.position += self.speed.interpolate(t) as f64;

            for channel in 0..channels {
                out[frame * channels + channel] = self.get(index + channel).unwrap_or_else(|e| {
                    if !self.end {
                        log(Log::Warning, e);
                    }

                    self.end = true;
                    0.0
                }) * self.volume.interpolate(t);
            }
        }

        self.speed.stabilize();
        self.volume.stabilize();
    }
}
