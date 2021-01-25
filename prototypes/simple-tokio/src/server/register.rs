/***********************************************************************
* simple-tokio/src/server/register.rs
*   Module "register" for a generic register object with time-averaged
*   lamp-glow averaging.
* Copyright (C) 2021, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2021-01-24  P.Kimpel
*   Original version, from simple-system/src/server/register.rs.
***********************************************************************/

#![allow(unused_variables, dead_code)]     // for now...

use std::ops::*;
use std::sync::{Arc, Mutex};

pub type EmulationTick = f64;

pub const CLOCK_PERIOD: EmulationTick = 0.30e-6;

pub struct EmulationClock {
    ticks: Mutex<EmulationTick>
}

impl EmulationClock {

    pub fn new(start_time: EmulationTick) -> Self {
        EmulationClock {ticks: Mutex::new(start_time)}
    }

    pub fn inc(&self) -> EmulationTick {
        let mut ticks = self.ticks.lock().unwrap();
        *ticks += CLOCK_PERIOD;
        *ticks
    }

    pub fn advance(&self, nr_ticks: EmulationTick) -> EmulationTick {
        let mut ticks = self.ticks.lock().unwrap();
        *ticks += nr_ticks;
        *ticks
    }

    pub fn read(&self) -> EmulationTick {
        *self.ticks.lock().unwrap()
    }
} // impl Emulation Clock


pub const LAMP_PERSISTENCE: EmulationTick = 1_f64/30_f64;

pub struct Register<T> {
    bits: u8,
    clock: Arc<EmulationClock>,
    last_tick: EmulationTick,
    mask: T,
    power_mask: T,
    sign_mask: T,
    overflow: bool,
    value: T,
    glow: Vec<f32>
}

impl<T> Register<T>
where T: Copy + Eq +
         BitAnd<Output=T> + BitOr<Output=T> + BitXor<Output=T> + Not<Output=T> +
         Add<Output=T> + AddAssign + Sub<Output=T> + Mul<Output=T> +
         Shl<Output=T> + Shr<Output=T> + ShrAssign + From<u8> {

    pub fn new(bits: u8, clock:Arc<EmulationClock>) -> Self {
        let initial_tick = clock.read();
        Register {
            bits,
            clock,
            last_tick: initial_tick,
            mask: (T::from(1) << T::from(bits)) - T::from(1),
            power_mask: T::from(1) << T::from(bits),
            sign_mask: T::from(1) << T::from(bits-1),
            overflow: false,
            value: T::from(0),
            glow: vec![0_f32; bits as usize]
        }
    }

    pub fn update_glow(&mut self, beta: EmulationTick) {
        let this_tick = self.clock.read();
        let elapsed = (this_tick - self.last_tick).max(CLOCK_PERIOD);
        let alpha = (elapsed/LAMP_PERSISTENCE + beta).min(1.0) as f32;
        let alpha1 = 1.0 - alpha;
        let mut v = self.value;

        self.last_tick = this_tick;
        for g in &mut self.glow {
            let b = v & T::from(1);
            if b == T::from(0) {
                *g *= alpha1;
            } else {
                *g = *g*alpha1 + alpha;
            }

            v >>= T::from(1);
        }
    }

    pub fn read_glow(&self) -> &Vec<f32> {
        &self.glow
    }

    pub fn read(&self) -> T {
        self.value
    }

    pub fn set(&mut self, value: T) {
        self.value = value & self.mask;
        self.update_glow(0.0);
    }

    pub fn add(&mut self, value: T) {
        let augend = self.value;
        let result = augend + value;
        if (augend & self.sign_mask) == (value & self.sign_mask) {
            if (value & self.sign_mask) != (result & self.sign_mask) {
                self.overflow = true;
            }
        }

        self.value = result & self.mask;
        self.update_glow(0.0);
    }

    pub fn add_unsigned(&mut self, value: T) {
        let augend = self.value;
        let result = augend + value;
        if (value & self.power_mask) != (result & self.power_mask) {
            self.overflow = true;
        }

        self.value = result & self.mask;
        self.update_glow(0.0);
    }

    pub fn negate(&mut self) {
        self.value = self.power_mask - self.value;
        self.update_glow(0.0);
    }
} // impl Register

pub struct FlipFlop {
    clock: Arc<EmulationClock>,
    last_tick: EmulationTick,
    value: bool,
    glow: f32
}

impl FlipFlop {

    pub fn new(clock:Arc<EmulationClock>) -> Self {
        let initial_tick = clock.read();
        FlipFlop {
            clock,
            last_tick: initial_tick,
            value: false,
            glow: 0.0
        }
    }

    pub fn update_glow(&mut self, beta: EmulationTick) {
        let this_tick = self.clock.read();
        let elapsed = (this_tick - self.last_tick).max(CLOCK_PERIOD);
        let alpha = (elapsed/LAMP_PERSISTENCE + beta).min(1.0) as f32;
        let alpha1 = 1.0 - alpha;

        self.last_tick = this_tick;
        if self.value {
            self.glow = self.glow*alpha1 + alpha;
        } else {
            self.glow *= alpha1;
        }
    }

    pub fn read_glow(&self) -> &f32 {
        &self.glow
    }

    pub fn read(&self) -> bool {
        self.value
    }

    pub fn set(&mut self, value: bool) {
        self.value = value;
        self.update_glow(0.0);
    }
} // impl FlipFlop