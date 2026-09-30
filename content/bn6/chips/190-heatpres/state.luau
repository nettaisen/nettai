--!strict
-- The link navis' chips (action 0x0A, chip.luau): the schema of what they
-- keep in the attack's variables (LinkChipState, types.d.luau: one for the
-- ten routines, each using the fields its routine does) and the steps
-- they share.

type State = LinkChipState

local state = {
    schema = { timer = "u16", target_x = "u8", target_y = "u8", held = "object", speed = "i32" },
}

-- The Atk+ bonus goes into the damage (`ldrh av+8; ldrh av+6; add;
-- strh av+8`), as every routine's first phase does.
function state.add_bonus(me: Object)
    me.attack_damage = me.attack_damage + me.extra
end

-- The damage word the routines hand their objects (`ldr av+8`): the
-- damage and the hit parameter above it.
function state.damage_word(me: Object): number
    return me.attack_damage + me.hit_param * 0x10000
end

-- A phase's timer (`ldrh; sub #1; strh; bgt`): true once it has run out.
function state.count_down(s: State): boolean
    local t = s.timer - 1
    s.timer = t
    return t <= 0
end

-- Go to phase `step` from its entry (`strh av+0`).
function state.go(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

return state
