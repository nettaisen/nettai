--!strict
-- EXE5's Boxer1-3 (action 0x1A's instant effect 21, 0x080E7D68): EXE5's own. An unseen boxer (effect object #0x64,
-- 0x080E7CE0) on the user's panel, which doesn't run while the battle is dimmed, punches with FireHit's fist
-- (chips/firehit/fist: EXE5's fist, attack object #0x5B) three rounds of three, a punch every 12 ticks: down the middle
-- row, the top and the bottom (0x080E7D64). A punch (0x080E7D86) walks the row from the panel ahead of the boxer and
-- strikes the first panel with one of the other side's bodies or a neutral object on it (0x080E7E08); with none, a
-- random panel of those it walked.

local fist = require("../firehit/fist")
local common_fist = require("@exelib/firehit/fist")

local boxer = {}

-- The rows of a round, in order.
local ROWS = { [0] = 2, [1] = 1, [2] = 3 }
local PUNCHES, ROUNDS, EVERY = 3, 3, 12
-- What a punch looks for on a panel (0x080E7E08: the other side's bodies and the neutral objects).
local TARGETS = { [0] = 0x0580_0000, [1] = 0x0A80_0000 }

export type State = {
    -- The punch in the round (+0x60).
    punch: number,
    -- The fists' hit modifier (its parameters' second byte, the record's +0x11).
    hit_mod: number,
}

-- 0x080E7D86: a punch on row `y` from column `x` on.
local function punch(me: Object, x: number, y: number)
    local dir = me.facing
    local walked: { number } = {}
    local tx: number?, ty: number? = nil, nil
    while field.valid(x, y) do
        table.insert(walked, bit32.bor(x, bit32.lshift(y, 4)))
        if bit32.btest(field.flags(x, y), TARGETS[me.alliance]) then
            tx, ty = x, y
            break
        end
        x += dir
    end
    if tx == nil then
        if #walked == 0 then
            return
        end
        local p = walked[battle.rng_positive() % #walked + 1]
        tx, ty = bit32.band(p, 7), bit32.rshift(p, 4)
    end
    local s = me.state :: State
    common_fist.spawn(fist.kind, me, tx, ty :: number, me.element, 0, s.hit_mod, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
end

-- 0x080E7D0C: every 12 ticks a punch (the first at once); three rounds of three.
local function update(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer, me.timer2 = 0, ROUNDS
        s.punch = 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.timer = EVERY
    punch(me, me.panel_x + me.facing, ROWS[s.punch])
    s.punch += 1
    if s.punch < PUNCHES then
        return
    end
    s.punch = 0
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

boxer.kind = new.kind {
    id = "boxer/boxer",
    pool = "effect",
    state = { punch = "u8", hit_mod = "u8" },
    -- 0x080E7CE0.
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

-- The chips' `instant` hook (0x080E7D68): the boxer on the user's panel, its side and facing, with the chip's element
-- and damage word; `hit_mod`, the fists'. (Its position is the dispatcher's registers: the user's row, the element
-- and the user's height.)
function boxer.instant(hit_mod: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local o = battle.spawn(boxer.kind, Vec3.new(spec.panel_y, spec.element, spec.z))
        if not o then
            return
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o:set_damage_word(spec.damage)
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        o.run_while_dimmed = false
        ;(o.state :: State).hit_mod = hit_mod
    end
end

return boxer
