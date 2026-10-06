--!strict
-- Otenko (EXE5 chip 0x0d7): a statue in front of the user that, for 600 ticks, raises the Atk+ bonus of the user's
-- next damaging chip by 1 every 5 ticks, up to 50: EXE6's Otenko (@exelib/otenko, the dimming handler's subtype 60)
-- but for its lifetime (600, EXE6's 1800: 0x080DBBC2), its place in the field-object registry (class 1, one a side:
-- 0x080DBD8A), how it leaves (with a puff with HP left, else bubbles, silent; EXE5 neither absorbs nor blinks
-- obstacles out: 0x080DBD1A), its blessing (with each side keeping its own gauge, the side's special bonus:
-- 0x080DBC78) and EXE5's look.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local common = require("@exelib/otenko")
local effects = require("../lib/effects")

-- Its sheet, 0c-49 (`otenko`: EXE6's JP ROMs' Otenko, the US ROMs' placeholder).
local SPRITE = asset.sprite("otenko")

local OTENKO = common.make {
    statue_id = "otenko/statue",
    controller_id = "otenko/controller",
    sprite = SPRITE,
    -- NameID 0xCF (actor record 0xCF: AI index 0x21, EXE5's field objects').
    identity = define.identity {
        class = "field_object",
        version = 2,
        ai_index = 0x21,
        look = { sprite = SPRITE, anim = 1, palette = 0, shadow = true },
    },
    lifetime = 0x258,
    class = 1,
    sounds = { appear = asset.sound("appear") },
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    effects = { puff = effects.puff, bubbles = effects.bubbles },
    special_bonus = true,
}

return define.chip {
    id = "otenko",
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 66,
    damage = 1,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    dimming = OTENKO.hook,
}
