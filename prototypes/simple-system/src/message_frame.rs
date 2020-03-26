/***********************************************************************
* simple-server/src/message_frame.rs
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

use std::io::{Read, BufReader, Write, BufWriter};
use std::net::TcpStream;

pub const FRAME_START: [u8;2] = [0x5A, 0x5A];
pub const FRAME_END: [u8;2] = [0xA5, 0xA5];

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn frame_message(writer: &mut BufWriter<TcpStream>, code: &str, payload: &Vec<u8>) -> Result<()> {
    /* Constructs a framed message to writer from the message code and payload
    parameters */

    writer.write_all(&FRAME_START)?;
    writer.write_all(&[code.len() as u8])?;
    writer.write_all(code.as_bytes())?;

    let payload_len = payload.len();
    writer.write_all(&[(payload_len >> 8) as u8, (payload_len & 0xFF) as u8])?;
    if payload_len > 0 {
        writer.write_all(payload)?;
    }

    writer.write_all(&FRAME_END)?;
    writer.flush()?;
    Ok(())
}

pub fn unframe_message<'a> (reader: &mut BufReader<TcpStream>, buf: &'a mut Vec<u8>) ->
        Result<(&'a [u8], &'a [u8])> {
    /* Receives a message from reader into buf and unframes it, returning
    Result<(code, payload>), where code and payload are slices within buf.
    code is the message code and payload is the raw message data that will usually
    need to be deserialized by the caller */

    loop {
        let frame_len = FRAME_START.len();
        let code_x = frame_len + 1;
        reader.read_exact(&mut buf[0..code_x])?;
        if !buf.starts_with(&FRAME_START) {
            println!("unframe_message invalid frame start={:?}", &buf[0..frame_len]);
        } else {
            let code_len = buf[frame_len] as usize;
            let payload_len_x = code_x + code_len;
            let payload_x = payload_len_x + 2;
            if buf.len() < payload_x {
                buf.resize(payload_x + 32, 0);
            }

            reader.read_exact(&mut buf[code_x..payload_x])?;
            let payload_len = ((buf[payload_len_x] as usize) << 8) | buf[payload_len_x+1] as usize;
            let frame_end_x = payload_x + payload_len;
            let buf_len = frame_end_x + FRAME_END.len();
            if buf.len() < buf_len {
                buf.resize(buf_len + 32, 0);
            }

            reader.read_exact(&mut buf[payload_x..buf_len])?;
            if !buf[frame_end_x..].starts_with(&FRAME_END) {
                println!("unframe_message invalid frame end={:?}", &buf[0..buf_len]);
                return Err("Invalid frame ending sentinel".into());
            } else {
                let code = &buf[code_x..payload_len_x];
                let payload = &buf[payload_x..frame_end_x];
                return Ok((code, payload));
            }
        }
    }
}
