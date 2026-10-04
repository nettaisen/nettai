--!strict
-- Falzar Beast: one of MegaMan's forms.

local gregar_beast = require("./gregar-beast")
local falzar_beast_buster = require("../weapons/falzar-beast-buster")
local charged_chip_bonus = require("../weapons/charged-chip-bonus")
local beast_claw = require("../weapons/beast-claw")
local forms = require("../../../rules/forms")

return define.form {
    id = "falzar-beast",
    kind = "beast",
    -- The target marker shows, its afterimages stay, a Null chip's A charge times by the claw,
    -- and its palette follows the mood (`sub_80E1566`, `sub_80E341E`, `sub_8012F62`,
    -- `sub_80100EC`).
    traits = { "shows_target_marker", "afterimages_stay", "alt_charge_time", "mood_palette" },
    -- What changes MegaMan into it (BN6's five sequences).
    change = forms.action,
    game = "falzar",
    -- (A weakness hit breaks a Beast as a Cross: the original's routine, by the form's
    -- number, drops this one to the Gregar beast.)
    breaks_to = gregar_beast,
    sprite = asset.sprite("falzar-beast"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = falzar_beast_buster.weapon, a_charge = charged_chip_bonus, alt_a_charge = beast_claw.weapon },
    -- Its faces in the emotion window, and in Full Synchro (`byte_801E700`).
    mugshot = { normal = asset.mugshot("falzar-beast"), full_synchro = asset.mugshot("falzar-beast-full-synchro") },
    -- Beast Out: its damaging Null chips' bonus, where the family's doesn't apply.
    null_bonus = 30,
    -- The chips A charges.
    charged_chips = { { family = "null", damaging = false } },
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "air_shoes", "float_shoes", "floating_body", "target_marker" },
    -- The Cross special's buster volley (`sub_802D4F0`).
    special_volley = 2,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 12 },
    identity = define.identity {
        class = "beast",
        version = 0,
        actor_type = "player",
        ai_index = 36,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 13,
        ice = "medium",
        -- The beast's head, whose palette follows the mood.
        parts = { beast_head = { sprite = asset.sprite("beast-head") } },
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
