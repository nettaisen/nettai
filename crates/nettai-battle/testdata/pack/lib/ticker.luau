--!strict
-- The test pack's family library (docs/design/content-model-v2.md §5): a
-- kind that counts its ticks, and a builder for actions that count theirs.

export type Variant = { lifetime: number }
export type State = { ticks: number, variant: Variant?, parent: any }

local ticker = {}

-- A kind: it counts ticks in its state and leaves after its variant's
-- lifetime (or `LIFETIME`). `parent` is the kind that spawned it.
local LIFETIME = 30

ticker.kind = define.kind {
    id = "test:test/ticker",
    pool = "effect",
    state = { ticks = "u16", variant = "record:ticker-variant", parent = "kind" },
    update = function(me: Object)
        local s = me.state :: State
        if me.kind ~= ticker.kind then
            error("a ticker's kind reads back as the ticker kind")
        end
        s.ticks += 1
        local lifetime = if s.variant then s.variant.lifetime else LIFETIME
        if s.ticks >= lifetime then
            me:free()
        end
    end,
}

-- A ticker variant: a record a ticker's state refers to.
function ticker.variant(v: Variant): Variant
    return define.record("ticker-variant", v)
end

-- One state for every action the builder makes: they continue each
-- other's attack state, as the original's actions of one number do.
local STATE = { ticks = "u16" }

export type Spec = { id: string?, length: number, next: any? }

-- An action that stands for `length` ticks, then starts `next` or ends.
-- While it runs, the navi runs it (`navi_action`).
function ticker.action(spec: Spec): any
    local def: any
    def = define.action {
        id = spec.id,
        state = STATE,
        args = spec,
        update = function(me: Object, s: { ticks: number })
            if me:navi_action() ~= def then
                error("the navi runs another action than this one")
            end
            s.ticks += 1
            if s.ticks >= spec.length then
                s.ticks = 0
                if spec.next then
                    me:set_attack(spec.next, 1)
                else
                    me:exit_attack()
                end
            end
        end,
    }
    return def
end

return ticker
