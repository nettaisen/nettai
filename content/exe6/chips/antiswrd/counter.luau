--!strict
-- AntiSwrd's counter (the original's navi action 0x48, `sub_80EEA3C`): the
-- shared counter (@exelib/antiswrd/counter) with EXE6's sounds, sonic boom
-- and attachment. The ruleset starts it by role
-- (`actions.anti_sword_counter`, rules/roles.luau).

local attachment = require("../../objects/attachment")
local common = require("@exelib/antiswrd/counter")
local parts = require("../../lib/swords/parts")
local sonic_boom = require("../../lib/swords/sonic_boom")

local counter = {}

-- The counter, swinging `blade`. Its booms are EXE6's, or a game's own kind
-- (`boom_kind`).
function counter.action_with(id: string, blade: AttachmentLook, boom_kind: Kind?): Action
    return common.action_with {
        id = id,
        blade = blade,
        boom = boom_kind or sonic_boom.kind,
        start_sound = asset.sound("log-in"),
        swing_sound = parts.sounds.swing,
        attachment = attachment.kind,
        blade_anim = parts.blade_anim,
    }
end

-- What the ruleset starts: both of the counter's starters, a trap chip's
-- catch (`sub_801056A`) and the AntiDmg program's stance (`sub_80105F2`,
-- with the variant its weapon zeroes), leave the attack's variant 0: a
-- sword.
counter.action = counter.action_with("antiswrd/counter", parts.blades.sword)

return counter
