--!strict
-- BN5's ShakPar1-3 (action 0x1A's instant effect 22, 0x080E7FE0): BN5's own. An unseen shaker (effect object #0x66,
-- 0x080E7F60) on the user's panel sends SpShake's wave (chips/spshake/spshake: attack object #0x68) from the user's side's
-- back column (column 1, or 6 for a side facing left) along the bottom, the top and the middle rows (0x080E7FDC), 15
-- ticks apart, then goes. The waves are of its element and its parameters (the record's: the fastest speed, hits
-- that paralyze, the chip's palette).

local spshake = require("../spshake/spshake")

local shakpar = {}

-- The rows, in order.
local ROWS = { [0] = 3, [1] = 1, [2] = 2 }
-- A SpShake wave's parameters (chips/spshake/spshake's `Wave`).
type Wave = { speed: number, paralyzes: boolean, palette: number }
local WAVES, EVERY = 3, 15

export type State = {
    -- The waves sent (+0x60).
    sent: number,
    wave: Wave,
}

-- 0x080E7F8C: every 15 ticks a wave (the first at once); three.
local function update(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer, me.timer2 = 0, 1
        s.sent = 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.timer = EVERY
    local x = bit32.bxor(me.alliance, me.flip) * 5 + 1
    spshake.spawn(me, x, ROWS[s.sent], me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), s.wave)
    s.sent += 1
    if s.sent >= WAVES then
        me:set_lifecycle("destroy")
    end
end

shakpar.kind = define.kind {
    id = "bn5:shakpar/shaker",
    pool = "effect",
    state = { sent = "u8", wave = "record:shake-wave" },
    -- 0x080E7F60.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            me:set_lifecycle("update")
            update(me)
        elseif l == "update" then
            update(me)
        else
            me:free()
        end
    end,
}

-- The chips' `instant` hook (0x080E7FE0): the shaker on the user's panel, its side and facing, with the chip's
-- element and damage word, sending `wave`. (Its position is the dispatcher's registers: the user's row, the element
-- and the user's height.)
function shakpar.instant(wave: Wave): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local o = battle.spawn(shakpar.kind, Vec3.new(spec.panel_y, spec.element, spec.z))
        if not o then
            return
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o:set_damage_word(spec.damage)
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        ;(o.state :: State).wave = wave
    end
end

return shakpar
