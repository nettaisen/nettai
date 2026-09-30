--!strict
-- The test pack's family library (docs/design/content-model-v2.md §5): a
-- kind that counts its ticks, and a builder for actions that count theirs.

export type State = { ticks: number }

local ticker = {}

-- A kind: it counts ticks in its state and leaves after `LIFETIME`.
local LIFETIME = 30

ticker.kind = define.kind {
    id = "test/ticker",
    pool = "effect",
    state = { ticks = "u16" },
    update = function(me: Object)
        local s = me.state :: State
        s.ticks += 1
        if s.ticks >= LIFETIME then
            me:free()
        end
    end,
}

-- One state for every action the builder makes: they continue each
-- other's attack state, as the original's actions of one number do.
local STATE = { ticks = "u16" }

export type Spec = { id: string?, length: number }

-- An action that stands for `length` ticks, then ends.
function ticker.action(spec: Spec): any
    return define.action {
        id = spec.id,
        state = STATE,
        args = spec,
        update = function(me: Object, s: State)
            s.ticks += 1
            if s.ticks >= spec.length then
                s.ticks = 0
                me:exit_attack()
            end
        end,
    }
end

return ticker
