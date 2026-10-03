--!strict
-- The trap chips' dimming controller, as BN5 and BN6 share it (BN6's
-- effect object #0x2A, `sub_80E34C0`; BN5's #0x2A): AntiNavi, AntiDmg,
-- AntiSwrd, AntiRecv, ElemTrap and BodyGrd (BN5's anti chips too). Each game
-- makes its own kind (`traps.make_kind`). The usual dimming phases
-- (@common/dimming), with the telop hidden,
-- and an effect that makes the chip its user's side's defensive chip (the
-- linked record: the damage intake, a navi chip's arrival or a recovery
-- springs the trap), with its object for the chips that have one
-- (ElemTrap's trap), then waits 61 ticks. See docs/engine/chips.md §3.6.9.
--
-- A chip's `dimming` hook is `traps.hook(trap?)` (`sub_80E353E`,
-- `off_802CCB4[20]`). What the original's first chip parameter picked in
-- `off_80E3574` (0 ElemTrap's trap, 1 to 5 nothing) is the hook's
-- argument: the trap the chip sets, if any.

local dimming_chips = require("../dimming")

local traps = {}

-- (A trap with an object of its own is a "trap" record, `Trap`: `set`
-- spawns it for the controller and its user, `sub_80E3560`'s table entry.)

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32), which the side's record keeps for the trap's spring.
    chip: Chip?,
    bonus: number,
    -- The chip's trap object, if it has one.
    trap: Trap?,
}

local WAIT = 0x3C

-- `sub_80E3504`: the side's previous defensive chip goes; this one takes
-- its place (`sub_802CE8A`, on its user's side), with its object
-- (`sub_80E3560`); 61 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        battle.clear_linked(me.alliance)
        local user = me.related1
        if not user then
            error("the trap chip's controller has no user (sub_802CE8A reads through a null pointer)")
        end
        local s = me.state :: State
        local trap = s.trap
        local object = if trap then trap.set(me, user, s.chip, s.bonus) else nil
        battle.set_linked(user.alliance, {
            chip = s.chip,
            bonus = s.bonus,
            damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
            owner = user,
            object = object,
        })
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

-- A game's controller kind, keyed `id`.
function traps.make_kind(id: string): Kind
    return define.kind {
        id = id,
        pool = "effect",
        state = { chip = "chip", bonus = "u16", trap = "record:trap" },
        -- `sub_80E34C0`.
        update = dimming_chips.phases { name = "a trap chip", effect = effect, hidden = true },
    }
end

-- A trap's object, as a record a controller can keep.
function traps.trap(t: Trap): Trap
    return define.record("trap", t)
end

-- A chip's `dimming` hook (`sub_80E353E`): the controller (of `kind`) for
-- `user`, on its side, setting `trap`'s object if the chip has one. (The
-- spawner copies the user's side only, not its facing.)
function traps.hook(kind: Kind, trap: Trap?): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec, true)
        if o then
            local s = o.state :: State
            s.chip, s.bonus = spec.chip, spec.bonus
            s.trap = trap
        end
        return o
    end
end

return traps
