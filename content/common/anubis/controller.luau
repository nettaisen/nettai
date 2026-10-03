--!strict
-- Anubis's and PoisPhar's dimming controller, as BN5 and BN6 share it (BN6's effect object #0x33, `sub_80E40EC`;
-- BN5's #0x33, 0x080E411C, the same code; BN6's docs/engine/dimming-chip-effects.md §7): the usual dimming phases
-- (BN6's lib/dimming), with an effect that drops the chip's statue on the panel in front of the user, whatever is
-- there, and waits 120 ticks. Each game makes its own (`controller.make`) with its statue's spawn; a chip's
-- `dimming` hook is the made controller's `hook(variant)`.

local dimming_chips = require("@bn6/lib/dimming")

local controller = {}

export type State = { variant: AnubisVariant }

export type Spec = {
    -- The kind's id.
    id: string,
    -- The game's statue's spawn (@common/anubis/statue's).
    statue: (owner: Object, variant: AnubisVariant, x: number, y: number, damage: number) -> Object?,
}

local WAIT = 0x78

function controller.make(spec: Spec): { hook: (variant: AnubisVariant) -> (user: Object, dspec: DimmingChipSpec) -> Object? }
    -- `sub_80E4130`: the statue, then 120 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
            spec.statue(me, (me.state :: State).variant, me.panel_x + me.facing, me.panel_y, damage)
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            dimming_chips.done(me)
        end
    end

    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { variant = "record:anubis-variant" },
        -- `sub_80E40EC`.
        update = dimming_chips.phases { name = "Anubis", effect = effect },
    }

    -- A chip's `dimming` hook (`sub_80E4164`): the controller dropping a statue of `variant`.
    local function hook(variant: AnubisVariant): (user: Object, dspec: DimmingChipSpec) -> Object?
        return function(user: Object, dspec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, dspec)
            if o then
                (o.state :: State).variant = variant
            end
            return o
        end
    end

    return { hook = hook }
end

return controller
