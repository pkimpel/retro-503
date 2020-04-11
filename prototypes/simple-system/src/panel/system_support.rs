/***********************************************************************
* panel-prototype/src/system_support.rs
*   Module "system_support" for instantiation and configuration of
*   glium/glutin/winit infrastructure.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-02-06  P.Kimpel
*     Original version, largely cloned from imgui-rs crate examples.
***********************************************************************/

use glium::glutin;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontId, FontSource, FontGlyphRanges, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;

const WINDOW_WIDTH: f64 = 540.0;
const WINDOW_HEIGHT: f64 = 240.0;

pub struct System {
    pub event_loop: EventLoop<()>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
    pub alt_font: FontId
}

impl System {
    pub fn new(title: &str) -> System {
        // Strip off the last node of the program title
        let title = match title.rfind('/') {
            Some(idx) => title.split_at(idx+1).1,
            None => title
        };

        // Create the event loop, graphics context, and OS-level window
        let event_loop = EventLoop::new();
        let context = glutin::ContextBuilder::new().with_vsync(true);
        let builder = WindowBuilder::new()
            .with_title(title.to_owned())
            .with_inner_size(glutin::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));

        // Create the Display object to drive OpenGL
        let display = Display::new(builder, context, &event_loop)
                              .expect("Failed to initialize display");

        // Create and initialize the ImGui context
        let mut imgui = Context::create();
        imgui.set_ini_filename(None);

        // Initialize the OS-level window platform and attach out display object
        let mut platform = WinitPlatform::init(&mut imgui);
        {
            let glw = display.gl_window();
            let window = glw.window();
            platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);
        }

        // Get the DPI factor from Winit
        let hidpi_factor = platform.hidpi_factor();

        // Define the fonts and scale them to the DPI factor
        let font_size = (12.0*hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {   // built-in font
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            }
        ]);
        let alt_font = imgui.fonts().add_font(&[
            FontSource::TtfData {           // Roboto Regular
                data: include_bytes!("../../resources/Roboto-Regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.5,
                    glyph_ranges: FontGlyphRanges::default(),
                    ..FontConfig::default()
                }),
            }
        ]);

        imgui.io_mut().font_global_scale = (1.0/hidpi_factor) as f32;

        // Initialize and attach the glium/OpenGL renderer
        let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initalize renderer");

        // Return the new System instance with the objects just initialized
        System {
            event_loop,
            display,
            imgui,
            platform,
            renderer,
            font_size,
            alt_font
        }
    }

    pub fn main_loop<F>(self, mut run_ui: F) 
            where F: FnMut(&mut bool, &mut Ui) + 'static {

        // Fetch local references to System member fields
        let System {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;

        // Initialize the frame-rate timer
        let mut last_frame = Instant::now();

        // Run the event loop
        event_loop.run(move |event, _, control_flow| match event {
            Event::NewEvents(_) => {
                last_frame = imgui.io_mut().update_delta_time(last_frame)
            }
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();
                platform
                    .prepare_frame(imgui.io_mut(), &gl_window.window())
                    .expect("Failed to prepare frame");
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let mut ui = imgui.frame();

                let mut run = true;
                run_ui(&mut run, &mut ui);
                if !run {
                    *control_flow = ControlFlow::Exit;
                }

                let gl_window = display.gl_window();
                let mut target = display.draw();
                target.clear_color_srgb(0.0, 0.0, 0.0, 1.0);
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("Rendering failed");
                target.finish().expect("Failed to swap buffers");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            event => {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        })
    }
} // System impl
