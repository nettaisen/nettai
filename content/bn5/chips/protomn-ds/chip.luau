--!strict
-- ProtoMnDS (BN5 chip 0x0f1): ProtoMan, dark, cuts in and slashes each enemy navi he can reach, as ProtoMan's (the
-- shared navi with BN5's look: ../protoman/chip.luau; his palette by the chip's Param1, 0x080C1714: 3 for its 4); its
-- damage by its user's side's statistic 3 (formula 24, its table's row 1).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("../../objects/attachment/attachment")
local common = require("@common/protoman/navi")
local navi = require("../protoman/navi")

local LOOK = common.look {
    sprite = asset.sprite("bn5:protoman"),
    palette = 3,
    sword = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 },
    sword_anim = 1,
    slash = define.effect { sprite = asset.sprite("bn5:sword-slash"), anim = 0, palette = 0 },
    spawner_address = 0x080C1A01,
}

return define.chip {
    id = "bn5:protomn-ds",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 79,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, per_player_gauges = 180 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_with(LOOK),
}
