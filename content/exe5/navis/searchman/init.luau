--!strict
-- EXE5's SearchMan (navi 3, NameID 0x183), of Team ProtoMan: a team navi (docs/design/exe5-map.md §15.16), from EXE5's
-- tables by navi as ProtoMan's are (navis/protoman): his battle sprite (08-03), his element, buster bonus and move
-- lag, his banners, his face, his actor record (version 0, a player, AI index 3) and his sprite's attach points
-- (category 8's row 3). He wears nothing and has no hooks of his own.
--
-- His weapons are his stats row's: the buster (routine 0, MegaMan's) and the charged shot (0x3F, ./charge); no B+Back.
-- His own chip is Satelity S (./chip), his own action 7. His own action 8 (0x080F0E64: five shots at the panel of a
-- target his attack is given, marked in the operation battle) is his attack in an auto battle, which no weapon or
-- chip starts: it comes with the gate navis' operation battle, not here.

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local satelity = require("@self/chip")

return define.navi {
    id = "searchman",
    sprite = asset.sprite("searchman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("searchman-win"), lose = asset.banner("searchman-deleted") },
    mugshot = asset.mugshot("searchman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x19, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("searchman-portrait"),
    },
    own_chip = { chip = satelity, code = "S" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 400, mega_level = 5, giga_level = 1 },
    -- What the story gives him (its table's row for him, 0x0804F960): his HP by its progress; his level is that
    -- progress up to 6.
    story = { hp = { 300, 300, 300, 300, 300, 350, 450, 550, 650 }, max_level = 6 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 3,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 3,
        attach_points = {
            { 0, 10 },
            { 0, 10 },
            { 0, 6 },
            { 6, 62 },
            { 0, 54 },
            { 8, 56 },
            { 18, 0 },
            { 18, 32 },
            { 16, 30 },
            { 28, 32 },
            { 21, 32 },
            { 22, 32 },
            { 22, 32 },
            { 24, 32 },
            { 27, 32 },
            { 28, 31 },
            { 44, 33 },
            { 0, 24 },
            { 29, 34 },
            { 42, 32 },
            { 0, 32 },
            { 0, 22 },
            { 27, 33 },
            { 28, 33 },
            { 24, 30 },
            { 28, 32 },
            { 4, 24 },
            { 0, 24 },
            { 24, 25 },
            { 42, 32 },
        },
    },
}
