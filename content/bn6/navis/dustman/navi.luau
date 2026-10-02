--!strict
-- dustman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local charge = require("./charge")
local dustman = require("./chip")

return define.navi {
    id = "dustman",
    name = "DustMan",
    sprite = asset.sprite("dustman"),
    element = "null",
    weakness = { "cursor" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("dustman-win"), lose = asset.banner("dustman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("dustman"),
    merge_height = 0,
    -- The no-running message: its lines' characters (which time it), its
    -- words, and the portrait beside it.
    run_message = {
        counts = { 18, 16, 9 },
        text = "Hahahah! You can't\nleave 'em in the\ndust now!",
        portrait = asset.sprite("dustman-portrait"),
    },
    own_chip = { chip = dustman, code = "D" },
    chip_bonus = {
        family = "break",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 },
    },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 250, super_armor = true, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 150, 150 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 10,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 10,
        ice = "large",
        attach_points = {
            { 2, 6 },
            { 2, 6 },
            { 2, 6 },
            { 0, 63 },
            { 0, 60 },
            { 8, 61 },
            { 29, 0 },
            { 23, 29 },
            { 16, 24 },
            { 30, 29 },
            { 24, 29 },
            { 25, 29 },
            { 27, 29 },
            { 29, 28 },
            { 32, 29 },
            { 27, 28 },
            { 66, 31 },
            { 0, 25 },
            { 39, 30 },
            { 43, 31 },
            { 27, 30 },
            { 0, 25 },
            { 32, 29 },
            { 32, 30 },
            { 28, 28 },
            { 29, 31 },
            { 30, 30 },
            { 0, 25 },
            { 106, 20 },
            { 118, -1 },
            { 0, 25 },
            { 78, 24 },
            { 0, 4 },
            { 0, 0 },
        },
    },
}
