/***********************************************************************
* panel-prototype/src/main.rs
*   Prototype for development of an initial Elliott 503 operator
*   control panel with pushbottons and lamps.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-01-26  P.Kimpel
*   Original version, from ui_one prototype.
***********************************************************************/

use chrono::{DateTime, Local, Timelike};
use imgui::{im_str, Condition, StyleColor, StyleVar, Window, Ui};
use imgui::{WindowDrawList};

mod register;
use register::{Register, EmulationClock};

mod system_support;
use system_support::{System};

mod widgets;
use widgets::*;

use widgets::panel_button::PanelButton;
use widgets::panel_lamp::PanelLamp;
use widgets::register_display::RegisterDisplay;


pub struct State {
    pub power_on: bool,
    pub last_clock: f64,
    pub busy_glow: f32,
    pub no_protn: bool,
    pub plotter_manual: bool,
    pub transfer_glow: f32,
    pub air_cond: bool,
    pub error_state: bool,
    pub tag_glow: f32,
    pub type_hold_glow: f32,
    pub manual_state: bool,
    pub reset_state: bool,
    pub backing_store_parity: bool
}


fn draw_clock(draw_list: &WindowDrawList) {
    // Build the simple clock
    const CENTER_X: f32 = 590.0;
    const CENTER_Y: f32 = 180.0;
    const CENTER: [f32; 2] = [CENTER_X, CENTER_Y];

    let stamp: DateTime<Local> = Local::now();
    let hour = (stamp.hour()%12) as f32;
    let minute = stamp.minute() as f32;
    let second = stamp.second() as f32;

    draw_list.add_circle(CENTER, 42.0, GRAY_LIGHT)
                .num_segments(24)
                .thickness(2.0)
                .build();

    for h in 0..12 {
        let angle = (h as f32 / 12.0 * 360.0).to_radians();
        let (x, y) = angle.sin_cos();
        let x1 = CENTER_X + x*40.0;
        let y1 = CENTER_Y - y*40.0;
        let x2 = CENTER_X + x*42.0;
        let y2 = CENTER_Y - y*42.0;
        draw_list.add_line([x1, y1], [x2, y2], BLACK_COLOR)
                    .thickness(2.0)
                    .build();
    }

    let angle = (((hour*60.0 + minute)*60.0 + second) / 43200.0 * 360.0).to_radians();
    let (x, y) = angle.sin_cos();
    let x = CENTER_X + x*25.0;
    let y = CENTER_Y - y*25.0;
    draw_list.add_line(CENTER, [x, y], GRAY_DARK)
                .thickness(3.0)
                .build();

    let angle = ((minute*60.0 + second) / 3600.0 * 360.0).to_radians();
    let (x, y) = angle.sin_cos();
    let x = CENTER_X + x*35.0;
    let y = CENTER_Y - y*35.0;
    draw_list.add_line(CENTER, [x, y], GRAY_DARK)
                .thickness(2.0)
                .build();

    let angle = (second / 60.0 * 360.0).to_radians();
    let (x, y) = angle.sin_cos();
    let x = CENTER_X + x*40.0;
    let y = CENTER_Y - y*40.0;
    draw_list.add_line(CENTER, [x, y], RED_COLOR)
                .thickness(1.0)
                .build();

    draw_list.add_circle(CENTER, 4.0, BLACK_COLOR)
                .num_segments(8)
                .filled(true)
                .build();

}

fn main() {
    const TIMER_PERIOD: f64 = 7.2e-6;
    let eclock = EmulationClock::new(0.0);
    let mut timer: Register<u32> = Register::new(30, &eclock);
    timer.set(1234567);

    let mut state = State {
        power_on: false,
        last_clock: 0.0,
        busy_glow: 0.0,
        no_protn: false,
        plotter_manual: false,
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
    let off_btn = PanelButton {
        position: [20.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: RED_DARK,
        on_color: RED_COLOR,
        label_text: im_str!("OFF"),
        ..Default::default()
    };

    let on_btn = PanelButton {
        position: [100.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GREEN_DARK,
        on_color: GREEN_COLOR,
        label_text: im_str!("ON"),
        ..Default::default()
    };

    let busy_lamp = PanelLamp {
        position: [180.0, 40.0],
        frame_size: [60.0, 40.0],
        off_color: AMBER_DARK,
        on_color: AMBER_COLOR,
        label_text: im_str!("BUSY"),
        ..Default::default()
    };

    let initial_instructions_btn = PanelButton {
        position: [260.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GRAY_LIGHT,
        on_color: GRAY_LIGHT,
        active_color: Some(GRAY_COLOR),
        label_text: im_str!("INITIAL\nINSTRUC\nTIONS"),
        ..Default::default()
    };

    let no_protn_btn = PanelButton {
        position: [340.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GREEN_DARK,
        on_color: GREEN_COLOR,
        label_text: im_str!("NO\nPROTN"),
        ..Default::default()
    };

    let clear_btn = PanelButton {
        position: [420.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: GRAY_LIGHT,
        on_color: GRAY_LIGHT,
        active_color: Some(GRAY_COLOR),
        label_text: im_str!("CLEAR"),
        ..Default::default()
    };

    let plotter_manual_btn = PanelButton {
        position: [20.0, 40.0],
        frame_size: [60.0, 60.0],
        off_color: RED_DARK,
        on_color: RED_COLOR,
        label_text: im_str!("DIGITAL\nPLOTTER\nMANUAL"),
        ..Default::default()
    };

    // Define the panel widgets -- middle row

    let transfer_lamp = PanelLamp {
        position: [180.0, 140.0],
        frame_size: [60.0, 40.0],
        off_color: GREEN_DARK,
        on_color: GREEN_COLOR,
        label_text: im_str!("TRANSFER"),
        ..Default::default()
    };

    // Define the panel widgets -- bottom row
    let air_condition_lamp = PanelLamp {
        position: [20.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: RED_DARK,
        on_color: RED_COLOR,
        label_text: im_str!("AIR\nCONDITION"),
        ..Default::default()
    };

    let error_lamp = PanelLamp {
        position: [100.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: RED_DARK,
        on_color: RED_COLOR,
        label_text: im_str!("ERROR"),
        ..Default::default()
    };

    let tag_lamp = PanelLamp {
        position: [180.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: AMBER_DARK,
        on_color: AMBER_COLOR,
        label_text: im_str!("TAG"),
        ..Default::default()
    };

    let type_hold_lamp = PanelLamp {
        position: [260.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: AMBER_DARK,
        on_color: AMBER_COLOR,
        label_text: im_str!("TYPE\nHOLD"),
        ..Default::default()
    };

    let manual_btn = PanelButton {
        position: [340.0, 200.0],
        frame_size: [60.0, 60.0],
        off_color: RED_DARK,
        on_color: RED_COLOR,
        label_text: im_str!("MANUAL"),
        ..Default::default()
    };

    let reset_btn = PanelButton {
        position: [420.0, 200.0],
        frame_size: [60.0, 60.0],
        off_color: GREEN_DARK,
        on_color: GREEN_COLOR,
        active_color: Some(GRAY_COLOR),
        label_text: im_str!("RESET"),
        ..Default::default()
    };

    let backing_store_lamp = PanelLamp {
        position: [20.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: RED_DARK,
        on_color: RED_COLOR,
        label_text: im_str!("BACKING\nSTORE\nPARITY"),
        ..Default::default()
    };

    let demo_register = RegisterDisplay {
        position: [101.0, 14.0],
        ..Default::default()
    };

    // Start the System event loop
    system.main_loop(|_run, ui| {
        let frames = ui.frame_count();
        let clock = ui.time();
        let ticks = clock.fract() as f32;
        let phase = (ticks*2.0) as i32;
        let angle = ((clock*24.0)%360.0).to_radians();

        // Set the current font and OS-level window background color
        let our_font = ui.push_font(alt_font);
        let tw = ui.push_style_color(StyleColor::WindowBg, BG_COLOR);
        let ts = ui.push_style_var(StyleVar::WindowRounding(0.0));

        // Update the timer register since the last frame
        if state.power_on {
            let mut delta_clock = clock - state.last_clock;
            while delta_clock > 0.0 {
                timer.add(1);       // Update registers BEFORE advancing the clock
                eclock.advance(TIMER_PERIOD);   // for proper glow calculation
                delta_clock -= TIMER_PERIOD;
            }
        }

        state.last_clock = clock;

        // Create the Panel A window
        let panel_a = Window::new(im_str!("Panel A"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .title_bar(false)
            .scrollable(false)
            .position([20.0, 20.0], Condition::FirstUseEver)
            .size([500.0, 300.0], Condition::FirstUseEver);
        //panel_a = panel_a.opened(run);      // Enable clicking of the window-close icon

        // Build our Panel A window and its inner widgets in the closure
        panel_a.build(&ui, || {
            let draw_list = ui.get_window_draw_list();

            if state.power_on {
                state.busy_glow = state.busy_glow*0.84 + (ticks*2345.0).fract()*0.16;
                state.transfer_glow = state.transfer_glow*0.84 + (ticks*3456.0).fract()*0.16;
                state.tag_glow = state.tag_glow*0.84 + (ticks*4567.0).fract()*0.16;
                state.type_hold_glow = state.type_hold_glow*0.84 + (ticks*7654.0).fract()*0.16;
            }

            // Define the blinking circle
            if state.power_on && phase > 0 {
                let (x, y) = angle.sin_cos();
                let x = (230.0 + x*100.0) as f32;
                let y = (170.0 - y*100.0) as f32;
                draw_list.add_circle([x, y], 8.0, RED_COLOR)
                         .filled(true)
                         .num_segments(16)
                         .thickness(1.0)
                         .build();
            }

            if off_btn.build(&ui, !state.power_on) && state.power_on {
                state.power_on = false;
                println!("Power Off... frames={}, time={}, fps={}", frames, clock, frames as f64/clock);
                // Do the power off
                state.no_protn = false;
                state.plotter_manual = false;
                state.manual_state = false;
                state.reset_state = false;
                state.busy_glow = 0.0;
                state.transfer_glow = 0.0;
                state.tag_glow = 0.0;
                state.type_hold_glow = 0.0;
                timer.set(0);
                timer.update_glow(1.0);
            }

            if on_btn.build(&ui, state.power_on) & !state.power_on {
                state.power_on = true;
                println!("Power On... frames={}, time={}, fps={}", frames, clock, frames as f64/clock);
                // Do the power on
            }

            busy_lamp.build(&ui, state.busy_glow);

            if initial_instructions_btn.build(&ui, true) && state.power_on {
                println!("Initial Instructions...");
                // Initiate initial instructions program
            }

            if no_protn_btn.build(&ui, state.no_protn) && state.power_on {
                state.no_protn = !state.no_protn;
                println!("No Protection... {}", if state.no_protn {"On"} else {"Off"});
                // Switch the protection state
            }

            if clear_btn.build(&ui, true) && state.power_on {
                println!("Clear...");
                // Clear the system state
                timer.set(0);
            }

            transfer_lamp.build(&ui, state.transfer_glow);

            air_condition_lamp.build(&ui, if state.air_cond {1.0} else {0.0});

            error_lamp.build(&ui, if state.error_state {1.0} else {0.0});

            tag_lamp.build(&ui, state.tag_glow);

            type_hold_lamp.build(&ui, state.type_hold_glow);

            if manual_btn.build(&ui, state.manual_state) && state.power_on {
                state.manual_state = !state.manual_state;
                println!("Manual... {}", if state.manual_state {"On"} else {"Off"});
                // Change global manual peripheral state
            }

            if reset_btn.build(&ui, state.reset_state) && state.power_on {
                state.reset_state = true;
                println!("Reset... On");
            } else {
                state.reset_state = false;
            }

            // Display the ImGui metrics window (debug)
            //ui.show_metrics_window(&mut metrics_open);
        });

        // Create the Panel B window
        let panel_b = Window::new(im_str!("Panel B"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .title_bar(false)
            .scrollable(false)
            .position([540.0, 20.0], Condition::FirstUseEver)
            .size([100.0, 300.0], Condition::FirstUseEver);
        //panel_b = panel_b.opened(run);    // Enable clicking of the window-close icon

        // Build our Panel B window and its inner widgets in the closure
        panel_b.build(&ui, || {
            let draw_list = ui.get_window_draw_list();

            if plotter_manual_btn.build(&ui, state.plotter_manual) && state.power_on {
                state.plotter_manual = !state.plotter_manual;
                println!("Digital Plotter Manual... {}", if state.plotter_manual {"On"} else {"Off"});
                // Change digital plotter state
            }

            backing_store_lamp.build(&ui, if state.backing_store_parity {1.0} else {0.0});

            draw_clock(&draw_list);
        });

        // Create the Panel C window
        let panel_c = Window::new(im_str!("Panel C"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .title_bar(false)
            .scrollable(false)
            .position([20.0, 340.0], Condition::FirstUseEver)
            .size([620.0, 40.0], Condition::FirstUseEver);
        //panel_c = panel_c.opened(run);    // Enable clicking of the window-close icon

        // Build our Panel C window and its inner widgets in the closure
        panel_c.build(&ui, || {
            let glow = timer.read_glow();
            let clicks = demo_register.build(&ui, glow);
        });

        // Pop the window background and font styles
        ts.pop(&ui);
        tw.pop(&ui);
        our_font.pop(&ui);       // revert to default font
    });
}
