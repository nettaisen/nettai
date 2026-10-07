--!strict
-- Rflectr1, Rflectr2 and Rflectr3: the navi holds up a shield for 60
-- ticks; the first hit it blocks from the front sends a wave back along
-- the row with the chip's damage (guard.luau). Each holds up its own look.

local guard = require("@self/guard")
local shield = require("@self/shield")
local shot = require("@self/shot")

-- Their counter: the Reflectors' wave (shot.luau) back along the row, 16
-- pixels up, with its sound.
local WAVE_Z = 0x10_0000
local WAVE_SOUND = asset.sound("wave")
local function wave(me: Object, x: number, y: number, damage: number)
    shot.spawn(me, x, y, WAVE_Z, damage)
    battle.play_sound(WAVE_SOUND)
end

-- Their shields' looks (`byte_80C9664`'s rows 0, 7 and 8).
local function look(anim: number): ReflectorShieldLook
    return shield.look { sprite = shield.sprite, anim = anim, fade_anim = anim + 1, fade_ticks = 0xE, palette = 0 }
end
local RFLECTR1, RFLECTR2, RFLECTR3 = look(0), look(6), look(8)

local chips: { [string]: Chip } = {
    rflectr1 = {
        codes = { "A", "C", "P", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 7, damage = 60, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = guard.action { ticks = 60, look = RFLECTR1, counter = wave },
    },
    rflectr2 = {
        codes = { "B", "G", "Y", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 16, damage = 120, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = guard.action { ticks = 60, look = RFLECTR2, counter = wave },
    },
    rflectr3 = {
        codes = { "E", "F", "O", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 25, damage = 200, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = guard.action { ticks = 60, look = RFLECTR3, counter = wave },
    },
}

return chips
