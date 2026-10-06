--!strict
-- eraseman: a link navi.

local by_level = require("../rules/by_level")
local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local eraseman = require("@self/chip").edeletbm

local navi: NaviDef = {
    sprite = asset.sprite("eraseman"),
    element = "null",
    weakness = { "wind" },
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("eraseman-win"), lose = asset.banner("eraseman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- EraseMan face, and shows GroundMan's.
    mugshot = asset.mugshot("eraseman"),
    version = "gregar",
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("eraseman-portrait"),
    },
    own_chip = { chip = eraseman, code = "K" },
    chip_bonus = {
        family = "cursor",
        dimming_chips = true,
        damage = by_level.row({ 0, 0, 0, 0, 10, 10, 10, 10, 10, 20, 20, 20, 20, 20, 30 }),
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 300, mega_level = 5, giga_level = 1 },
    cross_hp = { 150, 150 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 700 },
        by_level = {
            { hp = 100 },
            { hp = 150 },
            { hp = 200, rapid = 1 },
            { hp = 250, rapid = 1 },
            { hp = 300, rapid = 1, charge = 1, mega_level = 1 },
            { hp = 350, rapid = 1, charge = 1, mega_level = 1 },
            { hp = 400, rapid = 1, charge = 1, mega_level = 1 },
            { hp = 450, attack = 1, rapid = 2, charge = 1, mega_level = 1 },
            { hp = 500, attack = 1, rapid = 2, charge = 1, mega_level = 1 },
            { hp = 550, attack = 1, rapid = 2, charge = 2, mega_level = 2 },
            { hp = 600, attack = 1, rapid = 2, charge = 2, mega_level = 2 },
            { hp = 650, attack = 2, rapid = 2, charge = 2, mega_level = 2 },
            { hp = 700, attack = 2, rapid = 2, charge = 2, custom_level = 1, mega_level = 2 },
            { hp = 750, attack = 2, rapid = 2, charge = 2, custom_level = 1, mega_level = 2 },
            { hp = 800, attack = 3, rapid = 2, charge = 3, custom_level = 1, mega_level = 2 },
        },
    },
    identity = new.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 4,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 4,
        ice = "medium",
        attach_points = {
            { 3, 10 },
            { 6, 14 },
            { 3, 10 },
            { 12, 60 },
            { 12, 56 },
            { 14, 56 },
            { 24, 0 },
            { 23, 37 },
            { 16, 36 },
            { 30, 36 },
            { 24, 36 },
            { 24, 37 },
            { 23, 36 },
            { 26, 36 },
            { 30, 37 },
            { 32, 36 },
            { 44, 37 },
            { 8, 14 },
            { 35, 38 },
            { 42, 37 },
            { 24, 38 },
            { 8, 26 },
            { 30, 36 },
            { 28, 37 },
            { 24, 38 },
            { 28, 38 },
            { 32, 36 },
            { 8, 14 },
            { 94, 26 },
            { 120, 8 },
            { -14, 32 },
            { 64, 32 },
            { 4, 8 },
            { 0, 16 },
        },
    },
}

return { eraseman = navi }
