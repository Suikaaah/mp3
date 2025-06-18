use crate::{
    log::{Log, log},
    playback::{Playback, Sample},
    strerr::Strerr,
    util,
};
use sdl2::{
    EventPump, Sdl,
    audio::{AudioDevice, AudioDeviceLockGuard, AudioSpecDesired},
    image::LoadSurface,
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    surface::Surface,
    video::{Window, WindowContext},
};
use std::path::PathBuf;

type Device = AudioDevice<Playback>;

pub struct Engine {
    pub sdl_context: Sdl,
    canvas: Canvas<Window>,
    device: Option<Device>,
    files: Vec<PathBuf>,
    title: String,
}

impl Engine {
    pub fn new(title: String, (width, height): (u32, u32)) -> Result<Self, String> {
        let files = {
            let path = std::env::args().nth(1).ok_or(
                "please provide a folder containing mp3 files, which will be scanned recursively",
            )?;

            log(Log::Info, "collecting mp3 files... this might take a while");

            let mut files = util::collect_paths(path, "mp3")?;
            let mut rng = rand::rng();

            use rand::seq::SliceRandom;

            files.shuffle(&mut rng);
            files
        };

        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(&title, width, height)
            .position_centered()
            .build()
            .strerr()?;

        let canvas = window
            .into_canvas()
            .present_vsync()
            .accelerated()
            .build()
            .strerr()?;

        Ok(Self {
            sdl_context,
            canvas,
            device: None,
            files,
            title,
        })
    }

    pub fn event_pump(&self) -> Result<EventPump, String> {
        self.sdl_context.event_pump()
    }

    pub fn clear(&mut self) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
    }

    pub fn present(&mut self) {
        self.canvas.present();
    }

    pub fn set_title(&mut self, title: impl AsRef<str>) -> Result<(), String> {
        self.canvas.window_mut().set_title(title.as_ref()).strerr()
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color) -> Result<(), String> {
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(rect)
    }

    pub fn draw_surface(
        &mut self,
        surface: &Surface,
        (x, y): (i32, i32),
        texture_creator: &TextureCreator<WindowContext>,
    ) -> Result<(), String> {
        let texture = surface.as_texture(texture_creator).strerr()?;
        self.canvas.copy(
            &texture,
            None,
            Rect::new(x, y, surface.width(), surface.height()),
        )
    }

    pub fn draw_texture_fit(
        &mut self,
        texture: &Texture,
        (width, height): (u32, u32),
    ) -> Result<(), String> {
        let query = texture.query();
        let scale = height as f32 / query.height as f32;
        let width_scaled = query.width as f32 * scale;

        self.canvas.copy(
            texture,
            None,
            Rect::new(
                width as i32 - width_scaled as i32,
                0,
                width_scaled as u32,
                height,
            ),
        )
    }

    pub fn load_next<'a>(
        &mut self,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> Result<Option<Texture<'a>>, String> {
        let path = self.files.first().ok_or("queue is empty").strerr()?;
        let texture = match path.parent().and_then(util::find_image) {
            Some(image_path) => {
                // the default format is broken, which can be resolved by converting it (dumb)
                let mut surface =
                    Surface::from_file(image_path)?.convert_format(PixelFormatEnum::RGBA8888)?;
                let (width, height) = (surface.width(), surface.height());

                surface.with_lock_mut(|buffer| {
                    for y in 0..height {
                        for x in 0..width {
                            let uv = (y * width + x) as usize * 4;
                            let mul = (x as f32 / width as f32).powi(2);

                            for c in 1..4 {
                                buffer[uv + c] = (buffer[uv + c] as f32 * mul) as u8;
                            }
                        }
                    }
                });

                Some(surface.as_texture(texture_creator).strerr()?)
            }
            None => None,
        };
        let device = self.device.take();
        let device = self.load_device(
            path,
            device.map(|d| d.close_and_get_callback().take_buffer()),
        )?;
        let queue = self.files.len() - 1;
        let pathstr = path
            .file_name()
            .and_then(|osstr| osstr.to_str())
            .unwrap_or("<broken ahh filename>");

        self.set_title(format!("{} | Queue: {queue} | File: {pathstr}", self.title))?;
        self.files.swap_remove(0);
        self.device = Some(device);

        Ok(texture)
    }

    pub fn lock_device(&mut self) -> Option<AudioDeviceLockGuard<Playback>> {
        self.device.as_mut().map(|device| device.lock())
    }

    pub fn texture_creator(&self) -> TextureCreator<WindowContext> {
        self.canvas.texture_creator()
    }

    fn load_device<P>(
        &self,
        path: P,
        buffer: Option<Box<[Sample]>>,
    ) -> Result<AudioDevice<Playback>, String>
    where
        P: AsRef<std::path::Path>,
    {
        let playback = Playback::new(path, buffer)?;
        let sample_rate = playback.sample_rate as i32;
        let channels = playback.channels as u8;
        let desired_spec = AudioSpecDesired {
            freq: Some(sample_rate),
            channels: Some(channels),
            samples: None,
        };
        let device = self
            .sdl_context
            .audio()?
            .open_playback(None, &desired_spec, |_| playback)?;

        if device.spec().freq != sample_rate {
            Err("freq mismatch").strerr()
        } else {
            device.resume();
            Ok(device)
        }
    }
}
