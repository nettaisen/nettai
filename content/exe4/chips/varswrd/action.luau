--!strict
-- VarSwrd's action (EXE4's action 0x36, 0x080ED6BE; NeoVari's 0x5A, 0x080EE1A2): the shared one (@exelib/varswrd/action)
-- with EXE4's part: the pick's sound (0x10B); in ProtoSoul (NaviStats +0x24 7) the navi waits while A is up. No auto
-- battle's pick: the original's (NaviStats +0x26 set: the side's record 0x08022D8A's +0x12) is only the story's other
-- navis', no netbattle's.

local common = require("@exelib/varswrd/action")

local action = {}

local GAME: VarSwordGame = {
    sound = asset.sound("sword-command"),
    waits = function(me: Object): boolean
        return battle.navi(me.alliance).form.id == "protosoul"
    end,
}

-- VarSwrd's (and NeoVari's) action: its sequences and what they give.
function action.action(spec: VarSwordSpec): Action
    return common.make(spec, GAME)
end

return action
