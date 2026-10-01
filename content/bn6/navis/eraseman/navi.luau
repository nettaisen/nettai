--!strict
-- eraseman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local charge = require("./charge")
local eraseman = require("./chip")

return define.navi {
    id = "eraseman",
    name = "EraseMan",
    sprite = asset.sprite("eraseman"),
    element = "null",
    weakness = { "wind" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("eraseman-win"), lose = asset.banner("eraseman-deleted") },
    merge_height = 0,
    run_message = { 19, 15, 19 },
    own_chip = { chip = eraseman, code = "K" },
    chip_bonus = {
        family = "cursor",
        dimming_chips = true,
        by_level = { 0, 0, 0, 0, 10, 10, 10, 10, 10, 20, 20, 20, 20, 20, 30 },
    },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 300, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 150, 150 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 4,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 4,
        ice = "medium",
        attach_points = {
            { 3, 10 },
            { 6, 14 },
            { 3, 10 },
            { 12, 60 },
            { 12, 56 },
            { 14, 56 },
            { 24, 0 },
            { 23, 37 },
            { 16, 36 },
            { 30, 36 },
            { 24, 36 },
            { 24, 37 },
            { 23, 36 },
            { 26, 36 },
            { 30, 37 },
            { 32, 36 },
            { 44, 37 },
            { 8, 14 },
            { 35, 38 },
            { 42, 37 },
            { 24, 38 },
            { 8, 26 },
            { 30, 36 },
            { 28, 37 },
            { 24, 38 },
            { 28, 38 },
            { 32, 36 },
            { 8, 14 },
            { 94, 26 },
            { 120, 8 },
            { -14, 32 },
            { 64, 32 },
            { 4, 8 },
            { 0, 16 },
        },
    },
}
