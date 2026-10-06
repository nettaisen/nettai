--!strict
-- tenguman: a link navi.

local by_level = require("../rules/by_level")
local buster = require("./megaman/weapons/buster")
local tengu_wind = require("./megaman/weapons/tengu_wind")
local charge = require("@self/charge")
local tenguman = require("@self/chip").ftornado

local navi: NaviDef = {
    sprite = asset.sprite("tenguman"),
    element = "null",
    weakness = { "sword" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("tenguman-win"), lose = asset.banner("tenguman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("tenguman"),
    version = "falzar",
    merge_height = -8,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("tenguman-portrait"),
    },
    own_chip = { chip = tenguman, code = "T" },
    chip_bonus = {
        family = "wind",
        dimming_chips = false,
        damage = by_level.row({ 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 }),
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, back_special = tengu_wind.weapon },
    fresh = { hp = 150, float_shoes = true, air_shoes = true, mega_level = 5, giga_level = 1 },
    cross_hp = { 100, 150 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 40, float_shoes = true, air_shoes = true },
            { hp = 80, float_shoes = true, air_shoes = true },
            { hp = 120, rapid = 1, float_shoes = true, air_shoes = true },
            { hp = 160, rapid = 1, float_shoes = true, air_shoes = true },
            { hp = 200, attack = 1, rapid = 1, float_shoes = true, air_shoes = true },
            { hp = 240, attack = 1, rapid = 2, float_shoes = true, air_shoes = true },
            { hp = 280, attack = 1, rapid = 2, float_shoes = true, air_shoes = true },
            { hp = 320, attack = 1, rapid = 2, custom_level = 1, float_shoes = true, air_shoes = true },
            { hp = 360, attack = 1, rapid = 3, custom_level = 1, float_shoes = true, air_shoes = true },
            { hp = 400, attack = 2, rapid = 3, custom_level = 1, float_shoes = true, air_shoes = true },
            { hp = 440, attack = 2, rapid = 3, custom_level = 1, float_shoes = true, air_shoes = true },
            { hp = 500, attack = 2, rapid = 3, custom_level = 1, float_shoes = true, air_shoes = true },
            { hp = 650, attack = 2, rapid = 3, custom_level = 2, float_shoes = true, air_shoes = true },
            { hp = 800, attack = 2, rapid = 4, charge = 1, custom_level = 2, mega_level = 1, float_shoes = true, air_shoes = true },
            { hp = 1000, attack = 3, rapid = 4, charge = 1, custom_level = 2, mega_level = 1, float_shoes = true, air_shoes = true },
        },
    },
    identity = new.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 8,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 8,
        ice = "medium",
        attach_points = {
            { 2, 24 },
            { 0, 30 },
            { 2, 24 },
            { 2, 78 },
            { 2, 72 },
            { 6, 78 },
            { 20, 0 },
            { 16, 51 },
            { 16, 50 },
            { 20, 50 },
            { 14, 50 },
            { 16, 50 },
            { 14, 50 },
            { 19, 50 },
            { 22, 51 },
            { 21, 49 },
            { 40, 51 },
            { 5, 32 },
            { 32, 48 },
            { 32, 51 },
            { 16, 51 },
            { 0, 40 },
            { 22, 50 },
            { 19, 50 },
            { 16, 48 },
            { 19, 51 },
            { 22, 50 },
            { 5, 32 },
            { 88, 28 },
            { 120, 16 },
            { -8, 37 },
            { 64, 40 },
            { 2, 8 },
            { 4, 24 },
        },
    },
}

return { tenguman = navi }
