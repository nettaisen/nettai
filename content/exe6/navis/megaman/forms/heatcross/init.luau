--!strict
-- HeatCross: one of MegaMan's forms.

local base = require("./base")
local cross_overlay = require("./cross_overlay")
local heatman = require("../../heatman")
local heatcross_beast = require("./heatcross-beast")
local buster = require("../weapons/buster")
local charge = require("@self/charge")
local forms = require("../../../rules/forms")

return define.form {
    id = "heatcross",
    kind = "cross",
    -- What changes MegaMan into it (EXE6's five sequences).
    change = forms.action,
    version = "gregar",
    window_order = 0,
    cross_of = heatman,
    beast = heatcross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("megaman"),
    element = "fire",
    buster_bonus = 1,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    palette = 2,
    -- Its faces in the emotion window, and once Beast Out is spent
    -- (`byte_801E700`): the Falzar ROM has no HeatCross face, and shows SpoutCross's.
    mugshot = { normal = asset.mugshot("heatcross"), tired = asset.mugshot("heatcross-tired") },
    -- What R shows in the Cross window.
    -- The damage it adds to its family's damaging chips.
    chip_bonus = { family = "fire", damage = 50 },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 1,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 1 },
    identity = define.identity {
        class = "cross",
        version = 0,
        actor_type = "player",
        ai_index = 25,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 25,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("heatcross-overlay")),
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
