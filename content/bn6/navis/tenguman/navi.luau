--!strict
-- tenguman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local tengu_wind = require("../megaman/weapons/tengu-wind/weapon")
local charge = require("./charge")
local tenguman = require("./chip")

return define.navi {
    id = "tenguman",
    sprite = asset.sprite("tenguman"),
    element = "null",
    weakness = { "sword" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("tenguman-win"), lose = asset.banner("tenguman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`).
    mugshot = asset.mugshot("tenguman"),
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
        by_level = { 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10 },
    },
    -- What a Cross change brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, back_special = tengu_wind.weapon },
    fresh = { hp = 150, float_shoes = true, air_shoes = true, mega_level = 5, giga_level = 1 },
    cross_hp = { 100, 150 },
    identity = define.identity {
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
