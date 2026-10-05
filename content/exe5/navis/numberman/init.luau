--!strict
-- EXE5's NumberMan (navi 9, NameID 0x189), of Team Colonel: a team navi, which a player operates in MegaMan's place.
-- From EXE5's tables by navi: his battle sprite (08-09, 0x0800DA65), his element (none, 0x0800E634), his buster bonus
-- (0x0800F5AC), his move lag (0x0801D462's row), his netbattle banners (0x080090D8, 0x080090B0), his face in the
-- emotion window (0x0801971C on a Team Colonel ROM), and his identity: his actor record (0x08014C94: version 0, a
-- player, AI index 9) and his sprite's 30 attach points (category 8's table, 0x08015C1C, row 9).
--
-- His stats row (0x0801D55F, which 0x080111AA reads) gives him his weapons: the buster (routine 0, MegaMan's) and the
-- die's throw (0x3D, ./charge); no B+Back. A team navi has no forms. His own chip is NumTrap N (./chip: 0x08025EA0's
-- pair for him), a dimming of his own that sets a trap (./controller, ./trap, ./press).
--
-- He wears his face: NumberSoul's layer in its row 0 (forms/numbersoul/layer, a second sprite that shows while he
-- stands), which his actor record's init hook puts in his second related slot (0x0800EE6E: the identity's own part
-- routine, since the layer is the content's kind) and his death hook takes down (0x0800EFFA). An image made with
-- his record wears one too; an afterimage of him doesn't (EXE5's is a sprite alone: the effects rule
-- `afterimages_wear_overlays`). None of his other hooks touches it.

local buster = require("./megaman/weapons/buster")
local charge = require("@self/charge")
local layer = require("./megaman/forms/numbersoul/layer")
local numtrap = require("@self/chip")

-- The layer's row that is his face.
local FACE = 0

return define.navi {
    id = "numberman",
    sprite = asset.sprite("numberman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("numberman-win"), lose = asset.banner("numberman-deleted") },
    mugshot = asset.mugshot("numberman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x1f, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("numberman-portrait"),
    },
    own_chip = { chip = numtrap, code = "N" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 300, mega_level = 5, giga_level = 1 },
    -- What the story gives him (0x08052E9C: the table 0x0804F960's row for his place in his team, SearchMan's and
    -- his): his HP by the story's progress; his level, which his damage rows are read at, is that count up to 6.
    story = { hp = { 300, 300, 300, 300, 300, 350, 450, 550, 650 }, max_level = 6 },
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 9,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 9,
        parts = {
            -- 0x0800EE6E
            own = function(wearer: Object)
                wearer.related2 = layer.spawn(wearer, FACE)
            end,
        },
        overlay_hooks = { "death" },
        attach_points = {
            { 0, 0 },
            { 0, 10 },
            { 0, 4 },
            { 0, 60 },
            { 0, 56 },
            { 6, 52 },
            { 18, 0 },
            { 14, 26 },
            { 16, 24 },
            { 24, 24 },
            { 18, 24 },
            { 18, 24 },
            { 18, 24 },
            { 24, 24 },
            { 24, 24 },
            { 24, 24 },
            { 40, 26 },
            { 0, 22 },
            { 28, 24 },
            { 40, 25 },
            { 0, 24 },
            { 0, 24 },
            { 25, 25 },
            { 25, 25 },
            { 24, 24 },
            { 26, 24 },
            { 0, 18 },
            { 0, 22 },
            { 24, 25 },
            { 40, 20 },
        },
    },
}
