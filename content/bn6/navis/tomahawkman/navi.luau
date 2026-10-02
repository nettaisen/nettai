--!strict
-- tomahawkman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local a_charge = require("../megaman/weapons/charged-chip-bonus-1f/weapon")
local charge = require("./charge")
local tomahawkman = require("./chip")

return define.navi {
    id = "tomahawkman",
    name = "TomahawkMan",
    sprite = asset.sprite("tomahawkman"),
    element = "wood",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("tomahawkman-win"), lose = asset.banner("tomahawkman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("tomahawkman"),
    merge_height = 0,
    run_message = { 13, 17 },
    own_chip = { chip = tomahawkman, code = "T" },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = a_charge },
    fresh = { hp = 200, super_armor = true, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 100, 150 },
    -- The chips A charges, from a navi level on, and those a charge doubles.
    charged_chips = { family = "wood", from_level = 11 },
    charge_doubles = { element = "wood" },
    -- A hit's status doesn't take.
    traits = { "status_immune" },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 7,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 7,
        ice = "medium",
        attach_points = {
            { 0, 0 },
            { 4, 8 },
            { 0, 0 },
            { 10, 54 },
            { 10, 50 },
            { 10, 46 },
            { 24, 0 },
            { 13, 21 },
            { 13, 21 },
            { 22, 20 },
            { 16, 20 },
            { 18, 20 },
            { 18, 20 },
            { 22, 21 },
            { 22, 21 },
            { 20, 20 },
            { 34, 22 },
            { 6, 8 },
            { 31, 21 },
            { 34, 21 },
            { 18, 21 },
            { 4, 22 },
            { 22, 20 },
            { 22, 21 },
            { 16, 20 },
            { 22, 21 },
            { 22, 20 },
            { 6, 8 },
            { 88, 11 },
            { 120, -8 },
            { 4, 22 },
            { 64, 16 },
            { 0, 0 },
            { 8, 8 },
        },
    },
}
