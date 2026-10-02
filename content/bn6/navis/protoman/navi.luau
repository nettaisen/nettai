--!strict
-- protoman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local a_charge = require("./a-charge")
local back_special = require("./back_special")
local charge = require("./charge")
local protoman = require("./chip")

return define.navi {
    id = "protoman",
    name = "Navi 11",
    sprite = asset.sprite("protoman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("protoman-win"), lose = asset.banner("protoman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("protoman"),
    merge_height = 0,
    -- The no-running message: its lines' characters (which time it), its
    -- words, and the portrait beside it.
    run_message = {
        counts = { 19 },
        text = "Don't run away now!",
        portrait = asset.sprite("protoman-portrait"),
    },
    own_chip = { chip = protoman, code = "B" },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, back_special = back_special.weapon, a_charge = a_charge },
    fresh = { hp = 250, mega_level = 5, giga_level = 1, back_special_damage = 50 },
    cross_hp = { 200, 200 },
    -- The chips A charges, from a navi level on, and those a charge doubles.
    charged_chips = { family = "sword", from_level = 11 },
    charge_doubles = { family = "sword" },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 11,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 11,
        ice = "medium",
        attach_points = {
            { 4, 0 },
            { 4, 0 },
            { 4, 0 },
            { 10, 50 },
            { 10, 50 },
            { 10, 50 },
            { 16, 0 },
            { 16, 24 },
            { 16, 24 },
            { 27, 25 },
            { 20, 26 },
            { 20, 26 },
            { 20, 25 },
            { 24, 24 },
            { 24, 27 },
            { 24, 25 },
            { 43, 28 },
            { 6, 12 },
            { 30, 27 },
            { 40, 27 },
            { 20, 27 },
            { 2, 20 },
            { 24, 26 },
            { 24, 27 },
            { 24, 23 },
            { 24, 27 },
            { 27, 25 },
            { 4, 22 },
            { 90, 8 },
            { 114, -4 },
            { -8, 20 },
            { 68, 22 },
            { 4, 0 },
            { 0, 0 },
        },
    },
}
