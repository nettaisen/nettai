--!strict
-- protoman: a link navi.

local buster = require("./megaman/weapons/buster")
local a_charge = require("@self/a-charge")
local back_special = require("@self/back_special")
local charge = require("@self/charge")
local protoman = require("@self/chip")

return define.navi {
    id = "protoman",
    sprite = asset.sprite("protoman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("protoman-win"), lose = asset.banner("protoman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("protoman"),
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("protoman-portrait"),
    },
    own_chip = { chip = protoman, code = "B" },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, back_special = back_special.weapon, a_charge = a_charge },
    fresh = { hp = 250, mega_level = 5, giga_level = 1, back_special_damage = 50 },
    cross_hp = { 200, 200 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 100, back_special = back_special.weapon },
            { hp = 150, back_special = back_special.weapon },
            { hp = 200, rapid = 1, back_special = back_special.weapon },
            { hp = 250, rapid = 1, back_special = back_special.weapon },
            { hp = 300, attack = 1, rapid = 1, charge = 1, back_special = back_special.weapon },
            { hp = 350, attack = 1, rapid = 1, charge = 1, back_special = back_special.weapon },
            { hp = 400, attack = 1, rapid = 1, charge = 1, back_special = back_special.weapon },
            { hp = 430, attack = 1, rapid = 1, charge = 1, custom_level = 1, back_special = back_special.weapon },
            { hp = 460, attack = 1, rapid = 1, charge = 1, custom_level = 1, back_special = back_special.weapon },
            { hp = 490, attack = 1, rapid = 2, charge = 2, custom_level = 1, back_special = back_special.weapon },
            { hp = 520, attack = 1, rapid = 2, charge = 2, custom_level = 1, back_special = back_special.weapon_2 },
            { hp = 540, attack = 2, rapid = 2, charge = 2, custom_level = 1, back_special = back_special.weapon_2 },
            { hp = 560, attack = 2, rapid = 2, charge = 2, custom_level = 1, mega_level = 1, back_special = back_special.weapon_2 },
            { hp = 580, attack = 3, rapid = 2, charge = 2, custom_level = 1, mega_level = 1, back_special = back_special.weapon_2 },
            { hp = 600, attack = 4, rapid = 3, charge = 3, custom_level = 2, mega_level = 1, back_special = back_special.weapon_2 },
        },
    },
    -- The chips A charges (its family's damaging chips, from a navi level on), and those a charge doubles.
    charged_chips = { family = "sword", from_level = 11, damaging = true },
    charge_doubles = { family = "sword" },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 11,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 11,
        ice = "medium",
        attach_points = {
            { 4, 0 },
            { 4, 0 },
            { 4, 0 },
            { 10, 50 },
            { 10, 50 },
            { 10, 50 },
            { 16, 0 },
            { 16, 24 },
            { 16, 24 },
            { 27, 25 },
            { 20, 26 },
            { 20, 26 },
            { 20, 25 },
            { 24, 24 },
            { 24, 27 },
            { 24, 25 },
            { 43, 28 },
            { 6, 12 },
            { 30, 27 },
            { 40, 27 },
            { 20, 27 },
            { 2, 20 },
            { 24, 26 },
            { 24, 27 },
            { 24, 23 },
            { 24, 27 },
            { 27, 25 },
            { 4, 22 },
            { 90, 8 },
            { 114, -4 },
            { -8, 20 },
            { 68, 22 },
            { 4, 0 },
            { 0, 0 },
        },
    },
}
