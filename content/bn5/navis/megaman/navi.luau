--!strict
-- BN5's MegaMan (navi 0, NameID 0x180), from BN5's tables: his battle sprite (08-00, 0x0800DA65 by navi), his
-- element (0x0800E634), his buster bonus (0x0800F5AC), his move lag (4 after a move, `sub_8010332`'s
-- counterpart 0x0800E0BE: MegaMan's isn't by variant), his netbattle banners (0x080090D8, 0x080090B0), his
-- weapons (weapons/: the buster and the charged shot, BN6's actions with BN5's setups), and his identity: his
-- actor record (0x08014C94: version 0, a player, AI index 0) and his sprite's 30 attach points (./attach_points). BN5 has no weakness table (BN6's `byte_80108D1`).
--
-- Not yet (docs/design/bn5-map.md §15.7): his souls but ProtoSoul (forms, with Soul Unison), his move lag in soul 8
-- (0), the held offset and the custom screen's no-running message.

local attach_points = require("./attach_points")
local buster = require("./weapons/buster")
local charged_shot = require("./weapons/charged-shot")
local protosoul = require("./forms/protosoul/form")

return define.navi {
    id = "bn5:megaman",
    sprite = asset.sprite("bn5:megaman-navi"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("bn5:megaman-win"), lose = asset.banner("bn5:megaman-deleted") },
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    -- His souls, which the custom screen's soul button offers (rules/souls).
    forms = { souls = { protosoul } },
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        -- The Full Synchro aura's animation: BN6's rule (`sub_80C4C52`: the AI index), which BN5 hasn't (its aura
        -- is its own, to come with BN5's emotions); the engine asks every player for one.
        aura_anim = 0,
        attach_points = attach_points,
    },
}
