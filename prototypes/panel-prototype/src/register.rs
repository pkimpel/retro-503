/***********************************************************************
 * panel-prototype/src/register.rs
 *      Module "register" for a generic register object with time-averaged
 *      lamp-glow averaging.
 ***********************************************************************
 * Modification log.
 * 2020-02-09  P.Kimpel
 *     Original version.
 **********************************************************************/

use std::ops::*;
use std::cell::Cell;

pub type EmulationTick = f64;

pub const CLOCK_PERIOD: EmulationTick = 0.30e-6;

pub struct EmulationClock {
    ticks: Cell<EmulationTick>
}

impl EmulationClock {

    pub fn new(start_time: EmulationTick) -> Self {
        EmulationClock {ticks: Cell::new(start_time)}
    }

    pub fn inc(&self) -> EmulationTick {
        let ticks = self.ticks.get() + CLOCK_PERIOD;
        self.ticks.set(ticks);
        ticks
    }

    pub fn advance(&self, ticks: EmulationTick) -> EmulationTick {
        let ticks = self.ticks.get() + ticks;
        self.ticks.set(ticks);
        ticks
    }

    pub fn read(&self) -> EmulationTick {
        self.ticks.get()
    }
} // impl Emulation Clock


pub const LAMP_PERSISTENCE: EmulationTick = 1_f64/30_f64;

pub struct Register<'a, T> {
    bits: u8,
    clock: &'a EmulationClock,
    last_tick: EmulationTick,
    mask: T,
    power_mask: T,
    sign_mask: T,
    overflow: bool,
    value: Cell<T>,
    glow: Vec<f32>
}

impl<'a, T> Register<'a, T> 
where T: Copy + 
         BitAnd<Output=T> + BitOr<Output=T> + BitXor<Output=T> + Not<Output=T> +
         Add<Output=T> + AddAssign + Sub<Output=T> + Mul<Output=T> +
         Shl<Output=T> + Shr<Output=T> + ShrAssign +
         From<u8> + Eq {
    
    pub fn new(bits: u8, clock:&'a EmulationClock) -> Self {
        Register {
            bits,
            clock,
            last_tick: clock.read(),
            mask: (T::from(1) << T::from(bits)) - T::from(1),
            power_mask: T::from(1) << T::from(bits),
            sign_mask: T::from(1) << T::from(bits-1),
            overflow: false,
            value: Cell::new(T::from(0)),
            glow: vec![0_f32; bits as usize]
        }
    }

    pub fn update_glow(&mut self, beta: EmulationTick) {
        let this_tick = self.clock.read();
        let elapsed = (this_tick - self.last_tick).max(CLOCK_PERIOD);
        let alpha = (elapsed/LAMP_PERSISTENCE + beta).min(1.0) as f32;
        let a1 = 1.0 - alpha;
        let mut v = self.value.get();
        
        self.last_tick = this_tick;
        for g in self.glow.iter_mut() {
            let b = v & T::from(1);
            if b == T::from(0) {
                *g *= a1;
            } else {
                *g = *g*a1 + alpha;
            }

            v >>= T::from(1);
        }
    }

    pub fn read_glow(&self) -> &Vec<f32> {
        &self.glow
    }

    pub fn read(&self) -> T {
        self.value.get()
    }

    pub fn set(&mut self, value: T) {
        self.value.set(value & self.mask);
        self.update_glow(0.0);
    }

    pub fn add(&mut self, value: T) {
        let augend = self.value.get();
        let result = augend + value;
        if (augend & self.sign_mask) == (value & self.sign_mask) {
            if (value & self.sign_mask) != (result & self.sign_mask) {
                self.overflow = true;
            }
        }

        self.value.set(result & self.mask);
        self.update_glow(0.0);
    }

    pub fn add_unsigned(&mut self, value: T) {
        let augend = self.value.get();
        let result = augend + value;
        if (value & self.power_mask) != (result & self.power_mask) {
            self.overflow = true;
        }

        self.value.set(result & self.mask);
        self.update_glow(0.0);
    }

    pub fn negate(&mut self) {
        let result = self.power_mask - self.value.get();
        self.value.set(result);
        self.update_glow(0.0);
    }


} // impl Register
