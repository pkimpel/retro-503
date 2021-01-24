/***********************************************************************
* panel-prototype/src/widgets/register_display.rs
*      Module "widgets::register_display".
*      Panel lamps.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*      http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-02-16  P.Kimpel
*     Original version, cloned from widgets/register_lamp.rs.
***********************************************************************/

use imgui::{im_str, ImStr, StyleColor, StyleVar, Ui};
use super::*;


pub struct RegisterDisplay<'a> {
    pub position: Position,
    pub frame_size: FrameSize,
    pub lamp_spacing: f32,
    pub colors: &'a [Color4],
    pub active_color: Color4,
    pub border_color: Color4,
    pub border_size: f32,
    pub border_rounding: f32,
    pub label_color: Color4,
    pub label_text: &'a ImStr
}
 
impl<'a> Default for RegisterDisplay<'a> {
    fn default() -> Self {
        let label_text = im_str!("");
        RegisterDisplay {
            position: [0.0, 0.0],
            frame_size: [12.0, 12.0],
            lamp_spacing: 2.0,
            colors: &super::NEON_LEVEL,
            active_color: GRAY_COLOR,
            border_color: BLACK_COLOR,
            border_size: 0.0,
            border_rounding: 6.0,
            label_color: BLACK_COLOR,
            label_text
        }
    }
}
 
impl<'a> RegisterDisplay<'a> {
    pub fn build(&self, ui: &Ui, glow: &[f32]) -> Vec<bool> {
        let t0 = ui.push_style_vars(&[
            StyleVar::FrameRounding(self.border_rounding),
            StyleVar::FrameBorderSize(self.border_size)
        ]);
         
        let t1 = ui.push_style_colors(&[
            (StyleColor::Text, self.label_color),
            (StyleColor::Border, self.border_color),
            (StyleColor::ButtonActive, self.active_color)
            ]);

        let mut clicks = Vec::<bool>::with_capacity(glow.len());
        let increment = self.frame_size[0] + self.lamp_spacing;
        let mut x = self.position[0] + increment*(glow.len()-1) as f32;
        let y = self.position[1];

        for g in glow.iter() {
            let level = (*g*(self.colors.len()-1) as f32).round() as usize;
            let color = self.colors[level];
            let t2 = ui.push_style_colors(&[
                (StyleColor::Button, color),
                (StyleColor::ButtonHovered, color)
            ]);

            ui.set_cursor_pos([x, y]);
            let clicked = ui.button(self.label_text, self.frame_size);
            clicks.push(clicked);
            
            t2.pop(&ui);
            x -= increment;
        }
         
        t1.pop(&ui);
        t0.pop(&ui);
        clicks
    }
}
 