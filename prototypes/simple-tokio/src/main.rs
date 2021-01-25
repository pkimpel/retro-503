/***********************************************************************
* simple-tokio/src/main.rs
*   Prototype for development of an initial Elliott 503 operator
*   control panel with pushbottons and lamps.
* Copyright (C) 2021, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2021-01-24  P.Kimpel
*   Original version, from simple-system/src/main.rs.
***********************************************************************/

pub mod panel;
pub mod server;
pub mod message_frame;

const DEFAULT_SOCKET: &str = "127.0.0.1:503";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    let mut args = std::env::args();
    match (args.nth(1).as_ref().map(String::as_str), args.next().as_ref().map(String::as_str), args.next()) {
        (Some("panel"), None, None) => panel::main(DEFAULT_SOCKET),
        (Some("panel"), Some(addr), None) => panel::main(addr),
        (Some("server"), None, None) => server::main(DEFAULT_SOCKET),
        (Some("server"), Some(addr), None) => server::main(addr),
        _ => Err("Usage: simple-tokio panel|server [socket-addr]".into()),
    }
}
