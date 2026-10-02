--!strict
-- heatman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local charge = require("./charge")
local heatman = require("./chip")

return define.navi {
    id = "heatman",
    name = "HeatMan",
    sprite = asset.sprite("heatman"),
    element = "fire",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("heatman-win"), lose = asset.banner("heatman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- HeatMan face, and shows SpoutMan's.
    mugshot = asset.mugshot("heatman"),
    merge_height = 8,
    run_message = { 18, 17 },
    own_chip = { chip = heatman, code = "H" },
    chip_bonus = {
        family = "fire",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 0, 0, 0, 30, 30, 30, 30, 40, 40, 40, 50 },
    },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 100, float_shoes = true, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 100, 150 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 1,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 1,
        ice = "large",
        -- What he wears over his body; a flinch restarts it too.
        parts = {
            body = {
                sprite = asset.sprite("heatman-overlay"),
                anims = 160,
                behind = { { 0x15, 0x17 }, { 0x22, 0x23 }, { 0x3B, 0x4F }, { 0x64, 0x8B }, { 0x9F, 0x9F } },
            },
        },
        overlay_hooks = { "death", "refresh", "flinch" },
        attach_points = {
            { 0, 8 },
            { 0, 8 },
            { 0, 8 },
            { 6, 53 },
            { 8, 55 },
            { 8, 55 },
            { 24, 0 },
            { 22, 22 },
            { 16, 22 },
            { 26, 22 },
            { 18, 23 },
            { 24, 23 },
            { 20, 22 },
            { 24, 22 },
            { 26, 23 },
            { 24, 22 },
            { 59, 25 },
            { 8, 16 },
            { 33, 24 },
            { 40, 24 },
            { 22, 24 },
            { 2, 24 },
            { 26, 22 },
            { 24, 23 },
            { 24, 21 },
            { 24, 23 },
            { 28, 22 },
            { 2, 16 },
            { 91, 14 },
            { 120, -5 },
            { -4, 28 },
            { 67, 16 },
            { 6, 8 },
            { 0, 0 },
        },
    },
}
