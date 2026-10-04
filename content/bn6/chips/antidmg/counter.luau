--!strict
-- AntiDmg's counter (the original's navi action 0x47, `sub_80EE90C`): the
-- shared counter (@exelib/antidmg/counter) with BN6's shuriken (./shuriken),
-- sounds and puff. The ruleset starts it by role
-- (`actions.anti_damage_counter`, rules/roles.luau).

local common = require("@exelib/antidmg/counter")
local effects = require("../../lib/effects")
local shuriken = require("./shuriken")

local counter = {}

-- What the ruleset starts: both of the counter's starters, a trap chip's
-- catch (`sub_801056A`) and the AntiDmg program's stance (`sub_80105F2`,
-- with the variant its weapon zeroes), leave the attack's variant 0: a
-- random enemy.
counter.action = common.action_at {
    id = "bn6:antidmg/counter",
    shuriken = shuriken.kind,
    vanish_sound = asset.sound("bn6:trap-vanish"),
    throw_sound = asset.sound("bn6:boomerang"),
    puff = effects.bubbles,
}

return counter
