/***********************************************************************
* simple-tokio/src/message_frame.rs
*   Procedures to frame and unframe inter-module messages.
*   Message frame format:
*       starting sentinel: [u8;2] hex 5A5A
*       length of frame, excluding starting and ending sentinels [u8;2] (MSB, LSB)
*       length of sender_id: u8
*       sender_id: str
*       length of event_code: u8
*       event_code: str
*       length of payload: [u8;2] ([MSB, LSB], may be zero)
*       payload: bincode serialized [u8] (omitted if length=0)
*       ending sentinel: [u8;2] hex A5A5
*
* Copyright (C) 2021, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2021-01-24  P.Kimpel
*   Original version, from simple-system/src/message_frame.rs.
***********************************************************************/

use async_std::task;
use async_std::io::{self, BufReader, BufWriter};
use async_std::io::prelude::*;
use async_std::net::{TcpListener, TcpStream, SocketAddr, ToSocketAddrs};
use futures::{select, FutureExt};
use std::time::Duration;

pub const FRAME_START: [u8;2] = [0x5A, 0x5A];
pub const FRAME_END: [u8;2] = [0xA5, 0xA5];

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


/***** MessageListener *****/

pub struct MessageListener {
    my_id: String,
    listener: TcpListener
}

impl MessageListener {

    pub async fn bind<A>(addr: A, my_id: &str) -> Result<MessageListener>
        where A: ToSocketAddrs {
        /* Asynchronously binds to the provided socket address and creates a
        listener for that address to be used subsequently by the accept method */

        let listener = TcpListener::bind(addr).await?;
        Ok(MessageListener {
            my_id: my_id.to_string(),
            listener
        })
    }

    pub async fn accept(&self) -> Result<MessageSocket> {
        /* Asynchronously accepts the next connection from the listener.
        Created a buffered reader and writer for the connection and returns
        the peer address */

        let (stream, _peer_addr) = self.listener.accept().await?;
        Ok(MessageSocket::new(stream, self.my_id.as_str()).await)
    }

    pub fn bind_sync<A>(addr: A, my_id: &str) -> Result<MessageListener>
        where A: ToSocketAddrs {
        /* Synchronously bind to a socket address and return a listener */
        task::block_on(Self::bind(addr, my_id))
    }

    pub fn accept_sync(&self, timeout_secs: u64) -> Option<Result<MessageSocket>> {
        /* Synchronously accept the next connection from the listener, timing
        out and returning None if no connection is available within timeout_secs
        seconds */

        task::block_on(async {
            select! {
                s = self.accept().fuse() => {Some(s)}
                t = task::sleep(Duration::from_secs(timeout_secs)).fuse() => {None}
            }
        })
    }
}


/***** MessageSocket *****/

pub struct MessageSocket {
    my_id: String,
    stream: TcpStream
}

impl MessageSocket {

    pub async fn new(stream: TcpStream, my_id: &str) -> MessageSocket {
        /* Creates a new MessageSocket from the stream parameter */

        MessageSocket {
            my_id: my_id.to_string(),
            stream
        }
    }

    pub async fn connect<A>(addr: A, my_id: &str) -> Result<Self>
        where A: ToSocketAddrs {
        /* Asynchronously attempts to connect to a server at the specified
        socket address */

        let stream = TcpStream::connect(addr).await?;
        Ok(Self::new(stream, my_id).await)
    }

    pub fn connect_sync<A>(addr: A, my_id: &str) -> Result<Self>
        where A: ToSocketAddrs {
        /* Synchronously attempts to connect to a server at the specified
        socket address */
        task::block_on(Self::connect(addr, my_id))
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }

    pub fn sender(&self) -> MessageSender {
        /* Creates and returns a new message frame sender */
        MessageSender::new(&self.stream, self.my_id.as_str())
    }

    pub fn receiver(&self) -> MessageReceiver {
        /* Creates and returns a new message frame receiver */
        MessageReceiver::new(&self.stream)
    }
}


/***** MessageSender *****/

pub struct MessageSender {
    my_id: String,
    writer: BufWriter<TcpStream>
}

impl MessageSender {

    pub fn new(stream: &TcpStream, my_id: &str) -> MessageSender {
        /* Returns a new, buffered MessageSender for the specified stream */

        MessageSender {
            my_id: my_id.to_string(),
            writer: BufWriter::new(stream.clone())
        }
    }

    pub async fn send(&mut self, code: &str, payload: &Vec<u8>) -> Result<()> {
        /* Constructs a framed message and sends asynchronously to writer from
        the message code and payload parameters */
        let my_id_len = self.my_id.len();
        let code_len = code.len();
        let payload_len = payload.len();
        let frame_len = my_id_len + code_len + payload_len +
                2 + // bytes for frame_len
                1 + // length byte for my_id
                1 + // length byte for code
                2;  // length bytes for payload

        // Write the starting sentinel and message code
        self.writer.write_all(&FRAME_START).await?;
        self.writer.write_all(&[(frame_len >> 8) as u8, (frame_len & 0xFF) as u8]).await?;
        self.writer.write_all(&[my_id_len as u8]).await?;
        self.writer.write_all(self.my_id.as_bytes()).await?;
        self.writer.write_all(&[code.len() as u8]).await?;
        self.writer.write_all(code.as_bytes()).await?;

        // Write the payload, if any
        self.writer.write_all(&[(payload_len >> 8) as u8, (payload_len & 0xFF) as u8]).await?;
        if payload_len > 0 {
            self.writer.write_all(payload).await?;
        }

        // Write the ending sentinel and flush the buffer
        self.writer.write_all(&FRAME_END).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub fn send_sync(&mut self, code: &str, payload: &Vec<u8>) -> Result<()> {
        /* Synchronously sends a framed message to the socket */
        task::block_on(self.send(code, payload))
    }
}


/***** MessageReceiver *****/

pub struct MessageReceiver {
    reader: BufReader<TcpStream>
}

impl MessageReceiver {

    pub fn new(stream: &TcpStream) -> MessageReceiver {
        /* Returns a new, buffered MessageReceiver for the specified stream */

        MessageReceiver {
            reader: BufReader::new(stream.clone())
        }
    }

    pub async fn receive<'a> (&mut self, buf: &'a mut Vec<u8>) ->
            Result<(&'a [u8], &'a [u8], &'a [u8])> {
        /* Asynchronously receives a message from reader into buf and unframes
        it, returning Result<(id, code, payload)>, where id, code, and payload
        are slices within buf. Note that all values are raw u8 binary data: id
        is the identifier of the sender, code is the message code that will need
        to be converted to UTF8 by the caller, and payload is the raw message
        data that will usually need to be deserialized by the caller */

        let frame_len_x = FRAME_START.len();
        let id_x = frame_len_x + 2 + 1; // account for frame_length + id_length sizes
        if buf.len() < id_x {
            buf.resize(id_x + 256, 0);
        }

        loop {
            // Read the starting sentinel, frame_len and id_len bytes
            self.reader.read_exact(&mut buf[0..id_x]).await?;
            if !buf.starts_with(&FRAME_START) {
                println!("unframe_message invalid frame start={:?}", &buf[0..id_x]);
            } else {
                let frame_len = ((buf[frame_len_x] as usize) << 8) | buf[frame_len_x+1] as usize;
                let frame_end_len = FRAME_END.len();
                let total_len = frame_len + frame_len_x + frame_end_len;
                let id_len = buf[frame_len_x+2] as usize;
                let code_len_x = id_x + id_len;
                let code_x = code_len_x + 1;
                if code_x > total_len {
                    println!("unframe_msg id_len overflow {}+{} > {}", id_x, id_len, total_len);
                    return Err("Invalid frame id_len".into());
                } else {
                    if buf.len() < total_len {
                        buf.resize(total_len + 32, 0);
                    }

                    // Read the remaining bytes and ending sentinel
                    self.reader.read_exact(&mut buf[id_x..total_len]).await?;
                    let code_len = buf[code_len_x] as usize;
                    let payload_len_x = code_x + code_len;
                    let payload_x = payload_len_x + 2;
                    if payload_x > total_len {
                        println!("unframe_msg code_len overflow {}+{} > {}", code_x, code_len, total_len);
                        return Err("Invalid frame code_len".into());
                    } else {
                        let payload_len = ((buf[payload_len_x] as usize) << 8) | buf[payload_len_x+1] as usize;
                        let frame_end_x = payload_x + payload_len;
                        if frame_end_x + frame_end_len != total_len {
                            println!("unframe_msg frame_length mismatch {} : {}", frame_end_x, total_len);
                            return Err("Invalid frame payload_len".into());
                        } else {
                            if !buf[frame_end_x..].starts_with(&FRAME_END) {
                                println!("unframe_message invalid frame end={:?}", &buf[0..total_len]);
                                return Err("Invalid frame ending sentinel".into());
                            } else {
                                // All is copacetic: construct the sub-slices and return
                                let id = &buf[id_x..code_len_x];
                                let code = &buf[code_x..payload_len_x];
                                let payload = &buf[payload_x..frame_end_x];
                                return Ok((id, code, payload));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn receive_sync<'a> (&mut self, buf: &'a mut Vec<u8>) ->
            Result<(&'a [u8], &'a [u8], &'a [u8])> {
        /* Synchronously receive one frame from the socket */
        task::block_on(self.receive(buf))
    }
}