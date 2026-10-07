--!strict
-- A soul's turns (AIData +0x18, the custom screens asked for since the status reset, 0xFF none to count; +0x19, how
-- many the form lasts, 0x080180A8's by soul: each soul's form's `soul.turns`, 3 for every one; none, 0, for the base
-- form): the status reset starts them (0x0800BA3C: at the navi's start, the change's and the revert's), each custom
-- screen asked for counts one (0x0800BA20, both sides, from the fight: 0x08007192), and once a soul's reach its limit
-- the intake asks for the revert (./init: 0x0800B6B8). The emotion window counts down the turns left (0x08016A20: the
-- offer view's `form_offer.turns` and `form_offer.turn_limit`). A navi that starts the round in a soul (NaviStats
-- +0x24 set at its init, 0x08010830: none in a netbattle, whose navis start in their base form) never counts them.

local turns = {}

export type State = {
    -- AIData +0x18 and +0x19.
    soul_turns: number,
    soul_turn_limit: number,
}

-- None to count.
turns.NONE = 0xFF

-- 0x0800BA3C, as the status reset runs in `form`: none counted yet, toward the form's turns.
function turns.start(form: FormDef)
    local s = rules.state() :: State
    local soul: { turns: number }? = form.soul
    s.soul_turns = 0
    s.soul_turn_limit = if soul ~= nil then soul.turns else 0
end

-- 0x0800BA20: a custom screen asked for counts one, unless none are counted.
function turns.custom_requested(_side: number)
    local s = rules.state() :: State
    if s.soul_turns ~= turns.NONE then
        s.soul_turns += 1
    end
end

-- 0x0800B6B8: the soul's turns are over.
function turns.over(): boolean
    local s = rules.state() :: State
    return s.soul_turns ~= turns.NONE and s.soul_turns >= s.soul_turn_limit
end

-- 0x0801FF8C: the side's navi's turns are counted (Double Soul can be offered).
function turns.counted(): boolean
    return (rules.state() :: State).soul_turns ~= turns.NONE
end

return turns
