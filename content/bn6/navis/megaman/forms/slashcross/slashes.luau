--!strict
-- What SlashCross's charged slash (./charge, the original's action 0x41)
-- makes of a sword: the wave it sends (./sword_wave), the blade the navi
-- raises (none: it swings its own arm), the swing's sound, a second,
-- hidden wave (CrosSwrd's: the plain sword's), a second slash after it
-- (DblDream's), and the lock-on mode the Beast Out rush takes for it.
--
-- A sword names its own in its slash's spec (`charged = slashes.wide`,
-- lib/swords/slash.luau; MoonBld in its action's), which the charge reads
-- of the attack's chip. (The original has one table by the charged chip's
-- subtype for each of these: the wave's rows, the blade `byte_80EBB64`,
-- the lock-on mode `byte_80EB028`, and tests of the subtype for the rest.)

local lockon = require("../../../../rules/lockon")
local parts = require("../../../../lib/swords/parts")
local sword_wave = require("./sword_wave")

local slashes = {}

local function slash(s: ChargedSlash): ChargedSlash
    return define.record("charged-slash", s)
end

local waves = sword_wave.waves
local blades, sounds = parts.blades, parts.sounds

slashes.sword = slash { wave = waves.sword, blade = blades.sword, sound = sounds.swing, lockon = lockon.thunder }
slashes.wide = slash { wave = waves.wide, blade = blades.sword, sound = sounds.swing, lockon = lockon.widesht }
slashes.long = slash { wave = waves.long, blade = blades.sword, sound = sounds.swing, lockon = lockon.firebrn1 }
slashes.wide_blade = slash { wave = waves.wide_blade, blade = blades.sword, sound = sounds.swing, lockon = lockon.widesht }
slashes.long_blade = slash { wave = waves.long_blade, blade = blades.sword, sound = sounds.swing, lockon = lockon.firebrn1 }
slashes.life = slash { wave = waves.life, blade = blades.sword, sound = sounds.big_swing, lockon = lockon.drksword }
slashes.dark = slash { wave = waves.dark, blade = blades.sword, sound = sounds.big_swing, lockon = lockon.drksword }
-- Row 7, a sword no chip is.
slashes.unused = slash { wave = waves.unused, blade = blades.sword, sound = sounds.swing, lockon = lockon.drksword }
slashes.muramasa = slash { wave = waves.muramasa, blade = blades.sword, sound = sounds.swing, lockon = lockon.firebrn1 }
slashes.fighter = slash { wave = waves.fighter, blade = blades.sword, sound = sounds.swing, lockon = lockon.yoyo }
-- CrosSwrd's: its X, and the plain sword's wave unseen under it.
slashes.cross = slash {
    wave = waves.cross,
    hidden_wave = waves.sword,
    blade = blades.sword,
    sound = sounds.swing,
    lockon = lockon.thunder,
}
-- DblDream's: two slashes. (The second's lock-on mode is the table's
-- entry; the rush has locked on by then.)
slashes.double_dream_second = slash {
    wave = waves.double_dream_second,
    blade = blades.sword,
    sound = sounds.big_swing,
    lockon = lockon.drksword,
}
slashes.double_dream = slash {
    wave = waves.double_dream,
    blade = blades.sword,
    sound = sounds.big_swing,
    again = slashes.double_dream_second,
    lockon = lockon.drksword,
}
slashes.fire = slash { wave = waves.fire, blade = blades.fire, sound = sounds.swing, lockon = lockon.widesht }
slashes.aqua = slash { wave = waves.aqua, blade = blades.aqua, sound = sounds.swing, lockon = lockon.widesht }
slashes.elec = slash { wave = waves.elec, blade = blades.elec, sound = sounds.swing, lockon = lockon.widesht }
slashes.bamboo = slash { wave = waves.bamboo, blade = blades.bamboo, sound = sounds.swing, lockon = lockon.widesht }
-- The charged shot's own (./charge), and MoonBld's ring: no blade.
slashes.charge = slash { wave = waves.charge, sound = sounds.swing, lockon = lockon.widesht }
slashes.moon = slash { wave = waves.moon, sound = sounds.swing, lockon = lockon.moonbld }

-- Transitional: the slashes by the original's row, a sword chip's subtype
-- (rows 0x10 to 0x12 are the charge's own, MoonBld's and DblDream's
-- second), for a charged chip that is still a pack record with no
-- definition to ask. (Row 0x13, which the blade and lock-on tables have,
-- has no wave.)
local ROWS: { [number]: ChargedSlash } = {
    [0x00] = slashes.sword,
    [0x01] = slashes.wide,
    [0x02] = slashes.long,
    [0x03] = slashes.wide_blade,
    [0x04] = slashes.long_blade,
    [0x05] = slashes.life,
    [0x06] = slashes.dark,
    [0x07] = slashes.unused,
    [0x08] = slashes.muramasa,
    [0x09] = slashes.fighter,
    [0x0A] = slashes.cross,
    [0x0B] = slashes.double_dream,
    [0x0C] = slashes.fire,
    [0x0D] = slashes.aqua,
    [0x0E] = slashes.elec,
    [0x0F] = slashes.bamboo,
    [0x10] = slashes.charge,
    [0x11] = slashes.moon,
    [0x12] = slashes.double_dream_second,
}

function slashes.by_row(row: number): ChargedSlash
    local s = ROWS[row]
    if s == nil then
        error(string.format("sword wave kind %#x reads past byte_80D7F4C (sub_80D80B4)", row))
    end
    return s
end

return slashes
