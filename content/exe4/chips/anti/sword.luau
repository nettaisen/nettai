--!strict
-- EXE4's AntiSwrd counter (navi action 0x39, 0x080EDA68): the shared counter (@exelib/antiswrd/counter, EXE6's code)
-- with EXE4's spec: no invulnerability, nothing restarted at its swings (as EXE5's); the swords' blade in the related
-- slot as EXE4's own attachment, its animation the navi's number (NaviStats +0x23, MegaMan's 0: 0x080EDAC2); every
-- boom (../../objects/sonic_boom) of hit modifier 1 (0x080EDB00); its sounds 0x129 as it starts and the sword's swing.
-- The rules start it by role (`actions.anti_sword_counter`, rules/roles.luau).

local attachment = require("../../objects/attachment")
local common = require("@exelib/antiswrd/counter")
local sonic_boom = require("../../objects/sonic_boom")
local swords = require("../../lib/swords")

local sword = {}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

sword.action = common.action_with {
    id = "antiswrd/counter",
    blade = swords.blade,
    boom = sonic_boom.kind,
    start_sound = asset.sound("anti-sword"),
    swing_sound = swords.swing,
    hold = function(me: Object, blade: AttachmentLook)
        attachment.attach(me, blade, "related", NAVI_NUMBER)
    end,
    boom_hit_mod = 1,
    plain = true,
}

return sword
