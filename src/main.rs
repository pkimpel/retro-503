use glium::glutin::{self, Event, WindowEvent};
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontId, FontSource, FontGlyphRanges, Ui, im_str};
use imgui::{Condition, StyleColor, StyleVar, Window};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;

mod clipboard;

struct System {
    pub events_loop: glutin::EventsLoop,
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
        let events_loop = glutin::EventsLoop::new();
        let context = glutin::ContextBuilder::new().with_vsync(true);
        let builder = glutin::WindowBuilder::new()
            .with_title(title.to_owned())
            .with_dimensions(glutin::dpi::LogicalSize::new(512f64, 512f64));

        // Create the Display object to drive OpenGL    
        let display = Display::new(builder, context, &events_loop)
                              .expect("Failed to initialize display");
        
        // Create and initialize the ImGui context
        let mut imgui = Context::create();
        imgui.set_ini_filename(None);

        // Create and initialize the clipboard object (not currently used)
        if let Some(backend) = clipboard::init() {
            imgui.set_clipboard_backend(Box::new(backend));
        } else {
            eprintln!("Failed to initialize clipboard");
        }

        // Initialize the OS-level window platform and attach out display object
        let mut platform = WinitPlatform::init(&mut imgui);
        {
            let glw = display.gl_window();
            let window = glw.window();
            platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);
        }

        // Get the DPI factor from Winit
        let hidpi_factor = platform.hidpi_factor();

        // Define the fonts and scale them to the DPI factor
        let font_size = (24.0*hidpi_factor) as f32;
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
                data: include_bytes!("../resources/Roboto-Regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.75,
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
            events_loop,
            display,
            imgui,
            platform,
            renderer,
            font_size,
            alt_font
        }
    }

    pub fn main_loop<F: FnMut(&mut bool, &mut Ui)> (self, mut run_ui: F) {
        // Fetch local references to System member fields
        let System {
            mut events_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;

        // Fetch the window object
        let glw = display.gl_window();
        let window = glw.window();

        // Initialize the frame-rate timer
        let mut last_frame = Instant::now();

        // Initialize the keep-running flag
        let mut run = true;

        while run {
            // Fetch the next group of events and process them in the closure
            events_loop.poll_events(|event| {
                platform.handle_event(imgui.io_mut(), &window, &event);

                // Check if the window was closed
                if let Event::WindowEvent {event, ..} = event {
                    if let WindowEvent::CloseRequested = event {
                        run = false;
                    }
                }
            });

            // Prepare to generate the next frame
            let io = imgui.io_mut();
            platform.prepare_frame(io, &window)
                    .expect("Failed to start frame");
            last_frame = io.update_delta_time(last_frame);

            // Fetch the ImGui Ui instance and call the main_loop closure to draw the UI
            let mut ui = imgui.frame();
            run_ui(&mut run, &mut ui);

            // Render the frame and swap buffers to display it
            let mut target = display.draw();
            target.clear_color_srgb(1.0, 1.0, 1.0, 1.0);
            platform.prepare_render(&ui, &window);
            let draw_data = ui.render();
            renderer.render(&mut target, draw_data)
                    .expect("Rendering failed");
            target.finish().expect("Failed to swap buffers");
        } // end while run
    }
} // System impl


pub struct State {
    foo: bool
}


fn main() {
    let mut state = State {
        foo: false
    };

    let system = System::new(file!());
    let alt_font = system.alt_font;
    system.main_loop(|run, ui| {
        let bg_color = [0.85490, 0.83922, 0.79608, 1.0];    // Putty
        //let mut metrics_open = false;

        // Set the current font and OS-level window background color
        let _alt_font = ui.push_font(alt_font);
        let tw = ui.push_style_color(StyleColor::WindowBg, bg_color);

        // Create our UI window
        let mut window = Window::new(im_str!("UI One"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(true)
            .menu_bar(false)
            .scrollable(false)
            .position([6.0, 6.0], Condition::FirstUseEver)
            .size([500.0, 500.0], Condition::FirstUseEver);
        window = window.opened(run);    // Enable clicking of the window-close icon

        // Build our UI window and its inner widgets in the closure
        window.build(ui, || {
            // Set corner-rounding and border size for widget frames
            let t0 = ui.push_style_vars(&[
                StyleVar::FrameRounding(10.0),
                StyleVar::FrameBorderSize(4.0)
            ]);
            
            // Define the Click Me button and its click handler
            if ui.button(im_str!("Click Me"), [120.0, 80.0]) {
                state.foo = !state.foo;
                println!("Button clicked... foo={}, frames={}", state.foo, ui.frame_count());
            }

            // Define the Me Too! button and its click handler
            ui.set_cursor_pos([200.0, 150.0]);
            if ui.button(im_str!("Me Too!"), [80.0, 120.0]) {
                println!("Me too! clicked...");
            }

            // Define the Me Three button and its click handler
            ui.set_cursor_pos([350.0, 350.0]);
            let t1 = ui.push_style_color(StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
            let t2 = ui.push_style_color(StyleColor::Button, [1.0, 0.0, 0.0, 1.0]);
            let t3 = ui.push_style_color(StyleColor::Border, [0.0, 0.0, 0.0, 1.0]);
            if ui.button(im_str!("Me Three"), [100.0,100.0]) {
                println!("Me Three clicked...");
            }

            // Pop all of the style-stack tokens
            t3.pop(&ui);
            t2.pop(&ui); 
            t1.pop(&ui);
            t0.pop(&ui);         
        });

        // Pop the window background and font tokens
        tw.pop(&ui);
        _alt_font.pop(&ui);       // revert to default font

        // Display the ImGui metrics window (debug)
        //ui.show_metrics_window(&mut metrics_open);
    });
}
