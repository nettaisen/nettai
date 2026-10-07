--!strict
-- EXE4's MegaMan (navi 0, NaviStats +0x23), from EXE4's tables: his battle sprite (00-00: 0x0800B90A's table by navi,
-- 0x08017F98, his soul's number added to the index), his move lag (4 after a move: 0x0800C208's table 0x0800C240, by
-- the actor's variant, the navi and NaviStats +0x25, every entry 4), his netbattle banner (the navi banner table
-- 0x08008548's entry 0, banner 8: the loser's console shows it; no netbattle shows a win's, the flow's
-- `navi_win_banner`), his weapons (weapons/: the buster and the charged shot, EXE4's own actions), his fresh stats
-- (0x0800D6BE: HP 100 at +0x30, +0x32 and +0x34; the Mega and Giga limits 5 and 1 at +0x13 and +0x14), and his
-- identity: a player (the actor table 0x08003A24's kind 0).
--
-- Not yet (docs/design/exe4-map.md §18): his element and its weakness, the buster's own bonus (EXE4's buster is Attack
-- + 1 whatever the navi), his attach points, his souls (the soul's sprite and weapons, 0x080184F0) and the rest of
-- his identity (actor record, AI index, aura).

local buster = require("@self/weapons/buster")
local charged_shot = require("@self/weapons/charged_shot")

local navi: NaviDef = {
    sprite = asset.sprite("megaman"),
    element = "null",
    -- (EXE4's buster adds 1 to Attack itself: ./weapons/buster.)
    buster_bonus = 0,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("megaman-deleted"), lose = asset.banner("megaman-deleted") },
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    fresh = { hp = 100, mega_level = 5, giga_level = 1 },
    -- His forms (he is MegaMan: the rules ask whether a navi has them where the original asks whether it is
    -- MegaMan), and so his faces are his forms' (forms/base's `mugshot`): the souls Double Soul gives (0x080184F0 by
    -- soul), not yet ported (docs/design/exe4-map.md §18 item 25).
    forms = { souls = { form_list = {} } },
    identity = new.identity {
        class = "player",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        -- The Full Synchro aura's animation (0x080CD180, EXE5's code: its owner's AI index).
        aura_anim = 0,
    },
}

return { megaman = navi }
