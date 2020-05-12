/***********************************************************************
* proto-system/src/message_frame.rs
*   Procedures to frame and unframe inter-module messages.
*   Message frame format:
*       starting sentinel: [u8;2] hex 5A5A
*       length of event code: u8
*       event code: str
*       length of payload: [u8;2] ([MSB, LSB], may be zero)
*       payload: bincode serialized [u8] (omitted if length=0)
*       ending sentinel: [u8;2] hex A5A5
*
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-03-22  P.Kimpel
*   Original version.
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
    listener: TcpListener
}

impl MessageListener {

    pub async fn bind<A>(addr: A) -> Result<MessageListener>
        where A: ToSocketAddrs {
        /* Asynchronously binds to the provided socket address and creates a
        listener for that address to be used subsequently by the accept method */

        let listener = TcpListener::bind(addr).await?;
        Ok(MessageListener {
            listener
        })
    }

    pub async fn accept(&self) -> Result<MessageSocket> {
        /* Asynchronously accepts the next connection from the listener.
        Created a buffered reader and writer for the connection and returns
        the peer address */

        let (stream, _peer_addr) = self.listener.accept().await?;
        println!("MessageListener accept: socket NODELAY={}", stream.nodelay().unwrap());
        Ok(MessageSocket::new(stream).await)
    }

    pub fn bind_sync<A>(addr: A) -> Result<MessageListener>
        where A: ToSocketAddrs {
        /* Synchronously bind to a socket address and return a listener */
        task::block_on(Self::bind(addr))
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
    stream: TcpStream
}

impl MessageSocket {

    pub async fn new(stream: TcpStream) -> MessageSocket {
        /* Creates a new MessageSocket from the stream parameter */

        MessageSocket {
            stream
        }
    }

    pub async fn connect<A>(addr: A) -> Result<Self>
        where A: ToSocketAddrs {
        /* Asynchronously attempts to connect to a server at the specified
        socket address */

        let stream = TcpStream::connect(addr).await?;
        println!("MessageSocket connect: socket NODELAY={}", stream.nodelay().unwrap());
        Ok(Self::new(stream).await)
    }

    pub fn connect_sync<A>(addr: A) -> Result<Self>
        where A: ToSocketAddrs {
        /* Synchronously attempts to connect to a server at the specified
        socket address */
        task::block_on(Self::connect(addr))
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }

    pub fn sender(&self) -> MessageSender {
        /* Creates and returns a new message frame sender */
        MessageSender::new(&self.stream)
    }

    pub fn receiver(&self) -> MessageReceiver {
        /* Creates and returns a new message frame receiver */
        MessageReceiver::new(&self.stream)
    }
}


/***** MessageSender *****/

pub struct MessageSender {
    writer: BufWriter<TcpStream>
}

impl MessageSender {

    pub fn new(stream: &TcpStream) -> MessageSender {
        /* Returns a new, buffered MessageSender for the specified stream */

        MessageSender {
            writer: BufWriter::new(stream.clone())
        }
    }

    pub async fn send(&mut self, code: &str, payload: &Vec<u8>) -> Result<()> {
        /* Constructs a framed message and sends asynchronously to writer from
        the message code and payload parameters */

        // Write the starting sentinel and message code
        self.writer.write_all(&FRAME_START).await?;
        self.writer.write_all(&[code.len() as u8]).await?;
        self.writer.write_all(code.as_bytes()).await?;

        // Write the payload, if any
        let payload_len = payload.len();
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
            Result<(&'a [u8], &'a [u8])> {
        /* Asynchronously receives a message from reader into buf and unframes
        it, returning Result<(code, payload)>, where code and payload are slices
        within buf. Note that both values are raw u8 binary data: code is the
        message code that will need to be converted to UTF8 by the caller, and
        payload is the raw message data that will usually need to be
        deserialized by the caller */

        let frame_len = FRAME_START.len();
        let code_x = frame_len + 1;         // account for code-length size
        loop {
            // Read the starting sentinel and code-length bytes
            self.reader.read_exact(&mut buf[0..code_x]).await?;
            if !buf.starts_with(&FRAME_START) {
                println!("unframe_message invalid frame start={:?}", &buf[0..frame_len]);
            } else {
                let code_len = buf[frame_len] as usize;
                let payload_len_x = code_x + code_len;
                let payload_x = payload_len_x + 2;
                if buf.len() < payload_x {
                    buf.resize(payload_x + 32, 0);
                }

                // Read the code and payload-length bytes
                self.reader.read_exact(&mut buf[code_x..payload_x]).await?;
                let payload_len = ((buf[payload_len_x] as usize) << 8) | buf[payload_len_x+1] as usize;
                let frame_end_x = payload_x + payload_len;
                let buf_len = frame_end_x + FRAME_END.len();
                if buf.len() < buf_len {
                    buf.resize(buf_len + 32, 0);
                }

                // Read the payload and ending sentinel bytes
                self.reader.read_exact(&mut buf[payload_x..buf_len]).await?;
                if !buf[frame_end_x..].starts_with(&FRAME_END) {
                    println!("unframe_message invalid frame end={:?}", &buf[0..buf_len]);
                    return Err("Invalid frame ending sentinel".into());
                } else {
                    // All is copacetic: construct the sub-slices and return
                    let code = &buf[code_x..payload_len_x];
                    let payload = &buf[payload_x..frame_end_x];
                    return Ok((code, payload));
                }
            }
        }
    }

    pub fn receive_sync<'a> (&mut self, buf: &'a mut Vec<u8>) ->
            Result<(&'a [u8], &'a [u8])> {
        /* Synchronously receive one frame from the socket */
        task::block_on(self.receive(buf))
    }
}