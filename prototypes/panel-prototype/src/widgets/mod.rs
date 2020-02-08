/***********************************************************************
 * panel-prototype/src/widgets/mod.rs
 *  Module "widgets".
 *  User interface widgets for buttons, switches, lamps, etc.
 ***********************************************************************
 * Modification log.
 * 2020-02-07  P.Kimpel
 *     Original version.
 **********************************************************************/

 pub mod button;
 pub mod lamp;

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
pub static AMBER_COLOR: Color4 = [1.0, 0.494, 0.0, 1.0];
pub static AMBER_DARK: Color4 = [0.6, 0.296, 0.0, 1.0];
