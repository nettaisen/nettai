--!strict
-- The link navis' own chips (the original's action 0x0A, the entry the
-- user's own action table has past its idle: `off_80EA4C8[AIIndex][0xA]`):
-- what their routines share, any game's (no game's data in it; EXE6's
-- lib/link_chips is this). Each navi's chip is its own action, in its
-- folder (navis/heatman/chip.luau: HeatPres), which the chip runs; which
-- navi has which chip is the navi's (its own chip). See
-- docs/engine/standard-chips.md, "Action 0x0A".
--
-- Every routine runs its phases on the attack's step (0, 4, 8, ...), its
-- entry on `step_init`, and keeps its timer in its own state's `timer`
-- (the attack variables' halfword +0x10).

local link_chips = {}

-- `ldrh av+8; ldrh av+6; add; strh av+8`: the Atk+ bonus goes into the
-- damage (a halfword store), as each routine's first attacking phase does.
function link_chips.add_bonus(me: Object)
    me.attack_damage = me.attack_damage + me.extra
end

-- `ldr av+8`: the damage word the routines hand their hits and objects,
-- the damage (with its flag bits) and the hit parameter above it.
function link_chips.damage_word(me: Object): number
    return int.u32(me.attack_damage + me.hit_param * 0x10000)
end

-- The damage word's halves, as a hit region takes them.
function link_chips.damage(me: Object): number
    return bit32.band(link_chips.damage_word(me), 0xFFFF)
end

function link_chips.stamina(me: Object): number
    return bit32.rshift(link_chips.damage_word(me), 16)
end

-- `ldrh; sub #1; strh; bgt`: a phase's timer, counted down; true once it
-- has run out (the tick it reaches 0).
function link_chips.run_out(s: { timer: number }): boolean
    local t = s.timer - 1
    s.timer = t
    return t <= 0
end

-- `strh av+0`: go to phase `step`, from its entry (the halfword store
-- clears `step_init`).
function link_chips.go(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- `object_getFlip`: the flip value the sprite and effects take.
function link_chips.flip(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- `sub_800E258`: the panel under a position (the BIOS divisions).
function link_chips.panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- `sub_800E708(a, b, c)`: c lies between a and b, ends included (and a
-- or b equal to c).
function link_chips.between(a: number, b: number, c: number): boolean
    if a == c or b == c then
        return true
    end
    if a > b then
        return c >= b and c <= a
    end
    return c > a and c <= b
end

-- Run the phase the attack's step names from `phases` (the routine's
-- table, `name` in errors); a step past it is the game reading past its
-- table.
function link_chips.run<S>(me: Object, s: S, phases: { [number]: (Object, S) -> () }, name: string)
    local phase = phases[me.step]
    if phase == nil then
        error(string.format("%s: phase %#x reads past its table", name, me.step))
    end
    phase(me, s)
end

return link_chips
