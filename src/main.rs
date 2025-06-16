mod engine;
mod log;
mod playback;
mod smooth;
mod strerr;

use engine::Engine;
use log::{Log, log};
use playback::Playback;
use rand::seq::SliceRandom;
use sdl2::{
    audio::AudioDevice, event::Event, keyboard::Keycode, mouse::MouseButton, pixels::Color,
    rect::Rect,
};
use smooth::{Smooth, TimedSmooth};
use std::path::{Path, PathBuf};
use strerr::Strerr;

fn main() {
    if let Err(e) = practically_main() {
        log(Log::Error, e);
    }
}

fn collect_pathbufs<P>(path: P, extension: &str) -> Result<Vec<PathBuf>, String>
where
    P: AsRef<Path>,
{
    let body = |path: PathBuf| {
        if path.is_file() {
            path.extension()
                .map(|str| str == extension)
                .and_then(|ext_matched| if ext_matched { Some(vec![path]) } else { None })
        } else if path.is_dir() {
            collect_pathbufs(path, extension).ok()
        } else {
            None
        }
    };

    Ok(std::fs::read_dir(path)
        .strerr()?
        .filter_map(|entry_result| entry_result.ok())
        .filter_map(|entry| body(entry.path()))
        .flatten()
        .collect())
}

fn practically_main() -> Result<(), String> {
    const TITLE: &str = "Suika's MP3 Player";
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 200;
    const SIZE: (u32, u32) = (WIDTH, HEIGHT);
    const HALF_HEIGHT: u32 = HEIGHT / 2;
    const SPEED_ORIGIN: i32 = WIDTH as i32 * 2 / 5;
    const VOLUME_ORIGIN: i32 = WIDTH as i32 / 4;
    const TRANSITION_DURATION: f32 = 0.3;
    const CH_LENGTH: i32 = 8;
    const CH_THICKNESS: u32 = 2;
    const COLOR_SPEED: Color = Color::RGB(0x00, 0x31, 0x61);
    const COLOR_VOLUME: Color = Color::RGB(0x00, 0x6A, 0x67);
    const WHITE: Color = Color::RGB(0xFF, 0xFF, 0xFF);
    const FONT_SIZE: u16 = 48;
    const FF_INTENSITY: f32 = 32.0;

    let files = {
        let path = std::env::args().nth(1).ok_or(
            "please provide a folder containing mp3 files, which will be scanned recursively",
        )?;

        log(Log::Info, "collecting mp3 files... this might take a while");
        let mut files = collect_pathbufs(path, "mp3")?;
        let mut rng = rand::rng();
        files.shuffle(&mut rng);
        files
    };

    log(Log::Info, format!("{} mp3 file(s) found", files.len()));

    let mut files = files.iter();
    let mut load = |engine: &mut Engine, device: Option<AudioDevice<Playback>>| {
        let pathbuf = files.next().ok_or("queue is empty")?;
        let pathstr = pathbuf
            .file_name()
            .and_then(|osstr| osstr.to_str())
            .unwrap_or("<broken ahh filename>");
        let queue = files.len();

        engine.set_title(&format!("{TITLE} | Queue: {queue} | File: {pathstr}"))?;
        engine.load_device(
            pathbuf,
            device.map(|d| d.close_and_get_callback().take_buffer()),
        )
    };

    let mut engine = Engine::new(TITLE, SIZE)?;

    let ttf_context = sdl2::ttf::init().strerr()?;
    let font = ttf_context.load_font("CascadiaMono.ttf", FONT_SIZE)?;

    let mut device = load(&mut engine, None)?;
    let mut event_pump = engine.event_pump()?;
    let mut s_speed = TimedSmooth::new(SPEED_ORIGIN as f32, TRANSITION_DURATION);
    let mut s_volume = TimedSmooth::new(VOLUME_ORIGIN as f32, TRANSITION_DURATION);
    let mut s_progress = TimedSmooth::new(0.0, TRANSITION_DURATION);
    let mut progress_prev = 0.0;
    let mut scroll = None;

    enum Which {
        Speed,
        Volume,
    }

    'main_loop: loop {
        s_speed.update();
        s_volume.update();
        s_progress.update();

        let (mouse_left, mouse_right, mouse_x, mouse_y) = {
            let mouse = sdl2::mouse::MouseState::new(&event_pump);
            (
                mouse.left(),
                mouse.right(),
                mouse.x().clamp(0, WIDTH as i32),
                mouse.y().clamp(0, HEIGHT as i32),
            )
        };

        let which = if mouse_y < HALF_HEIGHT as i32 {
            Which::Speed
        } else {
            Which::Volume
        };

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main_loop,
                Event::KeyDown {
                    keycode: Some(Keycode::SPACE),
                    ..
                } => device = load(&mut engine, Some(device))?,
                Event::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                    MouseButton::Left if !mouse_right => match which {
                        Which::Speed => s_speed.shift_set(mouse_x as f32),
                        Which::Volume => s_volume.shift_set(mouse_x as f32),
                    },
                    MouseButton::Right => match which {
                        Which::Speed => s_speed.shift_set(SPEED_ORIGIN as f32),
                        Which::Volume => s_volume.shift_set(VOLUME_ORIGIN as f32),
                    },
                    _ => (),
                },
                Event::MouseMotion { .. } if mouse_left && !mouse_right => match which {
                    Which::Speed => s_speed.shift_set(mouse_x as f32),
                    Which::Volume => s_volume.shift_set(mouse_x as f32),
                },
                Event::MouseWheel { y, .. } => scroll = Some(y),
                _ => (),
            }
        }

        let to_speed =
            FF_INTENSITY.powf((s_speed.interpolate() - SPEED_ORIGIN as f32) / WIDTH as f32);
        let to_volume = s_volume.interpolate() / WIDTH as f32;
        let surface_speed = font
            .render(&format!("{to_speed:.2}x"))
            .blended(WHITE)
            .strerr()?;
        let surface_volume = font
            .render(&format!("{:.1}%", to_volume * 100.0))
            .blended(WHITE)
            .strerr()?;

        engine.clear();

        // rectangle for speed
        engine.draw_rect(
            Rect::new(0, 0, s_speed.interpolate() as u32, HALF_HEIGHT),
            COLOR_SPEED,
        )?;

        // rectangle for volume
        engine.draw_rect(
            Rect::new(
                0,
                HALF_HEIGHT as i32,
                s_volume.interpolate() as u32,
                HALF_HEIGHT,
            ),
            COLOR_VOLUME,
        )?;

        // crosshair for speed
        engine.draw_rect(
            Rect::new(
                SPEED_ORIGIN - CH_THICKNESS as i32,
                HALF_HEIGHT as i32 - CH_LENGTH,
                CH_THICKNESS,
                CH_LENGTH as u32,
            ),
            WHITE,
        )?;
        engine.draw_rect(
            Rect::new(
                SPEED_ORIGIN - CH_LENGTH,
                HALF_HEIGHT as i32 - CH_THICKNESS as i32,
                CH_LENGTH as u32,
                CH_THICKNESS,
            ),
            WHITE,
        )?;

        // crosshair for volume
        engine.draw_rect(
            Rect::new(
                VOLUME_ORIGIN - CH_THICKNESS as i32,
                HALF_HEIGHT as i32,
                CH_THICKNESS,
                CH_LENGTH as u32,
            ),
            WHITE,
        )?;
        engine.draw_rect(
            Rect::new(
                VOLUME_ORIGIN - CH_LENGTH,
                HALF_HEIGHT as i32,
                CH_LENGTH as u32,
                CH_THICKNESS,
            ),
            WHITE,
        )?;

        engine.draw_surface(surface_speed, (0, 0))?;
        engine.draw_surface(surface_volume, (0, HALF_HEIGHT as i32))?;
        engine.draw_rect(
            Rect::new(
                0,
                0,
                (s_progress.interpolate() * WIDTH as f32) as u32,
                CH_THICKNESS,
            ),
            WHITE,
        )?;

        let (go_next, progress) = {
            let mut device = device.lock();
            device.speed.set(to_speed);
            device.volume.set(to_volume);
            if let Some(y) = scroll {
                if let Err(e) = device.skip(-y) {
                    log(Log::Warning, format!("skip failed ({e})"));
                }
            }
            scroll = None;
            (device.end, device.progress())
        };

        if progress != progress_prev {
            s_progress.shift_set(progress);
        }
        progress_prev = progress;

        if go_next {
            device = load(&mut engine, Some(device))?;
        }

        engine.present();
    }

    Ok(())
}
