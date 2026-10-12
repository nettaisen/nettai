--!strict
-- MetaGel (EXE4 chip 0x05F): a dimming chip that drops a gel from high above on each row's front panel of the other
-- side's area, one every 13 ticks in a shuffled order: each hits that panel and, where it stays, takes it for the
-- user's side. The dimming chips' action 0x0C, its variant 0x0C (0x080220E0's): EXE5's controller and gel
-- (@exelib/metagel) in EXE4's look. Its controller (effect object #0x21, 0x080E4618; its spawner 0x080E4728 the usual
-- one) drops each row's gel on the other side's panel nearest the user's (0x0800A1AE: `nearest_other`); its gel
-- (attack object #0x45, 0x080D5DF4) has EXE4's sounds (0x115 as it drops, 0xCA as it lands), the puff of effect row
-- 0x15, and takes its panel unless that is the other side's back column (0x080D5F0C). Its record is written by
-- tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local common_controller = require("@exelib/metagel/controller")
local common_gel = require("@exelib/metagel/gel")
local sparks = require("../lib/sparks")

-- 0x080D5F0C: anywhere but the other side's back column (column 6 for side 0's gel, 1 for side 1's).
local function takes(x: number, _y: number, side: number): boolean
    return x ~= bit32.bxor(side, 1) * 5 + 1
end

local GEL = common_gel.make {
    id = "metagel/gel",
    sprite = asset.sprite("meta-gel"),
    drop = asset.sound("drop"),
    land = asset.sound("gel-land"),
    spark = sparks.aqua,
    is = collision.thrown,
    hits = collision.hits_navis,
    takes = takes,
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
}

local CONTROLLER = common_controller.make {
    id = "metagel/controller",
    gel = GEL.spawn,
    front = common_controller.nearest_other,
}

local chip: Chip = {
    codes = { "K", "T", "Z" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 52,
    damage = 120,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = CONTROLLER.hook,
}

return { metagel = chip }
