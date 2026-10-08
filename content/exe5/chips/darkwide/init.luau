--!strict
-- DarkWide (EXE5 chip 0x0c0): a dark chip: WideSht's shot (@exelib/widesht) with EXE5's shooter and a wave of kind 9
-- (chips/widesht: it goes through and trails), one wave (its subtype's count, 0x080ED008). Its use costs buster
-- blanks (rules/light_dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local widesht = require("./widesht/wave")
local dark_costs = require("../lib/dark_costs")

-- Its wave (kind 9, which EXE6's table has and no EXE6 chip fires): it goes through, trails afterimages of its own
-- sprite (animation 1: 0x080CC9A6's word 0x11D10, EXE6's rock cubes 0x10010) and carries bug 0x14 with argument 2, as
-- EXE5's row 0x25 in palette 1 (0x080CC82E).
local WAVE = widesht.variant {
    speed = 0x7_0000,
    pierces = true,
    holds = false,
    trails = true,
    collision = collision.attack_480,
    bug = 0x214,
    palette = 1,
    sprite = asset.sprite("wide-wave"),
    trail_sprite = asset.sprite("wide-wave"),
}
-- Its shooter (the action's subtype 2's, 0x080ECFDE: the attachment table's row 0x37, the wide shooter in palette 3).
local SHOOTER = attachment.look { sprite = asset.sprite("wide-shooter"), palette = 3, attach_point = 12 }

local chip: Chip = {
    codes = { "T" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 300,
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = widesht.action { wave = WAVE, waves = 1, z = 0, shooter = SHOOTER },
    -- What its use costs its user (rules/light_dark).
    dark_cost = dark_costs.buster_blanks,
    auto_battle = { class = "into_reach", range = "all_ahead" },
}

return { darkwide = chip }
