--!strict
-- EXE5's Poltrgst (the dimming handler's subtype 49): the shared one (@exelib/poltrgst, which EXE4's JunkSoul's
-- charged shot's is too) with EXE5's parts. Its controller (effect object #0x72, 0x080E8C1C): the usual dimming
-- phases, whose effect (0x080E8C60), when the field has an obstacle Poltergeist takes, sends out a stand-in of the
-- user (actor object #0x58, 0x080C38E8: drawn as the user in animation 12 while the user vanishes), which raises the
-- poltergeist (effect object #0x73, 0x080E8CFC: @exelib/poltergeist, which EXE4's JunkMan shares): it takes every such
-- obstacle (the field-object registry's eight slots in order, but those whose identities aren't `throwable`: EXE5's
-- NameIDs 0xDA, 0xD3, 0xD2, 0xE5, 0xE4 and 0xE7), front to back as its side faces (on one column, top row first), and
-- throws each at a panel (`obstacle.throw`, `sub_800F6AC`: the obstacle rises, shakes 30 ticks, 20 more for each
-- before it, and flies onto the panel, breaking there with a hit of the chip's damage); the panel a random one with
-- the other side's navi on it, else one of the other side's but its own, else any (`sub_800F6C6`, a
-- GetPositiveSignedRNG2 draw each), or, by its first parameter (the chip's: 0 for Poltrgst), the nearest such in front
-- (0x080E8E7C). Once the field has none left, 30 ticks later it ends, and the stand-in gives the user back.
--
-- EXE5's own of the shared code: a player user's overlay on the stand-in (0x080C39A0: its soul's put-on, NumberSoul's
-- layer), the Atk+ bonus on the stand-in's damage word, the stand-in's destroy state clearing the controller's flag
-- (0x080C39BE), and the user's facing with its side.

local common = require("@exelib/poltergeist")
local poltrgst = require("@exelib/poltrgst")

local made = poltrgst.make {
    controller_id = "poltrgst/controller",
    stand_in_id = "poltrgst/stand-in",
    poltergeist = common.make("poltrgst/poltergeist"),
    anim = 12,
    wears_overlay = true,
    adds_bonus = true,
    destroy_tells_controller = true,
    takes_facing = true,
}

local poltergeist = {}

poltergeist.controller = made.controller
-- 0x080E8CDA: the chip's `dimming` hook; `nearest`: its first parameter.
poltergeist.hook = made.hook

return poltergeist
