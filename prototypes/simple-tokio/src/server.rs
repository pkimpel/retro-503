/***********************************************************************
* simple-tokio/src/server.rs
*   Prototype for development of an initial Elliott 503 operator
*   control panel with pushbottons and lamps.
* Copyright (C) 2021, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2021-01-24  P.Kimpel
*   Original version, from simple-system/src/server.rs.
***********************************************************************/


use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bincode::{serialize, deserialize};
use ctrlc;

use crate::message_frame::{MessageListener, MessageSender, MessageReceiver};

mod register;
use register::{Register, FlipFlop, EmulationClock};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const SERVER_TIMEOUT: u64 = 5;          // sec
const TIMER_PERIOD: f64 = 7.2e-6;       // sec

pub struct ServerState {
    pub last_clock: f64,
    pub eclock: Arc<EmulationClock>,
    pub reset_countdown: u32,
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

fn send_status(sender: &mut MessageSender, state: &ServerState) -> Result<()> {

    sender.send_sync("POWER", &serialize(&state.power_on)?)?;
    sender.send_sync("NOPRO", &serialize(&state.no_protn)?)?;
    sender.send_sync("MANL", &serialize(&state.manual_state)?)?;
    sender.send_sync("RESET", &serialize(&state.reset_state)?)?;
    sender.send_sync("PLTMN", &serialize(&state.plotter_manual)?)?;
    sender.send_sync("XFER", &serialize(&state.transfer_glow)?)?;
    sender.send_sync("AC", &serialize(&state.air_cond_glow)?)?;
    sender.send_sync("ERROR", &serialize(&state.error_glow)?)?;
    sender.send_sync("TAG", &serialize(&state.tag_glow)?)?;
    sender.send_sync("THOLD", &serialize(&state.type_hold_glow)?)?;
    sender.send_sync("BSPAR", &serialize(&state.bs_parity_glow)?)?;
    sender.send_sync("BUSY", &serialize(&state.busy_ff.read_glow())?)?;
    sender.send_sync("A", &serialize(&state.a_reg.read_glow())?)?;
    sender.send_sync("ESTAT", &Vec::new())?;
    Ok(())
}

fn panel_receiver(mut receiver: MessageReceiver, mut sender: MessageSender,
        run_flag: Arc<AtomicBool>, state: Arc<Mutex<ServerState>>) -> Result<()> {

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
                                println!("receiver UnexpectedEof on TcpStream");
                                running = false;
                            }
                            _ => {return Err(e.into())}
                        }
                    }
                    None => {
                        return Err(e.into());
                    }
                }
            }
            Ok((id, code, payload)) => {
                let mut state = state.lock().unwrap();
                match std::str::from_utf8(code) {
                    Ok("STAT") => {
                        //println!("receiver STAT");
                        if state.reset_countdown > 0 {
                            state.reset_countdown -= 1;
                            if state.reset_countdown == 0 {
                                state.reset_state = false;
                            }
                        }
                        send_status(&mut sender, &state).expect("receiver error sending status");
                    }
                    Ok("INIT") => {
                        println!("receiver INIT from {}", String::from_utf8_lossy(id));
                    }
                    Ok("CLEAR") => {
                        println!("receiver CLEAR");
                        state.a_reg.set(0);
                    }
                    Ok("RESET") => {
                        println!("receiver RESET");
                        state.reset_state = true;
                        state.reset_countdown = 15;
                    }
                    Ok("MANL") => {
                        let on_off = deserialize(payload)?;
                        println!("receiver MANL {}", on_off);
                        state.manual_state = on_off;
                    }
                    Ok("PLTMN") => {
                        let on_off = deserialize(payload)?;
                        println!("receiver PLTMN {}", on_off);
                        state.plotter_manual = on_off;
                    }
                    Ok("NOPRO") => {
                        let on_off = deserialize(payload)?;
                        println!("receiver NOPRO {}", on_off);
                        state.no_protn = on_off;
                    }
                    Ok("POWER") => {
                        let on_off = deserialize(payload)?;
                        println!("receiver POWER {}", on_off);
                        change_power(&mut sender, &mut state, &on_off);
                    }
                    Ok("SHUT") => {
                        running = false;
                        println!("receiver SHUT from {}", String::from_utf8_lossy(id));
                    }
                    Ok(bad_code) => {
                        println!("receiver unrecognized message code {}", bad_code);
                    }
                    Err(e) => {
                        println!("receiver corrupt message code {:?} -- {}", code, e);
                    }
                }
            }
        }

        if !run_flag.load(Ordering::Relaxed) {
            running = false;
            sender.send_sync("KILL", &Vec::new())
                    .expect("Error sending KILL on receiver exit");
            }
    }

    Ok(())
}

fn change_power(mut sender: &mut MessageSender, state: &mut ServerState, on_off: &bool) {

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

        send_status(&mut sender, &state).expect("Error sending power status");
    }
}

fn simple_cpu(running: Arc<AtomicBool>, state: Arc<Mutex<ServerState>>) {

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        } else {
            let mut state = state.lock().unwrap();
            if !state.power_on {
                drop(state);
                thread::sleep(Duration::from_secs(2));
            } else {
                for _ in 0..500 {
                    let count = state.a_reg.read();
                    state.a_reg.add(1);
                    state.busy_ff.set(count & 1 == 0);
                    state.eclock.advance(TIMER_PERIOD);
                }

                drop(state);
                thread::sleep(Duration::from_millis((TIMER_PERIOD*1e6) as u64));
            }
        }
    }
}

pub fn main(socket_addr: &str) -> Result<()> {
    let eclock = Arc::new(EmulationClock::new(0.0));

    let mut state = ServerState {
        last_clock: 0.0,
        eclock: eclock.clone(),
        reset_countdown: 0,
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
    println!("Listening on {}", socket_addr);
    let listener = MessageListener::bind_sync(socket_addr, "MF")
                   .expect("Failed to bind TcpListener");

    // Spawn the simplistic processor
    let state = state_ref.clone();
    let run_flag = running.clone();
    let cpu = thread::spawn(move || {
        simple_cpu(run_flag, state)
    });

    // Get the next incoming TCP connection
    while running.load(Ordering::Relaxed) {
        match listener.accept_sync(SERVER_TIMEOUT) {
            Some(r) => {
                match r {
                    Err(e) => {
                        return Err(e.into());
                    }
                    Ok(socket) => {
                        println!("Connection from {}", socket.peer_addr()?);

                        // Spawn a thread to handle this connection
                        let state = state_ref.clone();
                        let run_flag = running.clone();
                        let r = socket.receiver();
                        let s = socket.sender();
                        let receiver = thread::spawn(move || {
                            panel_receiver(r, s, run_flag, state)
                        });

                        match receiver.join() {
                            Ok(_) => println!("server receiver thread terminated normally"),
                            Err(e) => println!("server receiver thread error {:?}", e)
                        }
                    }
                }
            }
            None => continue            // accept timed out
        }
    }

    match cpu.join() {
        Ok(_) => println!("server CPU thread terminated normally"),
        Err(e) => println!("server CPU thread error {:?}", e)
    }
    Ok(())
}
