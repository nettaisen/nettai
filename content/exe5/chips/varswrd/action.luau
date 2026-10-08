--!strict
-- VarSwrd's action (EXE5's action 0x2A, 0x080ED4A2; NeoVari's 0x31): the shared one (@exelib/varswrd/action) with
-- EXE5's part: the pick's sound (0x8B); the navi waits while A is up in the souls whose forms say so
-- (`var_sword_waits`: ProtoSoul and ShadowSoul, NaviStats +0x2C 1 and 8); and a navi in auto battle (NaviStats +0x2A,
-- never set in a Team Battle, or AIData +0xF0) takes the side's sword pick (0x0802D4E2's +0x12, which its AI draws: the
-- picks by their place, one past the last a Sword). +0xF0 is set by 0x0802C110 alone: the story navis' routines and
-- DarkInvs (0x080E231A, its user in auto battle for 600 ticks), not Chaos Unison's Dark MegaMan's (AI index 0x16:
-- 0x08104234), who takes the joypad path.

local common = require("@exelib/varswrd/action")

local action = {}

local GAME: VarSwordGame = {
    sound = asset.sound("ok-8b"),
    -- (The souls that wait while A is up, by NaviStats +0x2C: their forms' `var_sword_waits`.)
    waits = function(me: Object): boolean
        return battle.navi(me.alliance).form.var_sword_waits == true
    end,
    -- `sub_802C106`: AIData +0xF0 (the navi's `in_auto_battle`), which only DarkInvs's drive sets in a battle
    -- (chips/darkinvs, rules/auto_battle). (NaviStats +0x2A, which the pick reads first, no Team Battle sets.)
    auto_pick = function(me: Object): number?
        if me.in_auto_battle then
            return battle.sword_pick(me.alliance)
        end
        return nil
    end,
}

-- VarSwrd's (and NeoVari's) action: its sequences and what they give.
function action.action(spec: VarSwordSpec): Action
    return common.make(spec, GAME)
end

return action
