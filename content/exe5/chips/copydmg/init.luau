--!strict
-- CopyDmg (EXE5 chip 0x094): EXE5's action 0x24 (0x080ECEFE), EXE6's CopyDmg (@exelib/copydmg/action, its mark
-- @exelib/copydmg/mark in EXE5's look): the navi raises its buster and fires its mark along its row; the first navi it
-- hits takes, for 180 ticks, the damage its side took the tick before on top of its own (in the operation battle
-- mode the mark also hits its panel for 50 each tick). Its subtype 0 (0x080ECF78's table: subtype 1 a sword wave,
-- attack 0x96, which no chip sends).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local action = require("@exelib/copydmg/action")
local arm = require("../lib/arm")
local collision = require("../rules/collision")
local common_mark = require("@exelib/copydmg/mark")
local sparks = require("../lib/sparks")

-- The mark (EXE5's attack object #0x28, 0x080C9A6C).
local mark = common_mark.make {
    id = "copydmg/mark",
    sprite = asset.sprite("hit-sparks"),
    anim = 0x0B,
    sound = asset.sound("mark"),
    is = collision.attack,
    hits = collision.hits_navis,
    reload_anim = true,
    own_gauges_hit = { spark = sparks.plain, is = collision.attack, hits = collision.hits_navis },
}

local chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 1,
    mb = 12,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = action.make {
        sound = asset.sound("buster-shot"),
        raise_arm = arm.raise,
        shot = mark.spawn,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { copydmg = chip }
