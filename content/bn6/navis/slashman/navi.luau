--!strict
-- slashman: a link navi.

local buster = require("../megaman/weapons/buster/weapon")
local charge = require("./charge")
local slashman = require("./chip")

return define.navi {
    id = "bn6:slashman",
    sprite = asset.sprite("bn6:slashman"),
    element = "null",
    weakness = { "break" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("bn6:slashman-win"), lose = asset.banner("bn6:slashman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- SlashMan face, and shows TenguMan's.
    mugshot = asset.mugshot("bn6:slashman"),
    merge_height = 4,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("bn6:slashman-portrait"),
    },
    own_chip = { chip = slashman, code = "S" },
    chip_bonus = {
        family = "sword",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 0, 0, 0, 30, 30, 30, 30, 40, 40, 40, 50 },
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 150, mega_level = 5, giga_level = 1 },
    cross_hp = { 150, 150 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 3,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 3,
        ice = "medium",
        attach_points = {
            { 0, 0 },
            { 3, 8 },
            { 0, 0 },
            { 8, 50 },
            { 8, 48 },
            { 8, 48 },
            { 26, 0 },
            { 24, 26 },
            { 16, 25 },
            { 32, 24 },
            { 26, 24 },
            { 28, 24 },
            { 30, 26 },
            { 32, 24 },
            { 34, 26 },
            { 32, 24 },
            { 48, 26 },
            { 4, 10 },
            { 36, 26 },
            { 44, 26 },
            { 28, 26 },
            { 4, 18 },
            { 32, 24 },
            { 32, 25 },
            { 24, 24 },
            { 34, 26 },
            { 35, 24 },
            { 0, 18 },
            { 98, 16 },
            { 124, -4 },
            { -10, 22 },
            { 64, 22 },
            { 3, 0 },
            { 0, 10 },
        },
    },
}
