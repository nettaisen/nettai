--!strict
-- Poltrgst (EXE4 chip 0x17D, past the folders' chips): the dimming chip JunkSoul's charged shot uses (weapon routine
-- 0x61, 0x0800CE4C): the dimming handler's variant 0x41 (0x080EA118), EXE5's Poltrgst's controller and stand-in before
-- EXE5 (@exelib/poltrgst: the controller, effect object #0x72, 0x080EA05C; the stand-in, actor object #0x58,
-- 0x080CC624, MegaMan in his soul's sprite and his animation 10) raising EXE4's poltergeist (objects/poltergeist) at
-- random targets (its first parameter 0). EXE4's own of the shared code: only MegaMan has a stand-in, the damage word
-- is the chip's alone, and the spawners copy the side without a facing. No one cuts in on it (the handler's
-- 0x080EBA26: a chip from 0x170 on). No folder holds it. Its record is written by tools/exe4/gen_content.py
-- (bn6battle-verify) from the EXE4 ROMs.

local poltergeist = require("../objects/poltergeist")
local poltrgst = require("@exelib/poltrgst")

local made = poltrgst.make {
    controller_id = "poltrgst/controller",
    stand_in_id = "poltrgst/stand-in",
    poltergeist = poltergeist,
    anim = 10,
    megaman_only = true,
}

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = {},
    traits = { "no_cut_in" },
    dimming = made.hook(false),
}

return { poltrgst = chip }
