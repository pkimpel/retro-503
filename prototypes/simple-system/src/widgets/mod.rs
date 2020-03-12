/***********************************************************************
* panel-prototype/src/widgets/mod.rs
*   Module "widgets".
*   User interface widgets for buttons, switches, lamps, etc.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-02-07  P.Kimpel
*     Original version.
***********************************************************************/

pub mod panel_button;
pub mod panel_lamp;
pub mod register_display;

pub type Position = [f32; 2];
pub type FrameSize = [f32; 2];
pub type Color4 = [f32; 4];

pub static BG_COLOR: Color4 = [0.85490, 0.83922, 0.79608, 1.0];      // Putty
pub static RED_DARK: Color4 = [0.6, 0.0, 0.0, 1.0];
pub static RED_COLOR: Color4 = [1.0, 0.0, 0.0, 1.0];
pub static GREEN_DARK: Color4 = [0.0, 0.6, 0.0, 1.0];
pub static GREEN_COLOR: Color4 = [0.0, 1.0, 0.0, 1.0];
pub static BLACK_COLOR: Color4 = [0.0, 0.0, 0.0, 1.0];
pub static GRAY_DARK: Color4 = [0.25, 0.25, 0.25, 1.0];
pub static GRAY_COLOR: Color4 = [0.5, 0.5, 0.5, 1.0];
pub static GRAY_LIGHT: Color4 = [0.75, 0.75, 0.75, 1.0];
pub static AMBER_COLOR: Color4 = [1.0, 0.8, 0.0, 1.0];
pub static AMBER_DARK: Color4 = [0.6, 0.4, 0.0, 1.0];

pub static NEON_LEVEL: [Color4; 9] = [
    [0.2, 0.2,  0.2, 1.0],              // #333333 fully off
    [0.3, 0.25, 0.2, 1.0],
    [0.4, 0.3,  0.2, 1.0],
    [0.5, 0.35, 0.2, 1.0],
    [0.6, 0.4,  0.2, 1.0],
    [0.7, 0.45, 0.2, 1.0],
    [0.8, 0.5,  0.2, 1.0],
    [0.9, 0.55, 0.2, 1.0],
    [1.0, 0.6,  0.2, 1.0]                // #FF9933 fully on
];
