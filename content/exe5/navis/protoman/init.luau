--!strict
-- EXE5's ProtoMan (navi 1, NameID 0x181), Team ProtoMan's leader: a team navi, which a player operates in MegaMan's
-- place. From EXE5's tables by navi: his battle sprite (08-01, 0x0800DA65), his element (0x0800E634), his buster bonus
-- (0x0800F5AC), his move lag (0x0801D462's row), his netbattle banners (0x080090D8, 0x080090B0), his face in the
-- emotion window (0x08019724: the team navis' own pictures, in their second palette in Full Synchro), and his identity:
-- his actor record (0x08014C94: version 0, a player, AI index 1) and his sprite's 30 attach points (category 8's
-- table, 0x08015C1C, row 1).
--
-- His weapons are his stats row's (0x0801D55F, which 0x080111AA reads): the buster (routine 0, MegaMan's), the charged
-- slash (0x1E, ./charge) and the B+Back guard (0x0B, ./back). A team navi has no forms: the custom screen has no soul
-- button for him (0x08023C90), and a dark chip he uses does nothing (0x08010056). His own chip is StepSwrd B (./chip:
-- 0x08025EA0's pair for him, which the screen's offer picks between with a draw of the console's RNG, 0x08023EFE).

local back = require("@self/back")
local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local stepswrd = require("@self/chip").stepswrd

local navi: NaviDef = {
    sprite = asset.sprite("protoman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("protoman-win"), lose = asset.banner("protoman-deleted") },
    mugshot = asset.mugshot("protoman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x17, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("protoman-portrait"),
    },
    own_chip = { chip = stepswrd, code = "B" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, back_special = back.weapon },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 200, mega_level = 5, giga_level = 1, back_special_damage = 50 },
    -- What the story gives him (0x08052E9C, which a new game's start and a map script call: the table 0x0804F960's
    -- row for him): his HP by the story's progress, the count of the save's event flags set from 0x300 on (0 to 8);
    -- his level, which his damage rows are read at, is that count up to 6 (0x0800EBE0).
    story = { hp = { 200, 300, 350, 400, 450, 500, 600, 700, 800 }, max_level = 6 },
    identity = new.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 1,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 1,
        attach_points = {
            { 0, 0 },
            { 0, 4 },
            { 0, 0 },
            { 8, 54 },
            { 8, 50 },
            { 8, 52 },
            { 18, 0 },
            { 14, 27 },
            { 16, 24 },
            { 24, 26 },
            { 18, 26 },
            { 19, 26 },
            { 19, 26 },
            { 24, 26 },
            { 24, 26 },
            { 24, 26 },
            { 44, 28 },
            { 0, 22 },
            { 28, 26 },
            { 40, 27 },
            { 0, 27 },
            { 0, 18 },
            { 26, 27 },
            { 25, 27 },
            { 24, 26 },
            { 26, 26 },
            { 0, 18 },
            { 0, 22 },
            { 24, 25 },
            { 40, 26 },
        },
    },
}

return { protoman = navi }
