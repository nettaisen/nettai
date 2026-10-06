--!strict
-- Gregar Beast Over: one of MegaMan's forms.

local falzar_beast = require("./falzar_beast")["falzar-beast"]
local gregar_beast_buster = require("../weapons/gregar_beast_buster")
local beast_claw = require("../weapons/beast_claw")
local forms = require("../../../rules/forms")

local form: FormDef = {
    kind = "beast_over",
    -- The target marker shows, its afterimages stay, a Null chip's A charge times by the claw,
    -- and its damaging Null chips deal double (`sub_80E1566`, `sub_80E341E`, `sub_8012F62`,
    -- `sub_8012ABC`); rules/beast's berserk decides its idle (`sub_802D322`).
    traits = { "controlled", "shows_target_marker", "afterimages_stay", "alt_charge_time", "doubles_null" },
    -- It glows (`sub_8016A38`, `byte_8016A68`): the navi's color shader by the battle time, a tick each.
    glow = {
        0x0000, 0x0000, 0x0041, 0x0461, 0x0881, 0x0CC2, 0x10E2, 0x1102, 0x1543,
        0x1983, 0x1DC3, 0x21E4, 0x2204, 0x2204, 0x21E4, 0x1DC3, 0x1983, 0x1543,
        0x1102, 0x10E2, 0x0CC2, 0x0CA2, 0x0881, 0x0861, 0x0441, 0x0421,
    },
    -- What changes MegaMan into it (EXE6's five sequences).
    change = forms.action,
    version = "gregar",
    -- (No hit breaks Beast Over; the original's routine, by the form's number, would drop
    -- either to the Falzar beast.)
    breaks_to = falzar_beast,
    sprite = asset.sprite("gregar-beast"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = gregar_beast_buster.weapon, alt_a_charge = beast_claw.weapon },
    -- Its face in the emotion window (`byte_801E700`): the Falzar ROM has no
    -- Gregar Beast Over face, and shows Falzar Beast Over's.
    mugshot = asset.mugshot("gregar-beast-over"),
    -- What its status reset gives the navi (`sub_8014536`).
    status_reset = { "super_armor", "target_marker", "invulnerable", "controller_restart" },
    -- The Cross special's buster volley (`sub_802D4F0`): the original meant 6, and stores the form's number.
    special_volley = 23,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 23 },
    identity = new.identity {
        class = "player",
        changes_form = true,
        version = 0,
        actor_type = "player",
        ai_index = 47,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 12,
        ice = "medium",
        attach_points = {
            { 6, 0 },
            { 6, 0 },
            { 6, 0 },
            { 16, 44 },
            { 16, 44 },
            { 16, 44 },
            { 24, 0 },
            { 27, 17 },
            { 24, 17 },
            { 38, 18 },
            { 31, 18 },
            { 31, 18 },
            { 29, 17 },
            { 35, 17 },
            { 35, 19 },
            { 35, 17 },
            { 51, 20 },
            { 8, 10 },
            { 41, 20 },
            { 48, 19 },
            { 32, 19 },
            { 6, 16 },
            { 35, 18 },
            { 32, 18 },
            { 32, 16 },
            { 32, 19 },
            { 36, 17 },
            { 8, 16 },
            { 104, 8 },
            { 124, -10 },
            { -8, 24 },
            { 80, 14 },
            { 6, 0 },
            { 0, 4 },
        },
    },
}

return { ["gregar-beast-over"] = form }
