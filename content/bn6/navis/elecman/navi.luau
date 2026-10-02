--!strict
-- elecman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local charge = require("./charge")
local elecman = require("./chip")

return define.navi {
    id = "elecman",
    name = "ElecMan",
    sprite = asset.sprite("elecman"),
    element = "elec",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("elecman-win"), lose = asset.banner("elecman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- ElecMan face, and shows TomahawkMan's.
    mugshot = asset.mugshot("elecman"),
    merge_height = 0,
    run_message = { 20 },
    run_message_text = "Don't run away now!!",
    own_chip = { chip = elecman, code = "E" },
    chip_bonus = {
        family = "elec",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 0, 0, 0, 30, 30, 30, 30, 40, 40, 40, 50 },
    },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 200, float_shoes = true, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 130, 130 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 2,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 2,
        ice = "medium",
        attach_points = {
            { 0, 20 },
            { 0, 20 },
            { 0, 20 },
            { 2, 70 },
            { 2, 70 },
            { 2, 70 },
            { 16, 0 },
            { 12, 43 },
            { 12, 43 },
            { 20, 44 },
            { 14, 44 },
            { 14, 45 },
            { 14, 44 },
            { 20, 44 },
            { 20, 45 },
            { 20, 44 },
            { 35, 46 },
            { 2, 30 },
            { 27, 47 },
            { 36, 45 },
            { 16, 46 },
            { 0, 32 },
            { 20, 44 },
            { 18, 45 },
            { 20, 43 },
            { 20, 46 },
            { 23, 45 },
            { 2, 34 },
            { 88, 28 },
            { 112, 16 },
            { -8, 28 },
            { 64, 40 },
            { 2, 16 },
            { 2, 10 },
        },
    },
}
