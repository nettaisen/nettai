--!strict
-- TenguCross: one of MegaMan's forms.

local base = require("../base/form")
local cross_overlay = require("../cross_overlay")
local tenguman = require("../../../tenguman/navi")
local tengucross_beast = require("../tengucross-beast/form")
local buster = require("../../weapons/buster/weapon")
local charge = require("./charge")
local tengu_wind = require("../../weapons/tengu-wind/weapon")

return define.form {
    id = "tengucross",
    kind = "cross",
    game = "falzar",
    cross_of = tenguman,
    beast = tengucross_beast,
    -- A weakness hit breaks the Cross: back to the base form.
    breaks_to = base,
    sprite = asset.sprite("megaman"),
    element = "null",
    weakness = { "sword" },
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, back_special = tengu_wind.weapon },
    palette = 11,
    -- Its faces in the emotion window, and once Beast Out is spent (`byte_801E700`).
    mugshot = { normal = asset.mugshot("tengucross"), tired = asset.mugshot("tengucross-tired") },
    -- What R shows in the Cross window.
    -- The damage it adds to its family's damaging chips.
    chip_bonus = { family = "wind", damage = 10 },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "air_shoes" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 8,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 8 },
    identity = define.identity {
        class = "cross",
        version = 0,
        actor_type = "player",
        ai_index = 32,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 32,
        ice = "medium",
        -- What it wears over MegaMan's body.
        parts = cross_overlay(asset.sprite("tengucross-overlay")),
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
