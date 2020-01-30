/***********************************************************************
 * panel-prototype/src/main.rs
 *     Prototype for development of an initial Elliott 503 operator
 *     control panel with pushbottons and lamps.
 ***********************************************************************
 * Modification log.
 * 2020-01-26  P.Kimpel
 *     Original version, from ui_one prototype.
 **********************************************************************/

use glium::glutin::{self, Event, WindowEvent};
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontId, FontSource, FontGlyphRanges, Ui};
use imgui::{im_str, ImStr, Condition, StyleColor, StyleVar, Window};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;

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
            .with_dimensions(glutin::dpi::LogicalSize::new(660f64, 340f64));

        // Create the Display object to drive OpenGL    
        let display = Display::new(builder, context, &events_loop)
                              .expect("Failed to initialize display");
        
        // Create and initialize the ImGui context
        let mut imgui = Context::create();
        imgui.set_ini_filename(None);

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
            target.clear_color_srgb(0.0, 0.0, 0.0, 1.0);
            platform.prepare_render(&ui, &window);
            let draw_data = ui.render();
            renderer.render(&mut target, draw_data)
                    .expect("Rendering failed");
            target.finish().expect("Failed to swap buffers");
        } // end while run
    }
} // System impl


pub type Position = [f32; 2];
pub type FrameSize = [f32; 2];
pub type Color4 = [f32; 4];

static BG_COLOR: Color4 = [0.85490, 0.83922, 0.79608, 1.0];    // Putty
static RED_DARK: Color4 = [0.6, 0.0, 0.0, 1.0];
static RED_COLOR: Color4 = [1.0, 0.0, 0.0, 1.0];
static GREEN_DARK: Color4 = [0.0, 0.6, 0.0, 1.0];
static GREEN_COLOR: Color4 = [0.0, 1.0, 0.0, 1.0];
static BLACK_COLOR: Color4 = [0.0, 0.0, 0.0, 1.0];
static GRAY_DARK: Color4 = [0.25, 0.25, 0.25, 1.0];
static GRAY_COLOR: Color4 = [0.5, 0.5, 0.5, 1.0];
static GRAY_LIGHT: Color4 = [0.75, 0.75, 0.75, 1.0];
static AMBER_COLOR: Color4 = [1.0, 0.494, 0.0, 1.0];
static AMBER_DARK: Color4 = [0.6, 0.296, 0.0, 1.0];


pub struct Button<'a> {
    is_active: bool,
    position: Position,
    frame_size: FrameSize,
    off_color: Color4,
    on_color: Color4,
    border_color: Color4,
    border_size: f32,
    border_rounding: f32,
    label_color: Color4,
    label_text: &'a ImStr
}

impl Default for Button<'_> {
    fn default<'a>() -> Self {
        let label_text = im_str!("");
        Button {
            is_active: false,
            position: [0.0, 0.0],
            frame_size: [50.0, 50.0],
            off_color: GREEN_COLOR, 
            on_color: GREEN_COLOR,
            border_color: GRAY_COLOR,
            border_size: 6.0,
            border_rounding: 1.0,
            label_color: BLACK_COLOR,
            label_text
        }
    }
}

impl Button<'_> {
    fn build(&mut self, ui: &Ui, state: bool) -> bool {
        let t0 = ui.push_style_vars(&[
            StyleVar::FrameRounding(self.border_rounding),
            StyleVar::FrameBorderSize(self.border_size)
        ]);

        let color = &if state {self.on_color} else {self.off_color};
        let t1 = ui.push_style_colors(&[
            (StyleColor::Text, self.label_color),
            (StyleColor::Border, if self.is_active {GRAY_DARK} else {self.border_color}),
            (StyleColor::Button, *color),
            (StyleColor::ButtonActive, *color),
            (StyleColor::ButtonHovered, *color)
        ]);

        ui.set_cursor_pos(self.position);
        let clicked = ui.button(self.label_text, self.frame_size);
        self.is_active = ui.is_item_active();
        
        t1.pop(&ui);
        t0.pop(&ui);
        clicked
    }
}


pub struct Lamp<'a> {
    position: Position,
    frame_size: FrameSize,
    off_color: Color4,
    on_color: Color4,
    border_color: Color4,
    border_size: f32,
    border_rounding: f32,
    label_color: Color4,
    label_text: &'a ImStr
}

impl Default for Lamp<'_> {
    fn default<'a>() -> Self {
        let label_text = im_str!("");
        Lamp {
            position: [0.0, 0.0],
            frame_size: [50.0, 50.0],
            off_color: RED_COLOR, 
            on_color: GREEN_COLOR,
            border_color: GRAY_COLOR,
            border_size: 6.0,
            border_rounding: 1.0,
            label_color: BLACK_COLOR,
            label_text
        }
    }
}

impl Lamp<'_> {
    fn build(&self, ui: &Ui, intensity: f32) {
        let t0 = ui.push_style_vars(&[
            StyleVar::FrameRounding(self.border_rounding),
            StyleVar::FrameBorderSize(self.border_size)
        ]);

        // Compute the lamp intensity
        let mut color = self.off_color.clone();
        for t in color.iter_mut().zip(self.on_color.iter()) {
            let (c, on) = t;
            *c += (*on - *c)*intensity;
        }

        let t1 = ui.push_style_colors(&[
            (StyleColor::Text, self.label_color),
            (StyleColor::Border, self.border_color),
            (StyleColor::Button, color),
            (StyleColor::ButtonActive, color),
            (StyleColor::ButtonHovered, color)
        ]);

        ui.set_cursor_pos(self.position);
        let _ = ui.button(self.label_text, self.frame_size);
        
        t1.pop(&ui);
        t0.pop(&ui);
    }
}


pub struct State {
    power_on: bool,
    busy_glow: f32,
    no_protn: bool,
    digital_plotter_manual: bool,
    transfer_glow: f32,
    air_cond: bool,
    error_state: bool,
    tag_glow: f32,
    type_hold_glow: f32,
    manual_state: bool,
    reset_state: bool,
    backing_store_parity: bool
}


fn main() {
    let mut state = State {
        power_on: false,
        busy_glow: 0.0,
        no_protn: false,
        digital_plotter_manual: false,
        transfer_glow: 0.0,
        air_cond: false,
        error_state: false,
        tag_glow: 0.0,
        type_hold_glow: 0.0,
        manual_state: false,
        reset_state: false,
        backing_store_parity: false
    };

    // Instantiate the System infrastructure and default font
    let system = System::new(file!());
    let alt_font = system.alt_font;

    // Define the panel widgets -- top row
    let mut off_btn = Button {
        position: [20.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: RED_DARK, 
        on_color: RED_COLOR,
        label_text: im_str!("OFF"),
        ..Default::default()
    };

    let mut on_btn = Button {
        position: [100.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GREEN_DARK, 
        on_color: GREEN_COLOR,
        label_text: im_str!("ON"),
        ..Default::default()
    };

    let busy_lamp = Lamp {
        position: [180.0, 40.0],
        frame_size: [60.0, 40.0],
        off_color: AMBER_DARK, 
        on_color: AMBER_COLOR,
        label_text: im_str!("BUSY"),
        ..Default::default()
    };

    let mut initial_instructions_btn = Button {
        position: [260.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GRAY_LIGHT, 
        on_color: GRAY_LIGHT,
        label_text: im_str!("INITIAL\nINSTRUC\nTIONS"),
        ..Default::default()
    };

    let mut no_protn_btn = Button {
        position: [340.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GREEN_DARK, 
        on_color: GREEN_COLOR,
        label_text: im_str!("NO\nPROTN"),
        ..Default::default()
    };

    let mut clear_btn = Button {
        position: [420.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GRAY_LIGHT, 
        on_color: GRAY_LIGHT,
        label_text: im_str!("CLEAR"),
        ..Default::default()
    };

    let mut plotter_manual_btn = Button {
        position: [540.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: RED_DARK, 
        on_color: RED_COLOR,
        label_text: im_str!("DIGITAL\nPLOTTER\nMANUAL"),
        ..Default::default()
    };

    // Define the panel widgets -- middle row

    let transfer_lamp = Lamp {
        position: [180.0, 140.0],
        frame_size: [60.0, 40.0],
        off_color: GREEN_DARK, 
        on_color: GREEN_COLOR,
        label_text: im_str!("TRANSFER"),
        ..Default::default()
    };

    // Define the panel widgets -- bottom row
    let air_condition_lamp = Lamp {
        position: [20.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: RED_DARK, 
        on_color: RED_COLOR,
        label_text: im_str!("AIR\nCONDITION"),
        ..Default::default()
    };

    let error_lamp = Lamp {
        position: [100.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: RED_DARK, 
        on_color: RED_COLOR,
        label_text: im_str!("ERROR"),
        ..Default::default()
    };

    let tag_lamp = Lamp {
        position: [180.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: AMBER_DARK, 
        on_color: AMBER_COLOR,
        label_text: im_str!("TAG"),
        ..Default::default()
    };

    let type_hold_lamp = Lamp {
        position: [260.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: AMBER_DARK, 
        on_color: AMBER_COLOR,
        label_text: im_str!("TYPE\nHOLD"),
        ..Default::default()
    };

    let mut manual_btn = Button {
        position: [340.0, 200.0],
        frame_size: [60.0, 60.0],
        off_color: RED_DARK, 
        on_color: RED_COLOR,
        label_text: im_str!("MANUAL"),
        ..Default::default()
    };

    let mut reset_btn = Button {
        position: [420.0, 200.0],
        frame_size: [60.0, 60.0],
        off_color: GREEN_DARK, 
        on_color: GREEN_COLOR,
        label_text: im_str!("RESET"),
        ..Default::default()
    };

    let backing_store_lamp = Lamp {
        position: [540.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: RED_DARK, 
        on_color: RED_COLOR,
        label_text: im_str!("BACKING\nSTORE\nPARITY"),
        ..Default::default()
    };

    // Start the System event loop
    system.main_loop(|_run, ui| {

        // Set the current font and OS-level window background color
        let our_font = ui.push_font(alt_font);
        let tw = ui.push_style_color(StyleColor::WindowBg, BG_COLOR);
        let ts = ui.push_style_var(StyleVar::WindowRounding(0.0));

        // Create our UI window
        let window = Window::new(im_str!("UI One"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .title_bar(false)
            .scrollable(false)
            .position([20.0, 20.0], Condition::FirstUseEver)
            .size([620.0, 300.0], Condition::FirstUseEver);
        //window = window.opened(run);    // Enable clicking of the window-close icon

        // Build our UI window and its inner widgets in the closure
        window.build(ui, || {
            let frames = ui.frame_count();
            let clock = ui.time();
            let ticks = clock.fract() as f32;
            let phase = (ticks*2.0) as i32;
            let angle = ((clock*6.0)%360.0).to_radians();
            let draw_list = ui.get_window_draw_list();

            if state.power_on {
                state.busy_glow = state.busy_glow*0.99 + (ticks*20.0).fract()*0.01;
                state.transfer_glow = state.transfer_glow*0.70 + (ticks*3.0).fract()*0.30;
                state.tag_glow = state.tag_glow*0.70 + (ticks*1.75).fract()*0.30;
                state.type_hold_glow = state.type_hold_glow*0.50 + (ticks*0.3).fract()*0.50;
            } else {
                state.busy_glow = 0.0;
                state.transfer_glow = 0.0;
                state.tag_glow = 0.0;
                state.type_hold_glow = 0.0;
            }

            // Define the blinking circle
            if state.power_on && phase > 0 {
                let (x, y) = angle.sin_cos();
                let x = (270.0 + x*125.0) as f32;
                let y = (170.0 - y*125.0) as f32;
                draw_list.add_circle([x, y], 8.0, if phase==0 {GREEN_COLOR} else {RED_COLOR})
                         .filled(true)
                         .num_segments(16)
                         .thickness(1.0)
                         .build();
            }
            
            if off_btn.build(&ui, !state.power_on) && state.power_on {
                println!("Power Off... frames={}, time={}, fps={}", frames, clock, frames as f64/clock);
                state.power_on = false;
                // Do the power off
                state.no_protn = false;
                state.digital_plotter_manual = false;
                state.manual_state = false;
                state.reset_state = false;
            }
            
            if on_btn.build(&ui, state.power_on) & !state.power_on {
                println!("Power On... frames={}, time={}, fps={}", frames, clock, frames as f64/clock);
                state.power_on = true;
                // Do the power on
            }

            busy_lamp.build(&ui, state.busy_glow);

            if initial_instructions_btn.build(&ui, true) && state.power_on {
                println!("Initial Instructions...");
                // Initiate initial instructions program
            }

            if no_protn_btn.build(&ui, state.no_protn) && state.power_on {
                println!("No Protection");
                state.no_protn = !state.no_protn;
                // Switch the protection state
            }

            if clear_btn.build(&ui, true) && state.power_on {
                println!("Clear");
                // Clear the system state
            }

            if plotter_manual_btn.build(&ui, state.digital_plotter_manual) && state.power_on {
                println!("Digital Plotter Manual");
                state.digital_plotter_manual = !state.digital_plotter_manual;
                // Change digital plotter state
            }

            transfer_lamp.build(&ui, state.transfer_glow);

            air_condition_lamp.build(&ui, if state.air_cond {1.0} else {0.0});

            error_lamp.build(&ui, if state.error_state {1.0} else {0.0});

            tag_lamp.build(&ui, state.tag_glow);

            type_hold_lamp.build(&ui, state.type_hold_glow);

            if manual_btn.build(&ui, state.manual_state) && state.power_on {
                println!("Manual...");
                state.manual_state = !state.manual_state;
                // Change global manual peripheral state
            }

            if reset_btn.build(&ui, state.reset_state) && state.power_on {
                println!("Reset...");
                state.reset_state = true;
            } else {
                state.reset_state = false;
            }

            backing_store_lamp.build(&ui, if state.backing_store_parity {1.0} else {0.0});

            // Draw the panel divider bar
            draw_list.add_rect([520.0, 20.0], [540.0, 320.0], BLACK_COLOR)
                     .filled(true)
                     .thickness(1.0)
                     .build();
        });

        // Pop the window background and font tokens
        ts.pop(&ui);
        tw.pop(&ui);
        our_font.pop(&ui);       // revert to default font

        // Display the ImGui metrics window (debug)
        //ui.show_metrics_window(&mut metrics_open);
    });
}
