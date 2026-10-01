--!strict
-- groundman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local charge = require("./charge")
local groundman = require("./chip")

return define.navi {
    id = "groundman",
    name = "GroundMan",
    sprite = asset.sprite("groundman"),
    element = "null",
    weakness = { "cursor" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("groundman-win"), lose = asset.banner("groundman-deleted") },
    merge_height = 0,
    run_message = { 17, 20, 6 },
    own_chip = { chip = groundman, code = "G" },
    chip_bonus = {
        family = "break",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 },
    },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 300, super_armor = true, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 180, 180 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 9,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 9,
        ice = "large",
        -- What he wears over his body.
        parts = {
            body = {
                sprite = asset.sprite("groundman-overlay"),
                anims = 60,
                behind = { { 0x00, 0x27 }, { 0x3B, 0x3B } },
            },
        },
        attach_points = {
            { -4, 4 },
            { -2, 22 },
            { -4, 4 },
            { 0, 64 },
            { 0, 64 },
            { 0, 58 },
            { 22, 0 },
            { 12, 33 },
            { 12, 33 },
            { 16, 32 },
            { 8, 32 },
            { 16, 32 },
            { 16, 32 },
            { 16, 32 },
            { 16, 33 },
            { 14, 31 },
            { 40, 33 },
            { -4, 16 },
            { 30, 33 },
            { 30, 32 },
            { 14, 33 },
            { -2, 22 },
            { 16, 32 },
            { 16, 32 },
            { 16, 32 },
            { 14, 33 },
            { 18, 31 },
            { -4, 16 },
            { 88, 16 },
            { 120, 0 },
            { -2, 22 },
            { 64, 22 },
            { -4, 8 },
            { 0, 0 },
        },
    },
}
