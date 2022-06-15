/***********************************************************************
* proto-system/src/server.rs
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

use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

//use async_std::prelude::*;
use tokio::task::{self, JoinHandle};
//use futures::{select, FutureExt};
use tokio::sync::mpsc;
//use futures::sink::SinkExt;

//use bincode::{serialize, deserialize};
//use ctrlc;

//use crate::event::Event;
use crate::message_frame::{MessageListener, MessageSocket, MessageSender, MessageReceiver};

mod register;
//use register::{Register, FlipFlop, EmulationClock};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const SERVER_TIMEOUT: u64 = 5;          // sec
const TIMER_PERIOD: f64 = 7.2e-6;       // sec

enum BrokerEvent {
    NewClient(MessageSocket, String),
    NewConnection(MessageSocket)
}

async fn connection_identifier(socket: MessageSocket, mut broker_queue: mpsc::Sender<BrokerEvent>) {
    /* Solicits a new connection for its client ID. If the client returns one in a
    reasonable time, queues the client's socket and ID back to the broker for
    acceptance and setup. If validation is not successful, the socket is simply dropped
    upon exit from the function */
    let mut buf: Vec<u8> = Vec::with_capacity(32);
    let mut socket_sender = socket.sender();
    let mut socket_receiver = socket.receiver();

    const TIMEOUT_SECS: Duration = Duration::from_secs(2);

    // Send a "WRU" query to the new client
    let result = socket_sender.send("WRU", &Vec::new()).await;
    match result {
        Ok(_) => {}
        Err(e) => {
            println!("Error sending connection_identifier WRU: {}", e);
            return;
        }
    }

    // Wait for a response from the client or a timeout
    tokio::select! {
        r = socket_receiver.receive(&mut buf).fuse() => match r {
            Err(e) => {
                println!("Error receiving connection_identifier WRU reply: {}", e);
            }
            Ok((id, code, payload)) => {
                match str::from_utf8(code) {
                    Err(_) => {
                        println!("UTF-8 error in connection_identifier WRU code {:?}", code)
                    }
                    Ok("IAM") => {
                        if let Ok(id) = str::from_utf8(id) {
                            println!("Server IAM received from {}", id);
                            broker_queue.send(BrokerEvent::NewClient(socket, id.to_string())).await;
                        } else {
                            println!("UTF-8 error in connection_identifier WRU payload {:?}", payload);
                        }
                    }
                    Ok(_) => {
                        println!("invalid reply code in connection_identifier WRU response ")
                    }
                }
            }
        },
        t = task::sleep(TIMEOUT_SECS).fuse() => {
            println!("Timeout receiving connection_identifier WRU reply: {}", socket.peer_addr().unwrap());
        }
    };
}

async fn connection_broker(broker_queue: mpsc::Sender<BrokerEvent>, broker_receiver: mpsc::Receiver<BrokerEvent>) {
    /* Broker for managing client connections */
    let mut broker_receiver = broker_receiver.fuse();

    while let Some(ev) = broker_receiver.next().await {
        match ev {
            BrokerEvent::NewConnection(socket) => {
                task::spawn(connection_identifier(socket, broker_queue.clone()));
            }
            BrokerEvent::NewClient(socket, id) => {

            }
        }
    }
}

async fn serve(socket_addr: String, mut grim_reaper: mpsc::Receiver<()>) {
    /* Creates a MessageListener and accepts incoming connections from it.
    Runs until the sender for the grim_reaper channel is dropped */

    // Create the broker event queue channel and spawn the broker
    let (mut broker_queue, broker_receiver) = mpsc::channel::<BrokerEvent>(2000);
    let broker_handle = task::Builder::new()
        .name("Connection_Broker".to_string())
        .spawn(connection_broker(broker_queue.clone(), broker_receiver))
        .unwrap();

    // Instantiate the processor
    // let mut cpu = SimpleCPU::new(event_receiver);
    //
    // // Spawn the simplistic processor
    // let run_flag = running.clone();
    // let cpu = thread::spawn(move || {
    //     simple_cpu(run_flag, state)
    // });

    // Start listening for panel connections
    let listener = MessageListener::bind(socket_addr.as_str(), "MF").await
            .expect("Failed to bind TcpListener");
    println!("Listening on {}", socket_addr);

    // Get the next incoming TCP connection
    loop {
        tokio::select! {
            s = listener.accept().fuse() => {
                match s {
                    Err(e) => {
                        // let msg = e.message();
                        println!("Connection accept error: {}", e);
                    }
                    Ok(socket) => {
                        let peer_addr = socket.peer_addr().unwrap();
                        println!("Connection from {}", peer_addr);
                        broker_queue.send(BrokerEvent::NewConnection(socket)).await.unwrap();
                    }
                }
            }

            g = grim_reaper.next().fuse() => {
                println!("Function serve grimly reaped");
                break;
            }
        }
    }

    drop(broker_queue);
    broker_handle.await;
}

pub fn main(socket_addr: &str) -> Result<()> {

    // Set up a shared Boolean and Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));

    let (grim_sender, grim_reaper) = mpsc::channel::<()>(1);

    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("Ctrl-C signaled");
        r.store(false, Ordering::Relaxed);
        let _ = grim_sender.is_closed();
    }).expect("Error establishing Ctrl-C handler");

    task::block_on(task::Builder::new()
        .name("Server_Main".to_string())
        .spawn(serve(socket_addr.to_string(), grim_reaper))
        .unwrap()
    );

    Ok(())
}
