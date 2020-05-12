/***********************************************************************
* proto-system/src/panel.rs
*   Prototype for development of an initial Elliott 503 operator
*   control panel with pushbottons and lamps.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-03-12  P.Kimpel
*   Original version, from simple-system.
***********************************************************************/

use std::thread;
use std::sync::{Arc, mpsc, Mutex, atomic::{AtomicBool, Ordering}};
use bincode::{serialize, deserialize};

//use chrono::{DateTime, Local, Timelike};
use imgui::{im_str, Condition, StyleColor, StyleVar, Window, Ui};

use crate::message_frame::{MessageSocket, MessageSender, MessageReceiver};

mod system_support;
use system_support::{System};

mod widgets;
use widgets::*;

use widgets::panel_button::PanelButton;
use widgets::panel_lamp::PanelLamp;
use widgets::register_display::RegisterDisplay;

const STATUS_PERIOD: f64 = 1.0/20.0;    // sec

pub const FRAME_START: [u8;2] = [0x5A, 0x5A];
pub const FRAME_END: [u8;2] = [0xA5, 0xA5];

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct PanelState {
    pub frames: i32,
    pub clock: f64,
    pub next_status_clock: f64,
    pub status_request_count: i16,
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
    Kill,
    RequestStatus,
    PowerChange(bool),
    InitialInstructions,
    NoProtection(bool),
    Clear,
    Manual(bool),
    Reset,
    PlotterManual(bool)
}

// UI Panel A Definitions

struct PanelA<'a> {
    off_btn: PanelButton<'a>,
    on_btn: PanelButton<'a>,
    busy_lamp: PanelLamp<'a>,
    initial_btn: PanelButton<'a>,
    no_protn_btn: PanelButton<'a>,
    clear_btn: PanelButton<'a>,
    transfer_lamp: PanelLamp<'a>,
    air_cond_lamp: PanelLamp<'a>,
    error_lamp: PanelLamp<'a>,
    tag_lamp: PanelLamp<'a>,
    type_hold_lamp: PanelLamp<'a>,
    manual_btn: PanelButton<'a>,
    reset_btn: PanelButton<'a>
}

impl<'a> PanelA<'a> {
    fn define() -> Self {
        // Define the Panel A widgets

        PanelA {
            // Top row
            off_btn: PanelButton {
                position: [10.0, 10.0],
                frame_size: [60.0, 60.0],
                off_color: RED_DARK,
                on_color: RED_COLOR,
                label_text: im_str!("OFF"),
                ..Default::default()
            },
            on_btn: PanelButton {
                position: [80.0, 10.0],
                frame_size: [60.0, 60.0],
                off_color: GREEN_DARK,
                on_color: GREEN_COLOR,
                label_text: im_str!("ON"),
                ..Default::default()
            },
            busy_lamp: PanelLamp {
                position: [150.0, 10.0],
                frame_size: [60.0, 40.0],
                off_color: AMBER_DARK,
                on_color: AMBER_COLOR,
                label_text: im_str!("BUSY"),
                ..Default::default()
            },
            initial_btn: PanelButton {
                position: [220.0, 10.0],
                frame_size: [60.0, 60.0],
                off_color: GRAY_LIGHT,
                on_color: GRAY_LIGHT,
                active_color: Some(GRAY_COLOR),
                label_text: im_str!("INITIAL"),
                ..Default::default()
            },
            no_protn_btn: PanelButton {
                position: [290.0, 10.0],
                frame_size: [60.0, 60.0],
                off_color: GREEN_DARK,
                on_color: GREEN_COLOR,
                label_text: im_str!("NO\nPROTN"),
                ..Default::default()
            },
            clear_btn: PanelButton {
                position: [360.0, 10.0],
                frame_size: [60.0, 60.0],
                off_color: GRAY_LIGHT,
                on_color: GRAY_LIGHT,
                active_color: Some(GRAY_COLOR),
                label_text: im_str!("CLEAR"),
                ..Default::default()
            },

            // Middle row
            transfer_lamp: PanelLamp {
                position: [150.0, 80.0],
                frame_size: [60.0, 40.0],
                off_color: GREEN_DARK,
                on_color: GREEN_COLOR,
                label_text: im_str!("TRANSFER"),
                ..Default::default()
            },

            // Bottom row
            air_cond_lamp: PanelLamp {
                position: [10.0, 130.0],
                frame_size: [60.0, 40.0],
                off_color: RED_DARK,
                on_color: RED_COLOR,
                label_text: im_str!("AIR\nCOND"),
                ..Default::default()
            },
            error_lamp: PanelLamp {
                position: [80.0, 130.0],
                frame_size: [60.0, 40.0],
                off_color: RED_DARK,
                on_color: RED_COLOR,
                label_text: im_str!("ERROR"),
                ..Default::default()
            },
            tag_lamp: PanelLamp {
                position: [150.0, 130.0],
                frame_size: [60.0, 40.0],
                off_color: AMBER_DARK,
                on_color: AMBER_COLOR,
                label_text: im_str!("TAG"),
                ..Default::default()
            },
            type_hold_lamp: PanelLamp {
                position: [220.0, 130.0],
                frame_size: [60.0, 40.0],
                off_color: AMBER_DARK,
                on_color: AMBER_COLOR,
                label_text: im_str!("TYPE\nHOLD"),
                ..Default::default()
            },
            manual_btn: PanelButton {
                position: [290.0, 110.0],
                frame_size: [60.0, 60.0],
                off_color: RED_DARK,
                on_color: RED_COLOR,
                label_text: im_str!("MANUAL"),
                ..Default::default()
            },
            reset_btn: PanelButton {
                position: [360.0, 110.0],
                frame_size: [60.0, 60.0],
                off_color: GREEN_DARK,
                on_color: GREEN_COLOR,
                active_color: Some(GRAY_COLOR),
                label_text: im_str!("RESET"),
                ..Default::default()
            }
        }
    }

    fn build(&self, ui: &Ui, state: &PanelState, event_tx: &mpsc::Sender<Event>) {
        // Create the Panel A window

        let panel = Window::new(im_str!("Panel A"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .title_bar(false)
            .scrollable(false)
            .position([10.0, 10.0], Condition::FirstUseEver)
            .size([430.0, 180.0], Condition::FirstUseEver);

        // Build our Panel A window and its inner widgets in the closure
        panel.build(&ui, || {
            self.busy_lamp.build(&ui, state.busy_glow);
            self.transfer_lamp.build(&ui, state.transfer_glow);
            self.air_cond_lamp.build(&ui, state.air_cond_glow);
            self.error_lamp.build(&ui, state.error_glow);
            self.tag_lamp.build(&ui, state.tag_glow);
            self.type_hold_lamp.build(&ui, state.type_hold_glow);

            if self.off_btn.build(&ui, !state.power_on) && state.power_on {
                println!("Power Off... frames={}, time={}, fps={}",
                        state.frames, state.clock, state.frames as f64/state.clock);
                event_tx.send(Event::PowerChange(false)).unwrap();
            }

            if self.on_btn.build(&ui, state.power_on) && !state.power_on {
                println!("Power On... frames={}, time={}, fps={}",
                        state.frames, state.clock, state.frames as f64/state.clock);
                event_tx.send(Event::PowerChange(true)).unwrap();
                event_tx.send(Event::RequestStatus).unwrap();   // bootstrap the status mechanism
            }

            if self.initial_btn.build(&ui, true) && state.power_on {
                println!("Initial Instructions...");
                event_tx.send(Event::InitialInstructions).unwrap();
            }

            if self.no_protn_btn.build(&ui, state.no_protn) && state.power_on {
                println!("No Protection... {}", if !state.no_protn {"On"} else {"Off"});
                event_tx.send(Event::NoProtection(!state.no_protn)).unwrap();
            }

            if self.clear_btn.build(&ui, true) && state.power_on {
                println!("Clear...");
                event_tx.send(Event::Clear).unwrap();
            }

            if self.manual_btn.build(&ui, state.manual_state) && state.power_on {
                println!("Manual... {}", if !state.manual_state {"On"} else {"Off"});
                event_tx.send(Event::Manual(!state.manual_state)).unwrap();
            }

            if self.reset_btn.build(&ui, state.reset_state) && state.power_on {
                println!("Reset... On");
                event_tx.send(Event::Reset).unwrap();
            }
        });
    }
}

// UI Panel B Definitions

struct PanelB<'a> {
    plotter_manual_btn: PanelButton<'a>,
    bs_parity_lamp: PanelLamp<'a>
}

impl<'a> PanelB<'a> {
    fn define() -> Self {
        //Define the Panel B widgets

        PanelB {
            plotter_manual_btn: PanelButton {
                position: [10.0, 10.0],
                frame_size: [60.0, 60.0],
                off_color: RED_DARK,
                on_color: RED_COLOR,
                label_text: im_str!("PLOTTER\nMANUAL"),
                ..Default::default()
            },
            bs_parity_lamp: PanelLamp {
                position: [10.0, 130.0],
                frame_size: [60.0, 40.0],
                off_color: RED_DARK,
                on_color: RED_COLOR,
                label_text: im_str!("BS\nPARITY"),
                ..Default::default()
            }
        }
    }

    fn build(&self, ui: &Ui, state: &PanelState, event_tx: &mpsc::Sender<Event>) {
        // Create the Panel B window

        let panel = Window::new(im_str!("Panel B"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .title_bar(false)
            .scrollable(false)
            .position([450.0, 10.0], Condition::FirstUseEver)
            .size([80.0, 180.0], Condition::FirstUseEver);

        // Build our Panel B window and its inner widgets in the closure
        panel.build(&ui, || {
            self.bs_parity_lamp.build(&ui, state.bs_parity_glow);

            if self.plotter_manual_btn.build(&ui, state.plotter_manual) && state.power_on {
                println!("Plotter Manual... {}", if !state.plotter_manual {"On"} else {"Off"});
                event_tx.send(Event::PlotterManual(!state.plotter_manual)).unwrap();
            }
        });

    }
}

// UI Panel C Definitions

struct PanelC<'a> {
    a_reg: RegisterDisplay<'a>
}

impl<'a> PanelC<'a> {
    fn define() -> Self {
        // Define the widgets for Panel C

        PanelC {
            a_reg: RegisterDisplay {
                position: [42.0, 9.0],
                ..Default::default()
            }
       }
    }

    fn build(&self, ui: &Ui, state: &PanelState, _event_tx: &mpsc::Sender<Event>) {
        // Create the Panel C window

        let panel = Window::new(im_str!("Panel C"))
            .resizable(false)
            .scroll_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .title_bar(false)
            .scrollable(false)
            .position([10.0, 200.0], Condition::FirstUseEver)
            .size([520.0, 30.0], Condition::FirstUseEver);

        // Build our Panel C window and its inner widgets in the closure
        panel.build(&ui, || {
            let _clicks = self.a_reg.build(&ui, &state.a_glow[..]);
            // ?? Need to report clicks back to the server: event_tx.Send()
        });
    }
}

// Thread functions

fn event_sender(event_rx: mpsc::Receiver<Event>, mut sender: MessageSender) -> Result<()> {
    /* Frame and send an event message to the core server based on the value
    of event_rx */
    use Event::*;

    for ev in event_rx {
        match ev {
            RequestStatus => {
                sender.send_sync("STAT", &Vec::new())?;
            }
            PowerChange(state) => {
                sender.send_sync("POWER", &serialize(&state)?)?;
            }
            InitialInstructions => {
                sender.send_sync("INIT", &Vec::new())?;
            }
            NoProtection(state) => {
                sender.send_sync("NOPRO", &serialize(&state)?)?;
            }
            Clear => {
                sender.send_sync("CLEAR", &Vec::new())?;
            }
            Manual(state) => {
                sender.send_sync("MANL", &serialize(&state)?)?;
            }
            Reset => {
                sender.send_sync("RESET", &Vec::new())?;
            }
            PlotterManual(state) => {
                sender.send_sync("PLTMN", &serialize(&state)?)?;
            }
            ShutDown => {
                sender.send_sync("SHUT", &Vec::new())?;
                break;
            }
            Kill => {
                break;
            }
        };
    }

    Ok(())
}

fn core_receiver(mut receiver: MessageReceiver, event_tx: mpsc::Sender<Event>,
                exit_flag: Arc<AtomicBool>, state: Arc<Mutex<PanelState>>) -> Result<()> {
    /* Receive and process messages from the core server task */

    let mut buf = vec![0_u8; 256];
    let mut running = true;

    while running {
        match receiver.receive_sync(&mut buf) {
            Err(e) => {
                match e.downcast_ref::<std::io::Error>() {
                    Some(ie) => {
                        match ie.kind() {
                            std::io::ErrorKind::TimedOut |
                            std::io::ErrorKind::WouldBlock => {
                                println!("TcpStream timeout");
                            }
                            std::io::ErrorKind::UnexpectedEof => {
                                println!("panel_receiver UnexpectedEof on TcpStream");
                                running = false;
                                match event_tx.send(Event::Kill) {
                                    Ok(_) => {}
                                    Err(_) => println!("panel_receiver unable to send internal Kill")
                                }
                            }
                            _ => {return Err(e.into())}
                        }
                    }
                    None => {
                        return Err(e)
                    }
                }
            }
            Ok((code, payload)) => {
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
                    Ok("RESET") => {
                        state.reset_state = deserialize(payload)?;
                    }
                    Ok("POWER") => {
                        let old_power = state.power_on;
                        state.power_on = deserialize(payload)?;
                        if state.power_on && !old_power {
                            state.status_request_count = 0;
                            state.next_status_clock = state.clock;
                        }
                    }
                    Ok("ESTAT") => {
                        //println!("Received Server status");
                        if state.status_request_count > 0 {
                            state.status_request_count -= 1;
                        }
                    }
                    Ok("KILL") => {
                        println!("Received KILL from Server");
                        running = false;
                        state.power_on = false;
                        event_tx.send(Event::Kill)?;
                    }
                    Ok(bad_code) => {
                        println!("core_receiver unrecognzed message code {}", bad_code);
                    }
                    Err(e) => {
                        println!("core_receiver corrupt message code {:?} -- {}", code, e)
                    }
                }
            }
        }
    }

    exit_flag.store(true, Ordering::Relaxed);
    Ok(())
}

pub fn main(server_addr: &str) -> Result<()> {

    // Define the UI

    let panel_a = PanelA::define();
    let panel_b = PanelB::define();
    let panel_c = PanelC::define();

    // Create the internal panel-state structure

    let state = Arc::new(Mutex::new(PanelState {
        frames: 0,
        clock: 0.0,
        next_status_clock: 0.0,
        status_request_count: 0,
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
    }));

    // Create the internal event channel and TCP connection

    let (event_tx, event_rx) = mpsc::channel::<Event>();
    let socket = MessageSocket::connect_sync(server_addr)
            .expect(&format!("Failed to connect to core server on {}", server_addr)[..]);
    println!("Connected to {} on {}", socket.peer_addr().unwrap(), socket.local_addr().unwrap());

    // Start the communication threads

    let sender = socket.sender();
    let receiver = socket.receiver();

    let exit_flag = Arc::new(AtomicBool::new(false));
    let event_tx_dup = event_tx.clone();
    let exit_flag_dup = exit_flag.clone();
    let state_dup = state.clone();

    let core_thread = thread::spawn(move || {
        core_receiver(receiver, event_tx_dup, exit_flag_dup, state_dup)
    });

    let ev_thread = thread::spawn(move || {
        event_sender(event_rx, sender)
    });

    event_tx.send(Event::RequestStatus).unwrap();   // request initial server status

    // Instantiate the System infrastructure and default font

    let system = System::new(file!());
    let alt_font = system.alt_font;

    // Start the System event loop

    system.main_loop(move |run, ui| {

        // Check to see if the main window has been closed
        if !*run {
            println!("Panel main window closed");
            match event_tx.send(Event::ShutDown) {
                Ok(_) => {}
                Err(_) => println!("Unable to send ShutDown after main window close")
            }
            return;
        }

        // Check to see if the server has shut down
        if exit_flag.load(Ordering::Relaxed) {
            println!("Forcing UI to close");
            *run = false;           // terminate the UI rendering loop
            return;
        }

        // Set the current font and OS-level window background color
        let our_font = ui.push_font(alt_font);
        let tw = ui.push_style_color(StyleColor::WindowBg, BG_COLOR);
        let ts = ui.push_style_var(StyleVar::WindowRounding(0.0));

        // Generate the UI frame
        let mut state = state.lock().unwrap();
        state.frames = ui.frame_count();
        state.clock = ui.time();

        panel_a.build(&ui, &state, &event_tx);
        panel_b.build(&ui, &state, &event_tx);
        panel_c.build(&ui, &state, &event_tx);

        // Check if it's time for the next server status request
        if state.power_on {
            if state.next_status_clock < state.clock {
                if state.status_request_count <= 2 {
                    //println!("Requesting Server status");
                    event_tx.send(Event::RequestStatus).unwrap();
                    state.next_status_clock = state.clock + STATUS_PERIOD;
                }
            }
        }

        // Pop the window background and font styles
        ts.pop(&ui);
        tw.pop(&ui);
        our_font.pop(&ui);       // revert to default font
    });

    ev_thread.join().unwrap().unwrap();
    core_thread.join().unwrap().unwrap();
    Ok(())
}
