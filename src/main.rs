use crate::cpu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::event::Event;
use std::fs::File;
use std::io::Read;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::env;
use sdl2::keyboard::Keycode;

mod font;
mod cpu;

const SCALE: u32 = 12;
const WINDOW_WIDTH  : u32 = (SCREEN_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT : u32 = (SCREEN_HEIGHT as u32) * SCALE;
const TICKS_PER_FRM : usize = 12;

fn control_map(key : Keycode) -> Option<usize>{
    match key{
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

fn draw(emu: &cpu::Processor, canvas: &mut Canvas<Window>){
    canvas.set_draw_color(Color::RGB(0,0,0));
    canvas.clear();

    let vram_buffer = emu.get_display();

    canvas.set_draw_color(Color::RGB(255,255,255));
    for (i,pixel) in vram_buffer.iter().enumerate(){
        if *pixel {
            let x = (i % SCREEN_WIDTH) as u32;
            let y = (i / SCREEN_WIDTH) as u32;

            let rect = Rect::new((x*SCALE) as i32, (y*SCALE) as i32, SCALE, SCALE);
            canvas.fill_rect(rect).unwrap();
        }
    }
    canvas.present();
}

fn main() {

    let mut args = env::args();
    args.next();
    let path = args.next().unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("CHIP-8 Emulator",WINDOW_WIDTH,WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();

    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    
    let mut chip = cpu::Processor::new();
    let mut rom  = File::open(&path).expect("Unable to open file");
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).unwrap();
    chip.load_state(&buffer);
    
    'gameloop : loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit{..} => {
                    chip.reset();
                    break 'gameloop;
                },
                Event::KeyDown{keycode : Some(key), ..} => {
                    if let Some(k) = control_map(key){
                        chip.key_input(k,true);
                    }
                },
                Event::KeyUp{keycode : Some(key), ..} => {
                    if let Some(k) = control_map(key){
                        chip.key_input(k,false);
                    }
                },
                _ => ()
            }
        }
        for _ in 0..TICKS_PER_FRM{
            chip.tick();
        }
        chip.tick_timers();
        draw(&chip,&mut canvas);
    }
}
