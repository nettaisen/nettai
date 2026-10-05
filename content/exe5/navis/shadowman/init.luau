--!strict
-- EXE5's ShadowMan (navi 8, NameID 0x188), of Team Colonel: a team navi, which a player operates in MegaMan's place.
-- From EXE5's tables by navi: his battle sprite (08-08, 0x0800DA65), his element (none, 0x0800E634), his buster bonus
-- (0x0800F5AC), his move lag (0x0801D462's row), his netbattle banners (0x080090D8, 0x080090B0), his face in the
-- emotion window (0x0801971C on a Team Colonel ROM), and his identity: his actor record (0x08014C94: version 0, a
-- player, AI index 8) and his sprite's 30 attach points (category 8's table, 0x08015C1C, row 8).
--
-- His stats row (0x0801D55F, which 0x080111AA reads) gives him FloatShoes, and his weapons: the buster (routine 0,
-- MegaMan's), the charged attack (0x24, ./charge: he vanishes and throws a shuriken from above) and the B+Back
-- anti-damage stance (0x0D, ./back). A team navi has no forms. His own chip is SplitUp S (./chip: 0x08025EA0's pair
-- for him), which sends his shadow (./shadow).
--
-- His state table (0x080EADA8's row 8) has two more actions, 7 (0x080F9522: he and his shadows throw flames along
-- their rows) and 8 (0x080F97AE: he steps in beside his target and slashes), and his code a move of his own that
-- calls up to two shadows (0x080F9444). Nothing a player does starts them: their chips have no code (0x19F names
-- action 7), and they are the operation battle's ShadowMan's, which nettai doesn't build.

local back = require("@self/back")
local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local splitup = require("@self/chip")

return define.navi {
    id = "shadowman",
    sprite = asset.sprite("shadowman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("shadowman-win"), lose = asset.banner("shadowman-deleted") },
    mugshot = asset.mugshot("shadowman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x1e, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("shadowman-portrait"),
    },
    own_chip = { chip = splitup, code = "S" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon, back_special = back.weapon },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 300, float_shoes = true, mega_level = 5, giga_level = 1, back_special_damage = 50 },
    -- What the story gives him (0x08052E9C: the table 0x0804F960's row for his place in his team, GyroMan's and his):
    -- his HP by the story's progress; his level, which his damage rows are read at, is that count up to 6.
    story = { hp = { 250, 250, 250, 300, 350, 400, 500, 600, 700 }, max_level = 6 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 8,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 8,
        attach_points = {
            { 0, 24 },
            { 0, 28 },
            { 0, 20 },
            { 0, 84 },
            { 0, 84 },
            { 0, 84 },
            { 18, 0 },
            { 15, 58 },
            { 24, 56 },
            { 24, 57 },
            { 19, 57 },
            { 19, 57 },
            { 20, 57 },
            { 23, 56 },
            { 23, 57 },
            { 24, 56 },
            { 32, 58 },
            { 3, 40 },
            { 28, 56 },
            { 40, 57 },
            { 0, 58 },
            { 0, 46 },
            { 25, 58 },
            { 25, 58 },
            { 24, 56 },
            { 26, 56 },
            { 2, 48 },
            { 3, 40 },
            { 24, 25 },
            { 36, 56 },
        },
    },
}
