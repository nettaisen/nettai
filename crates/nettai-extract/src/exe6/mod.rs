mod custom;
pub(super) mod graphics;
mod gregar;
mod hud;
mod jp;
mod lettering;
pub(super) mod names;
use crate::rom::Rom;
fn u32at(rom: &Rom, a: u32) -> u32 {
    rom.u32(a)
}

pub(super) struct Roms<'a> {
    pub falzar: &'a Rom,
    pub gregar: &'a Rom,
    pub falzar_jp: &'a Rom,
    pub gregar_jp: &'a Rom,
}
