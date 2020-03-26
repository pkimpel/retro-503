/***********************************************************************
* simple-server/src/server.rs
*   Prototype for development of an initial Elliott 503 operator
*   control panel with pushbottons and lamps.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-03-22  P.Kimpel
*   Original version, from panel-prototype.
***********************************************************************/


use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::time::Duration;

use bincode::{serialize, deserialize};
use ctrlc;

use crate::message_frame;

mod register;
use register::{Register, FlipFlop, EmulationClock};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const SERVER_IP_ADDR: &str = "localhost:503";
const SERVER_TIMEOUT: u64 = 5;          // sec
const TIMER_PERIOD: f64 = 7.2e-6;       // sec

pub struct ServerState {
    pub last_clock: f64,
    pub eclock: Arc<EmulationClock>,
    // Push-push (toggle) button states
    pub power_on: bool,
    pub no_protn: bool,
    pub plotter_manual: bool,
    pub manual_state: bool,
    pub reset_state: bool,
    // lamp intensities
    pub transfer_glow: f32,
    pub air_cond_glow: f32,
    pub error_glow: f32,
    pub tag_glow: f32,
    pub type_hold_glow: f32,
    pub bs_parity_glow: f32,
    // Registers & Flip-flops
    pub busy_ff : FlipFlop,
    pub a_reg: Register<u32>
}

fn send_status(writer: &mut BufWriter<TcpStream>, state: &ServerState) -> Result<()> {

    message_frame::frame_message(writer, "POWER", &serialize(&state.power_on)?)?;
    message_frame::frame_message(writer, "NOPRO", &serialize(&state.no_protn)?)?;
    message_frame::frame_message(writer, "MANL", &serialize(&state.manual_state)?)?;
    message_frame::frame_message(writer, "PLTMN", &serialize(&state.plotter_manual)?)?;
    message_frame::frame_message(writer, "XFER", &serialize(&state.transfer_glow)?)?;
    message_frame::frame_message(writer, "AC", &serialize(&state.air_cond_glow)?)?;
    message_frame::frame_message(writer, "ERROR", &serialize(&state.error_glow)?)?;
    message_frame::frame_message(writer, "TAG", &serialize(&state.tag_glow)?)?;
    message_frame::frame_message(writer, "THOLD", &serialize(&state.type_hold_glow)?)?;
    message_frame::frame_message(writer, "BSPAR", &serialize(&state.bs_parity_glow)?)?;
    message_frame::frame_message(writer, "BUSY", &serialize(&state.busy_ff.read_glow())?)?;
    message_frame::frame_message(writer, "A", &serialize(&state.a_reg.read_glow())?)?;
    message_frame::frame_message(writer, "ESTAT", &Vec::new())?;
    Ok(())
}

fn panel_receiver(stream: TcpStream, _peer_addr: SocketAddr, run_flag: Arc<AtomicBool>, 
        state: Arc<Mutex<ServerState>>) -> Result<()> {

    stream.set_read_timeout(Some(Duration::from_secs(SERVER_TIMEOUT)))
          .expect("Error setting stream read timeout");

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);
    let mut buf = vec![0_u8; 256];
    let mut running = true;

    while running {
        match message_frame::unframe_message(&mut reader, &mut buf) {
            Err(e) => {
                if let Some(ie) = e.downcast_ref::<std::io::Error>() {
                    match ie.kind() {
                        std::io::ErrorKind::TimedOut | 
                        std::io::ErrorKind::WouldBlock => {
                            println!("TcpStream timeout");
                        }
                        std::io::ErrorKind::UnexpectedEof => {
                            println!("panel_receiver UnexpectedEof on TcpStream");
                            running = false;
                        }
                        _ => {return Err(e.into())}
                    }
                } else {
                    return Err(e.into());
                }
            }
            Ok((code, payload)) => {
                let mut state = state.lock().unwrap();
                match std::str::from_utf8(code) {
                    Ok("STAT") => {
                        //println!("panel_receiver STAT");
                        send_status(&mut writer, &state).expect("panel_receiver error sending status");
                    }
                    Ok("INIT") => {
                        println!("panel_receiver INIT");
                    }
                    Ok("CLEAR") => {
                        println!("panel_receiver CLEAR");
                    }
                    Ok("RESET") => {
                        println!("panel_receiver RESET");
                    }
                    Ok("MANL") => {
                        let on_off = deserialize(payload)?;
                        state.manual_state = on_off;
                        println!("panel_receiver MANL {}", on_off);
                    }
                    Ok("PLTMN") => {
                        let on_off = deserialize(payload)?;
                        state.plotter_manual = on_off;
                        println!("panel_receiver PLTMN {}", on_off);
                    }
                    Ok("NOPRO") => {
                        let on_off = deserialize(payload)?;
                        state.no_protn = on_off;
                        println!("panel_receiver NOPRO {}", on_off);
                    }
                    Ok("POWER") => {
                        let on_off = deserialize(payload)?;
                        println!("panel_receiver POWER {}", on_off);
                        change_power(&mut writer, &mut state, &on_off);
                    }
                    Ok("SHUT") => {
                        running = false;
                        println!("panel_receiver SHUT");
                    }
                    Ok(bad_code) => {
                        println!("panel_receiver unrecognized message code {}", bad_code);
                    }
                    Err(e) => {
                        println!("panel_receiver corrupt message code {:?} -- {}", code, e);
                    }
                }
            }
        }

        if !run_flag.load(Ordering::Relaxed) {
            running = false;
            message_frame::frame_message(&mut writer, "SHUT", &Vec::new())
                .expect("Error sending ShutDown on panel_receiver exit");
        }
    }

    Ok(())
}

fn change_power(mut writer: &mut BufWriter<TcpStream>, state: &mut ServerState, on_off: &bool) {

    if state.power_on ^ on_off {
        state.power_on = *on_off;
        state.manual_state = false;
        state.plotter_manual = false;
        state.no_protn = false;
        state.reset_state = false;
        state.transfer_glow = 0.0;
        state.air_cond_glow = 0.0;
        state.error_glow = 0.0;
        state.tag_glow = 0.0;
        state.type_hold_glow = 0.0;
        state.bs_parity_glow = 0.0;
        state.busy_ff.set(false);
        if *on_off {
            state.a_reg.add(7654321);
        } else {
            state.a_reg.set(0);
        }

        state.eclock.advance(1e6);
        state.a_reg.update_glow(1.0);
        state.busy_ff.update_glow(1.0);

        send_status(&mut writer, &state).expect("Error sending power status");
    }
}

fn simple_cpu(running: Arc<AtomicBool>, state: Arc<Mutex<ServerState>>) {
    
    while running.load(Ordering::Relaxed) {
        let mut state = state.lock().unwrap();

        for _ in 0..1000 {
            let count = state.a_reg.read();
            state.a_reg.add(1);
            state.busy_ff.set(count & 1 == 0);
            state.eclock.advance(TIMER_PERIOD);
        }

        thread::sleep(Duration::from_millis((TIMER_PERIOD*1e6) as u64));
    }
}

pub fn main() -> Result<()> {
    let eclock = Arc::new(EmulationClock::new(0.0));

    let mut state = ServerState {
        last_clock: 0.0,
        eclock: eclock.clone(),
        power_on: false,
        no_protn: false,
        plotter_manual: false,
        manual_state: false,
        reset_state: false,
        transfer_glow: 0.0,
        air_cond_glow: 0.0,
        error_glow: 0.0,
        tag_glow: 0.0,
        type_hold_glow: 0.0,
        bs_parity_glow: 0.0,
        busy_ff: FlipFlop::new(eclock.clone()),
        a_reg: Register::new(30, eclock.clone())
    };

    state.a_reg.set(1234567);
    let state_ref = Arc::new(Mutex::new(state));

    // Set up a shared Boolean and Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("Ctrl-C signaled");
        r.store(false, Ordering::Relaxed);
    }).expect("Error establishing Ctrl-C handler");

    // Start listening for panel connections
    let listener = TcpListener::bind(SERVER_IP_ADDR)
                   .expect("Failed to bind TcpListener");
    listener.set_nonblocking(true)
            .expect("Error setting non_blocking on TcpListener");
    println!("Listening on {}", SERVER_IP_ADDR);

    while running.load(Ordering::Relaxed) {
        // Get the next incoming TCP connection
        match listener.accept() {
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    println!("Server listener accept wait");
                    thread::sleep(Duration::from_secs(SERVER_TIMEOUT));
                } else {
                    return Err(e.into());
                }
            }
            Ok((stream, peer_addr)) => {
                println!("Connection from {}", peer_addr);
                
                // Spawn a thread to handle this connection
                stream.set_nonblocking(false)
                      .expect("Error resetting non_blocking on stream");
                let state = state_ref.clone();
                let run_flag = running.clone();
                let receiver = thread::spawn(move || {
                    panel_receiver(stream, peer_addr, run_flag, state)
                });

                // Spawn the simplistic processor
                let state = state_ref.clone();
                let run_flag = running.clone();
                let cpu = thread::spawn(move || {
                    simple_cpu(run_flag, state)
                });

                match cpu.join() {
                    Ok(_) => println!("server CPU thread terminated normally"),
                    Err(e) => println!("server CPU thread error {:?}", e)                    
                }
                match receiver.join() {
                    Ok(_) => println!("server panel_receiver thread terminated normally"),
                    Err(e) => println!("server receiver thread error {:?}", e)
                }
            }
        }
    }

    Ok(())
}
