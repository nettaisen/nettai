--!strict
-- SpoutCross: one of MegaMan's forms.

local base = require("./base")
local cross_overlay = require("./cross_overlay")
local spoutman = require("../../spoutman")
local spoutcross_beast = require("./spoutcross-beast")
local buster = require("../weapons/buster")
local charge = require("@self/charge")
local charged_chip_bonus_1f = require("../weapons/charged-chip-bonus-1f")
local forms = require("../../../rules/forms")

return define.form {
    id = "spoutcross",
    kind = "cross",
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "falzar",
    cross_of = spoutman,
    beast = spoutcross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("megaman"),
    element = "aqua",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = charged_chip_bonus_1f },
    palette = 5,
    -- Its faces in the emotion window, and once Beast Out is spent (`byte_801E700`).
    mugshot = { normal = asset.mugshot("spoutcross"), tired = asset.mugshot("spoutcross-tired") },
    -- What R shows in the Cross window.
    -- The chips A charges.
    charged_chips = { { family = "aqua" } },
    -- A charged Aqua chip hits twice as hard, and an Aqua chip's use heals.
    charge_doubles = { element = "aqua" },
    chip_heals = { element = "aqua" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 6,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 6 },
    identity = define.identity {
        class = "cross",
        version = 0,
        actor_type = "player",
        ai_index = 30,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 30,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("spoutcross-overlay")),
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
