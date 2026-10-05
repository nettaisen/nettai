--!strict
-- elecman: a link navi.

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local elecman = require("@self/chip")

return define.navi {
    id = "elecman",
    sprite = asset.sprite("elecman"),
    element = "elec",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("elecman-win"), lose = asset.banner("elecman-deleted") },
    -- Its face in the emotion window (`byte_801CDDC`): the Falzar ROM has no
    -- ElecMan face, and shows TomahawkMan's.
    mugshot = asset.mugshot("elecman"),
    version = "gregar",
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("elecman-portrait"),
    },
    own_chip = { chip = elecman, code = "E" },
    chip_bonus = {
        family = "elec",
        dimming_chips = false,
        by_level = { 0, 0, 0, 0, 0, 0, 0, 30, 30, 30, 30, 40, 40, 40, 50 },
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    fresh = { hp = 200, float_shoes = true, mega_level = 5, giga_level = 1 },
    cross_hp = { 130, 130 },
    -- What the save's reload gives it at its link navi level
    -- (`reloadCurNaviBaseStats_8120df0`, docs/engine/link-navis.md): its base
    -- and maximum HP by the story's progress (`off_8120F44`), and what each
    -- level, 0 to 14, adds over its fresh stats (`pt_8121200`'s scripts).
    levels = {
        base_hp = { 300, 300, 300, 400, 500, 600, 800 },
        by_level = {
            { hp = 100, float_shoes = true },
            { hp = 150, float_shoes = true },
            { hp = 200, rapid = 1, mega_level = 1, float_shoes = true },
            { hp = 250, rapid = 1, mega_level = 1, float_shoes = true },
            { hp = 300, attack = 1, rapid = 1, mega_level = 1, float_shoes = true },
            { hp = 350, attack = 1, rapid = 1, mega_level = 1, float_shoes = true },
            { hp = 400, attack = 1, rapid = 1, mega_level = 1, float_shoes = true },
            { hp = 450, attack = 1, rapid = 1, mega_level = 1, float_shoes = true },
            { hp = 500, attack = 1, rapid = 1, mega_level = 1, float_shoes = true },
            { hp = 550, attack = 2, rapid = 1, charge = 1, mega_level = 1, float_shoes = true },
            { hp = 600, attack = 2, rapid = 1, charge = 1, mega_level = 1, float_shoes = true },
            { hp = 700, attack = 2, rapid = 1, charge = 1, custom_level = 1, mega_level = 2, float_shoes = true },
            { hp = 800, attack = 2, rapid = 1, charge = 1, custom_level = 1, mega_level = 2, float_shoes = true },
            { hp = 900, attack = 2, rapid = 2, charge = 2, custom_level = 1, mega_level = 2, float_shoes = true },
            { hp = 1100, attack = 3, rapid = 2, charge = 2, custom_level = 1, mega_level = 2, float_shoes = true },
        },
    },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 2,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 2,
        ice = "medium",
        attach_points = {
            { 0, 20 },
            { 0, 20 },
            { 0, 20 },
            { 2, 70 },
            { 2, 70 },
            { 2, 70 },
            { 16, 0 },
            { 12, 43 },
            { 12, 43 },
            { 20, 44 },
            { 14, 44 },
            { 14, 45 },
            { 14, 44 },
            { 20, 44 },
            { 20, 45 },
            { 20, 44 },
            { 35, 46 },
            { 2, 30 },
            { 27, 47 },
            { 36, 45 },
            { 16, 46 },
            { 0, 32 },
            { 20, 44 },
            { 18, 45 },
            { 20, 43 },
            { 20, 46 },
            { 23, 45 },
            { 2, 34 },
            { 88, 28 },
            { 112, 16 },
            { -8, 28 },
            { 64, 40 },
            { 2, 16 },
            { 2, 10 },
        },
    },
}
