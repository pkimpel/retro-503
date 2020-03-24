/***********************************************************************
* simple-server/src/panel.rs
*   Prototype for development of an initial Elliott 503 operator
*   control panel with pushbottons and lamps.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-03-12  P.Kimpel
*   Original version, from panel-prototype.
***********************************************************************/

use std::thread;
use std::sync::{Arc, mpsc, Mutex};
use std::io::{Read, Write};
use std::net::{TcpStream};
use bincode::{serialize, deserialize};

use chrono::{DateTime, Local, Timelike};
use imgui::{im_str, Condition, StyleColor, StyleVar, Window};

mod system_support;
use system_support::{System};

mod widgets;
use widgets::*;

use widgets::panel_button::PanelButton;
use widgets::panel_lamp::PanelLamp;
use widgets::register_display::RegisterDisplay;

const SERVER_IP_ADDR: &str = "localhost:50300";

pub const FRAME_START: [u8;2] = [0x5A, 0x5A];
pub const FRAME_END: [u8;2] = [0xA5, 0xA5];

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct PanelState {
    pub last_clock: f64,
    // Push-push (toggle) button states
    pub power_on: bool,
    pub no_protn: bool,
    pub plotter_manual: bool,
    pub manual_state: bool,
    pub reset_state: bool,
    // lamp intensities
    pub busy_glow: f32,
    pub transfer_glow: f32,
    pub air_cond_glow: f32,
    pub error_glow: f32,
    pub tag_glow: f32,
    pub type_hold_glow: f32,
    pub bs_parity_glow: f32,
    pub a_glow: Vec<f32>
}

enum Event {
    ShutDown,
    RequestStatus,
    PowerChange(bool),
    InitialInstructions,
    NoProtection(bool),
    Clear,
    Manual(bool),
    Reset,
    PlotterManual(bool)
}

fn event_sender(event_rx: mpsc::Receiver<Event>, mut writer: TcpStream) -> Result<()> {
    /* Frame and send and event message to the core server based on the value
    of event_rx. The framing envelope is defined as follows:
        starting sentinel: [u8;2] hex 5A5A
        length of event code: u8
        event code: str
        length of payload: [u8;2] ([msb, lsb] may be zero)
        payload: bincode serialized [u8] (may be omitted)
        ending sentinel: [u8;2] hex A5A5
     */
    use Event::*;

    fn build_frame(buf: &mut Vec<u8>, code: &str, payload: &[u8]) {
        buf.clear();
        buf.extend(&FRAME_START);
        buf.push(code.len() as u8);
        buf.extend(code.bytes());

        let pay_len = payload.len();
        buf.push((pay_len >> 8) as u8);
        buf.push((pay_len & 0xFF) as u8);
        if pay_len > 0 {
            buf.extend(payload);
        }

        buf.extend(&FRAME_END);
    }

    let mut buf: Vec<u8> = Vec::with_capacity(16);

    for ev in event_rx {
        match ev {
            ShutDown => {
                build_frame(&mut buf, "SHUT", &[]);
            }
            RequestStatus => {
                build_frame(&mut buf, "STAT", &[]);
            }
            PowerChange(state) => {
                build_frame(&mut buf, "POWER", &serialize(&state)?[..]);
            }
            InitialInstructions => {
                build_frame(&mut buf, "INIT", &[]);
            }
            NoProtection(state) => {
                build_frame(&mut buf, "NOPRO", &serialize(&state)?[..]);
            }
            Clear => {
                build_frame(&mut buf, "CLEAR", &[]);
            }
            Manual(state) => {
                build_frame(&mut buf, "MANL", &serialize(&state)?[..]);
            }
            Reset => {
                build_frame(&mut buf, "RESET", &[]);
            }
            PlotterManual(state) => {
                build_frame(&mut buf, "PLTMN", &serialize(&state)?[..]);
            }
        };

        writer.write_all(&buf[..]).expect("Error writing Event msg");
        if let ShutDown = ev {
            break;
        }
    }

    Ok(())
}

fn proc_receiver(mut reader: TcpStream, state: Arc<Mutex<PanelState>>) -> Result<()> {
    /* Receive and process messages from the processor task. Uses the same 
    message framing scheme as for event_sender */

    let mut buf = vec![0_u8; 256];
    let mut running = true;

    while running {
        let frame_len = FRAME_START.len();
        let code_x = frame_len + 1;
        match reader.read_exact(&mut buf[0..code_x]) {
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                println!("proc_receiver UnexpectedEof on server TcpStream");
                return Ok(())
            } 
            Err(e) => return Err(e.into())
        }
        
        if !buf[..].starts_with(&FRAME_START) {
            println!("proc_receiver invalid frame start={:?}", &buf[0..frame_len]);
        } else {
            let code_len = buf[frame_len] as usize;
            let pay_len_x = code_x + code_len;
            let payload_x = pay_len_x + 2;
            if buf.len() < payload_x {
                buf.resize(payload_x + 32, 0);
            }

            reader.read_exact(&mut buf[code_x..payload_x])?;
            let payload_len = ((buf[pay_len_x] as usize) << 8) | buf[pay_len_x+1] as usize;
            let frame_end_x = payload_x + payload_len;
            let buf_len = frame_end_x + FRAME_END.len();
            if buf.len() < buf_len {
                buf.resize(buf_len + 32, 0);
            }

            reader.read_exact(&mut buf[payload_x..buf_len])?;
            if !buf[frame_end_x..].starts_with(&FRAME_END) {
                println!("proc_receiver invalid frame end={:?}", &buf[frame_end_x..]);
            } else {
                let code = &buf[code_x..pay_len_x];
                let payload = &buf[payload_x..frame_end_x];
                let mut state = state.lock().unwrap();
                match std::str::from_utf8(code) {
                    Ok("A") => {
                        state.a_glow.clear();
                        state.a_glow.extend(deserialize::<Vec<f32>>(payload)?);
                    }
                    Ok("BUSY") => {
                        state.busy_glow = deserialize(payload)?;
                    }
                    Ok("XFER") => {
                        state.transfer_glow = deserialize(payload)?;
                    }
                    Ok("AC") => {
                        state.air_cond_glow = deserialize(payload)?;
                    }
                    Ok("ERROR") => {
                        state.error_glow = deserialize(payload)?;
                    }
                    Ok("TAG") => {
                        state.tag_glow = deserialize(payload)?;
                    }
                    Ok("THOLD") => {
                        state.type_hold_glow = deserialize(payload)?;
                    }
                    Ok("BSPAR") => {
                        state.bs_parity_glow = deserialize(payload)?;
                    }
                    Ok("NOPRO") => {
                        state.no_protn = deserialize(payload)?;
                    }
                    Ok("MANL") => {
                        state.manual_state = deserialize(payload)?;
                    }
                    Ok("PLTMN") => {
                        state.plotter_manual = deserialize(payload)?;
                    }
                    Ok("POWER") => {
                        state.power_on = deserialize(payload)?;
                    }
                    Ok("SHUT") => {
                        running = false;
                    }
                    Ok(bad_code) => {
                        println!("proc_receiver unrecognzed message code {}", bad_code);
                    }
                    Err(e) => {
                        println!("proc_receiver corrupt message code {:?} -- {}", code, e)
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn main() -> Result<()> {
    let state = PanelState {
        last_clock: 0.0,
        power_on: false,
        no_protn: false,
        plotter_manual: false,
        manual_state: false,
        reset_state: false,
        busy_glow: 0.0,
        transfer_glow: 0.0,
        air_cond_glow: 0.0,
        error_glow: 0.0,
        tag_glow: 0.0,
        type_hold_glow: 0.0,
        bs_parity_glow: 0.0,
        a_glow: vec![0.0_f32]
    };

    let state_ref = Arc::new(Mutex::new(state));

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
    let air_cond_lamp = PanelLamp {
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

    let bs_parity_lamp = PanelLamp {
        position: [20.0, 220.0],
        frame_size: [60.0, 40.0],
        off_color: RED_DARK,
        on_color: RED_COLOR,
        label_text: im_str!("BACKING\nSTORE\nPARITY"),
        ..Default::default()
    };

    let a_reg = RegisterDisplay {
        position: [101.0, 14.0],
        ..Default::default()
    };

    let (event_tx, event_rx) = mpsc::channel::<Event>();
    let stream = TcpStream::connect(SERVER_IP_ADDR)
                           .expect("Failed to connect to core server");

    // Start the communication threads
    let writer = stream.try_clone().unwrap();
    let state_dup = state_ref.clone();
    let proc_thread = thread::spawn(move || {
        proc_receiver(stream, state_dup)
    });

    let ev_thread = thread::spawn(move || {
        event_sender(event_rx, writer)
    });

    // Start the System event loop
    system.main_loop(|run, ui| {
        let frames = ui.frame_count();
        let clock = ui.time();

        // Check to see if the main window has been closed
        if !*run {
            println!("Panel main window closed");
            event_tx.send(Event::ShutDown)
                    .expect("Error sending ShutDown after main window close");
        }

        // Set the current font and OS-level window background color
        let our_font = ui.push_font(alt_font);
        let tw = ui.push_style_color(StyleColor::WindowBg, BG_COLOR);
        let ts = ui.push_style_var(StyleVar::WindowRounding(0.0));

        let mut state = state_ref.lock().unwrap();

        // Update the timer since the last frame
        if state.power_on {
            let mut delta_clock = clock - state.last_clock;
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
            busy_lamp.build(&ui, state.busy_glow);
            transfer_lamp.build(&ui, state.transfer_glow);
            air_cond_lamp.build(&ui, state.air_cond_glow);
            error_lamp.build(&ui, state.error_glow);
            tag_lamp.build(&ui, state.tag_glow);
            type_hold_lamp.build(&ui, state.type_hold_glow);

            if off_btn.build(&ui, !state.power_on) && state.power_on {
                state.power_on = false;
                println!("Power Off... frames={}, time={}, fps={}", frames, clock, frames as f64/clock);
                // Do the power off
                event_tx.send(Event::PowerChange(false)).unwrap();
                state.no_protn = false;
                state.plotter_manual = false;
                state.manual_state = false;
                state.reset_state = false;
                state.busy_glow = 0.0;
                state.transfer_glow = 0.0;
                state.air_cond_glow = 0.0;
                state.error_glow = 0.0;
                state.tag_glow = 0.0;
                state.type_hold_glow = 0.0;
                state.bs_parity_glow = 0.0;
            }

            if on_btn.build(&ui, state.power_on) & !state.power_on {
                state.power_on = true;
                println!("Power On... frames={}, time={}, fps={}", frames, clock, frames as f64/clock);
                event_tx.send(Event::PowerChange(true)).unwrap();
            }

            if initial_instructions_btn.build(&ui, true) && state.power_on {
                println!("Initial Instructions...");
                event_tx.send(Event::InitialInstructions).unwrap();
            }

            if no_protn_btn.build(&ui, state.no_protn) && state.power_on {
                state.no_protn = !state.no_protn;
                println!("No Protection... {}", if state.no_protn {"On"} else {"Off"});
                event_tx.send(Event::NoProtection(state.no_protn)).unwrap();
            }

            if clear_btn.build(&ui, true) && state.power_on {
                println!("Clear...");
                event_tx.send(Event::Clear).unwrap();
            }

            if manual_btn.build(&ui, state.manual_state) && state.power_on {
                state.manual_state = !state.manual_state;
                println!("Manual... {}", if state.manual_state {"On"} else {"Off"});
                event_tx.send(Event::Manual(state.manual_state)).unwrap();
            }

            if reset_btn.build(&ui, state.reset_state) && state.power_on {
                state.reset_state = true;
                println!("Reset... On");
                event_tx.send(Event::Reset).unwrap();
            }
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

        // Build our Panel B window and its inner widgets in the closure
        panel_b.build(&ui, || {
            bs_parity_lamp.build(&ui, state.bs_parity_glow);

            if plotter_manual_btn.build(&ui, state.plotter_manual) && state.power_on {
                state.plotter_manual = !state.plotter_manual;
                println!("Digital Plotter Manual... {}", if state.plotter_manual {"On"} else {"Off"});
                event_tx.send(Event::PlotterManual(state.plotter_manual)).unwrap();
            }
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

        // Build our Panel C window and its inner widgets in the closure
        panel_c.build(&ui, || {
            let _clicks = a_reg.build(&ui, &state.a_glow[..]);
            // ?? Need to report clicks back to the core
        });

        // Pop the window background and font styles
        ts.pop(&ui);
        tw.pop(&ui);
        our_font.pop(&ui);       // revert to default font
    });

    ev_thread.join().unwrap().unwrap();
    proc_thread.join().unwrap().unwrap();
    Ok(())
}
