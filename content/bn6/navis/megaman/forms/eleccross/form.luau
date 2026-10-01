--!strict
-- ElecCross: one of MegaMan's forms.

local base = require("../base/form")
local cross_overlay = require("../cross_overlay")
local elecman = require("../../../elecman/navi")
local eleccross_beast = require("../eleccross-beast/form")
local buster = require("../../weapons/buster/weapon")
local charge = require("./charge")
local a_charge = require("./a-charge")

return define.form {
    id = "eleccross",
    name = "ElecCross",
    kind = "cross",
    game = "gregar",
    cross_of = elecman,
    beast = eleccross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("megaman"),
    element = "elec",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = a_charge, alt_a_charge = a_charge },
    palette = 7,
    mugshot = asset.mugshot("tomahawkcross"),
    -- The damage it adds to its family's damaging chips.
    chip_bonus = { family = "elec", damage = 50 },
    -- The chips A charges.
    charged_chips = { { family = "null" } },
    -- A charged chip paralyzes.
    charged_bonus = { paralyzes = true },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 2,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 2 },
    identity = define.identity {
        class = "cross",
        version = 0,
        actor_type = "player",
        ai_index = 26,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 26,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("body-overlay-08")),
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
