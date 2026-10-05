--!strict
-- EXE5's GyroMan (navi 2, NameID 0x182), of Team ProtoMan: a team navi (docs/design/exe5-map.md §15.16), from EXE5's
-- tables by navi as ProtoMan's are (navis/protoman): his battle sprite (08-02), his element, buster bonus and move
-- lag, his banners, his face, his actor record (version 0, a player, AI index 2) and his sprite's attach points
-- (category 8's row 2). His stats row gives him FloatShoes and AirShoes.
--
-- He wears his propeller (his actor record's hook, 0x0800EE4E: the body overlays' row 0, sprite 10-0A), which his
-- deletion takes off (0x0800EFDE) and an animation change (0x0800F13C's entry for him) or a flinch (0x080F0A5A, which
-- tests for it first) restarts; his paralysis and drag hooks are empty routines (0x080F0A56).
--
-- He alone has a tick and an idle of his own (./hover): over a panel that isn't solid he takes off and stays in the
-- air, and lands on a solid one. His weapons are his stats row's: his buster (0x43, ./buster: MegaMan's on a solid
-- panel, strikes from the air otherwise) and the charged tornado (0x28, ./charge); no B+Back. His own chip is
-- Airforce G (./chip).

local airforce = require("@self/chip")
local body_overlays = require("../lib/body_overlays")
local buster = require("@self/buster")
local charge = require("@self/charge")
local hover = require("@self/hover")

return define.navi {
    id = "gyroman",
    sprite = asset.sprite("gyroman"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("gyroman-win"), lose = asset.banner("gyroman-deleted") },
    mugshot = asset.mugshot("gyroman"),
    -- The no-running message's portrait: the speaker beside the box (L on the custom screen, 0x08023190: the run
    -- dialog's script 0x18, which script 3's navi check sends him to). Its words are locales/<lang>.toml's; the
    -- content's own (en) time it.
    run_message = {
        portrait = asset.sprite("gyroman-portrait"),
    },
    own_chip = { chip = airforce, code = "G" },
    own_chip_draws = true,
    weapons = { buster = buster.weapon, charge_shot = charge.weapon },
    -- His fresh stats (his stats row): what a side starts him with.
    fresh = { hp = 300, float_shoes = true, air_shoes = true, mega_level = 5, giga_level = 1 },
    -- What the story gives him (its table's row for him, 0x0804F960): his HP by its progress; his level is that
    -- progress up to 6.
    story = { hp = { 250, 250, 250, 300, 350, 400, 500, 600, 700 }, max_level = 6 },
    -- Taking off (animation 18) and in the air (20), his charge glow shows 16 pixels higher (the charge glow's test
    -- of his name, 0x080E0E58).
    charge_glow_lift = { anims = { 18, 20 }, pixels = 16 },
    -- His tick rests while his Airforce runs (0x080F09EE).
    tick = hover.tick(airforce.action :: Action),
    idle = hover.idle,
    identity = define.identity {
        class = "link_navi",
        version = 0,
        actor_type = "player",
        ai_index = 2,
        -- The Full Synchro aura's animation: its owner's AI index (the aura's start, 0x080C4608).
        aura_anim = 2,
        parts = { body = body_overlays.gyroman },
        overlay_hooks = { "death", "refresh", "flinch_checked" },
        attach_points = {
            { 0, 16 },
            { 0, 14 },
            { 0, 8 },
            { 0, 64 },
            { 0, 56 },
            { 0, 64 },
            { 18, 0 },
            { 19, 40 },
            { 18, 37 },
            { 28, 40 },
            { 22, 40 },
            { 23, 40 },
            { 22, 40 },
            { 27, 38 },
            { 27, 40 },
            { 28, 39 },
            { 48, 42 },
            { 0, 32 },
            { 34, 40 },
            { 46, 41 },
            { 5, 40 },
            { 0, 25 },
            { 32, 41 },
            { 24, 41 },
            { 24, 40 },
            { 33, 40 },
            { 6, 32 },
            { 0, 32 },
            { 24, 25 },
            { 44, 40 },
        },
    },
}
