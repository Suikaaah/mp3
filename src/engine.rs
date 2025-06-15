use crate::{
    playback::{Playback, Sample},
    strerr::Strerr,
};
use sdl2::{
    EventPump, Sdl,
    audio::{AudioDevice, AudioSpecDesired},
    pixels::Color,
    rect::Rect,
    render::{Canvas, TextureCreator},
    surface::Surface,
    video::{Window, WindowContext},
};

pub struct Engine {
    pub sdl_context: Sdl,
    pub canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
}

impl Engine {
    pub fn new(title: &str, (width, height): (u32, u32)) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(title, width, height)
            .position_centered()
            .build()
            .strerr()?;

        let canvas = window
            .into_canvas()
            .present_vsync()
            .accelerated()
            .build()
            .strerr()?;

        let texture_creator = canvas.texture_creator();

        Ok(Self {
            sdl_context,
            canvas,
            texture_creator,
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

    pub fn set_title(&mut self, title: &str) -> Result<(), String> {
        self.canvas.window_mut().set_title(title).strerr()
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color) -> Result<(), String> {
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(rect)
    }

    pub fn draw_surface(&mut self, surface: Surface, (x, y): (i32, i32)) -> Result<(), String> {
        let texture = surface.as_texture(&self.texture_creator).strerr()?;
        self.canvas.copy(
            &texture,
            None,
            Rect::new(x, y, surface.width(), surface.height()),
        )
    }

    pub fn load_device<P>(
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
