--!strict
-- GroundCross: one of MegaMan's forms.

local base = require("../base/form")
local cross_overlay = require("../cross_overlay")
local groundman = require("../../../groundman/navi")
local groundcross_beast = require("../groundcross-beast/form")
local buster = require("../../weapons/buster/weapon")
local drill = require("./drill")
local rock_barrage = require("../../weapons/rock-barrage/weapon")

return define.form {
    id = "groundcross",
    name = "GroundCross",
    kind = "cross",
    game = "falzar",
    cross_of = groundman,
    beast = groundcross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("megaman"),
    element = "null",
    weakness = { "cursor" },
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = drill.weapon, a_charge = rock_barrage },
    palette = 15,
    -- Its faces in the emotion window, and once Beast Out is spent (`byte_801E700`).
    mugshot = { normal = asset.mugshot("groundcross"), tired = asset.mugshot("groundcross-tired") },
    -- The damage it adds to its family's damaging chips.
    chip_bonus = { family = "break", damage = 10 },
    -- The chips A charges.
    charged_chips = { { family = "break" } },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 9,
    -- A Cross change that finds the drill rising (its animation) lets go of it.
    cross_release_anim = 0x16,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 9 },
    identity = define.identity {
        class = "cross",
        version = 0,
        actor_type = "player",
        ai_index = 33,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 33,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("body-overlay-0d")),
        attach_points = {
            { 2, 0 },
            { 2, 0 },
            { 2, 0 },
            { 8, 48 },
            { 8, 48 },
            { 8, 48 },
            { 16, 0 },
            { 16, 24 },
            { 16, 24 },
            { 24, 24 },
            { 18, 24 },
            { 18, 24 },
            { 18, 24 },
            { 24, 24 },
            { 23, 25 },
            { 24, 24 },
            { 40, 26 },
            { 5, 11 },
            { 31, 25 },
            { 40, 26 },
            { 20, 25 },
            { 2, 16 },
            { 24, 24 },
            { 24, 25 },
            { 24, 23 },
            { 22, 25 },
            { 28, 25 },
            { 2, 18 },
            { 88, 16 },
            { 120, -4 },
            { -8, 16 },
            { 64, 16 },
            { 2, 0 },
            { 0, 0 },
        },
    },
}
