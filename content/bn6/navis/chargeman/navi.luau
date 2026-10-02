--!strict
-- chargeman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local a_charge = require("../megaman/weapons/charged-chip-bonus-29/weapon")
local charge = require("./charge")
local chargeman = require("./chip")

return define.navi {
    id = "chargeman",
    sprite = asset.sprite("chargeman"),
    element = "fire",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("chargeman-win"), lose = asset.banner("chargeman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- ChargeMan face, and shows DustMan's.
    mugshot = asset.mugshot("chargeman"),
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("chargeman-portrait"),
    },
    own_chip = { chip = chargeman, code = "C" },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, a_charge = a_charge },
    fresh = { hp = 250, first_barrier = 0, mega_level = 5, giga_level = 1 },
    cross_hp = { 200, 200 },
    -- The chips A charges, from a navi level on, and the most damage the charge adds to a
    -- Fire chip, by the navi's level.
    charged_chips = { family = "fire", from_level = 3 },
    fire_charge = { 0, 0, 0, 30, 30, 30, 30, 30, 30, 50, 50, 50, 50, 50, 100 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 5,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 5,
        ice = "medium",
        attach_points = {
            { 0, 0 },
            { 0, 10 },
            { 0, 0 },
            { 16, 48 },
            { 4, 46 },
            { 16, 36 },
            { 28, 0 },
            { 14, 17 },
            { 16, 17 },
            { 22, 16 },
            { 16, 16 },
            { 18, 17 },
            { 16, 17 },
            { 22, 17 },
            { 22, 18 },
            { 22, 16 },
            { 48, 18 },
            { 0, 8 },
            { 32, 18 },
            { 38, 18 },
            { 18, 18 },
            { 0, 20 },
            { 22, 16 },
            { 22, 17 },
            { 20, 16 },
            { 22, 18 },
            { 22, 17 },
            { 0, 8 },
            { 80, 8 },
            { 112, -8 },
            { -4, 24 },
            { 60, 14 },
            { 0, 0 },
            { 0, 8 },
        },
    },
}
