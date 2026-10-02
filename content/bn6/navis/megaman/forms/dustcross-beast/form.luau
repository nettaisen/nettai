--!strict
-- DustCross Beast: one of MegaMan's forms.

local dustman = require("../../../dustman/navi")
local falzar_beast = require("../falzar-beast/form")
local throw_absorbed = require("./throw_absorbed")
local absorb = require("../../weapons/absorb/weapon")
local charged_chip_bonus = require("../../weapons/charged-chip-bonus/weapon")
local scatter = require("./scatter")
local forms = require("../../../../rules/forms/system")

return define.form {
    id = "dustcross-beast",
    kind = "cross_beast",
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "falzar",
    cross_of = dustman,
    -- A weakness hit breaks the Cross: back to the Beast.
    breaks_to = falzar_beast,
    sprite = asset.sprite("dustcross-beast"),
    element = "null",
    weakness = { "cursor" },
    buster_bonus = 0,
    weapons = {
        buster = throw_absorbed.weapon,
        back_special = absorb.weapon,
        a_charge = charged_chip_bonus,
        alt_a_charge = scatter.weapon,
    },
    mugshot = asset.mugshot("dustcross-beast"),
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "null", damaging = false } },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "air_shoes", "float_shoes", "floating_body", "lockon_marker" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 22,
    traits = { "scrap_button", "special_holds_buster" },
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 22 },
    identity = define.identity {
        class = "cross_beast",
        version = 0,
        actor_type = "player",
        ai_index = 46,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 13,
        ice = "medium",
        -- The beast's head, in the Cross's palette.
        parts = { beast_head = { sprite = asset.sprite("beast-head"), palette = 10 } },
        attach_points = {
            { 4, 16 },
            { 4, 24 },
            { 4, 16 },
            { 9, 64 },
            { 9, 64 },
            { 9, 64 },
            { 16, 0 },
            { 16, 37 },
            { 16, 38 },
            { 24, 38 },
            { 17, 38 },
            { 17, 38 },
            { 18, 38 },
            { 22, 38 },
            { 22, 39 },
            { 24, 37 },
            { 40, 39 },
            { 9, 23 },
            { 29, 40 },
            { 40, 39 },
            { 18, 39 },
            { 4, 32 },
            { 22, 38 },
            { 22, 39 },
            { 24, 39 },
            { 20, 39 },
            { 25, 38 },
            { 4, 32 },
            { 88, 28 },
            { 114, 8 },
            { -8, 28 },
            { 65, 34 },
            { 4, 10 },
            { 4, 8 },
        },
    },
}
