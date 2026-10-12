--!strict
-- BlakWing (EXE4 chip 0x0D6): the dimming chips' variant 0x29 (0x080E80CA): the shared BlakWing (@exelib/blakwing,
-- EXE5's code), EXE4's parts: perches over the enemy's columns and 18 wings falling down them, one every 12 ticks;
-- sheet 0C-42, the puff (effect row 21), the start's sound (0xAA) and the flap's (0x87, ShadeMan's call), its wings an
-- attack of type 0x480 hitting navis (row 0x25) with the plain spark; its perches each in its column's slot (columns 4
-- to 6, or 1 to 3, left to right: 0x080DD1BA), and as its controller ends the panel its user stands on becomes a hole
-- (0x080E80B6).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local blakwing = require("@exelib/blakwing")
local collision = require("../rules/collision")
local rock = require("../lib/rock")
local sparks = require("../lib/sparks")

local chip: Chip = {
    codes = { "W" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 58,
    damage = 20,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = blakwing.make {
        controller_id = "blakwing/controller",
        flock_id = "blakwing/flock",
        perch_id = "blakwing/perch",
        wing_id = "blakwing/wing",
        sprite = asset.sprite("black-wing"),
        puff = rock.effects.puff,
        sounds = { start = asset.sound("place"), flap = asset.sound("shade-call") },
        is = collision.attack_480,
        hits = collision.hits_navis,
        spark = sparks.plain,
        perches_by_column = true,
        ends_with_hole = true,
    },
    megaman = "dark",
}

return { blakwing = chip }
