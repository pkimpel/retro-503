/***********************************************************************
* proto-system/src/event.rs
*   Enum for inter-module event definition
*
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-04-15  P.Kimpel
*   Original version.
***********************************************************************/

pub enum Event {
    IAm,
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
