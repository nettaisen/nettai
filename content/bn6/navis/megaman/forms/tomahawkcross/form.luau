--!strict
-- TomahawkCross: one of MegaMan's forms.

local base = require("../base/form")
local cross_overlay = require("../cross_overlay")
local tomahawkman = require("../../../tomahawkman/navi")
local tomahawkcross_beast = require("../tomahawkcross-beast/form")
local buster = require("../../weapons/buster/weapon")
local charge = require("./charge")
local charged_chip_bonus_20 = require("../../weapons/charged-chip-bonus-20/weapon")
local forms = require("../../../../rules/forms/system")

return define.form {
    id = "bn6:tomahawkcross",
    kind = "cross",
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "falzar",
    cross_of = tomahawkman,
    beast = tomahawkcross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("bn6:megaman"),
    element = "wood",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = charged_chip_bonus_20 },
    palette = 17,
    -- Its faces in the emotion window, and once Beast Out is spent (`byte_801E700`).
    mugshot = { normal = asset.mugshot("bn6:tomahawkcross"), tired = asset.mugshot("bn6:tomahawkcross-tired") },
    -- What R shows in the Cross window.
    -- The chips A charges.
    charged_chips = { { family = "wood" } },
    -- A charged Wood chip hits twice as hard.
    charge_doubles = { element = "wood" },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "clear_statuses" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 7,
    traits = { "status_immune" },
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 7 },
    identity = define.identity {
        class = "cross",
        version = 0,
        actor_type = "player",
        ai_index = 31,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 31,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("bn6:tomahawkcross-overlay")),
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
