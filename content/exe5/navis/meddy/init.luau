--!strict
-- EXE5's Meddy (navi 6, NameID 0x186), of Team ProtoMan: a team navi (docs/design/exe5-map.md §15.16), from EXE5's
-- tables by navi as ProtoMan's are (navis/protoman): her battle sprite (08-06), her element, buster bonus and move
-- lag, her banners, her face, her actor record (version 0, a player, AI index 6) and her sprite's attach points
-- (category 8's row 6). Her sprite's palettes go by two (0x0801D737). She wears nothing and has no hooks of her own.
--
-- Her weapons are her stats row's: the buster (routine 0, MegaMan's) and the charged capsule (0x3B, ./charge); no
-- B+Back. Her own chip is MeddyCap M (./chip). Both are her own action 7, her throw (./throw), her only own action.

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local meddycap = require("@self/chip").meddycap

local navi: NaviDef = {
    sprite = asset.sprite("meddy"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("meddy-win"), lose = asset.banner("meddy-deleted") },
    mugshot = asset.mugshot("meddy"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x1c, which script 3's navi check sends her to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("meddy-portrait"),
    },
    own_chip = { chip = meddycap, code = "M" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    -- Her fresh stats (her stats row): what a side starts her with.
    fresh = { hp = 400, mega_level = 5, giga_level = 1 },
    -- What the story gives her (its table's row for her, 0x0804F960): her HP by its progress; her level is that
    -- progress up to 6.
    story = { hp = { 300, 300, 300, 300, 300, 300, 400, 500, 600 }, max_level = 6 },
    palette_step = 2,
    identity = new.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 6,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 6,
        attach_points = {
            { 0, 6 },
            { 0, 4 },
            { 0, 3 },
            { 0, 56 },
            { 0, 56 },
            { 0, 56 },
            { 16, 0 },
            { 8, 29 },
            { 12, 27 },
            { 18, 29 },
            { 11, 28 },
            { 11, 28 },
            { 13, 28 },
            { 14, 28 },
            { 16, 28 },
            { 20, 28 },
            { 32, 30 },
            { 0, 20 },
            { 18, 31 },
            { 31, 29 },
            { -5, 29 },
            { 0, 22 },
            { 16, 30 },
            { 18, 29 },
            { 14, 28 },
            { 18, 29 },
            { 0, 20 },
            { 0, 24 },
            { 24, 25 },
            { 36, 28 },
        },
    },
}

return { meddy = navi }
