--!strict
-- Snake's snakes and the holes they come out of (BN6's attack objects #0x72, the nest, and #0x73, the snake,
-- `sub_80D2BDC` and `sub_80D2EBC`; BN5's #0x53 and #0x54, 0x080D03E4's nest and the same snake code but for its
-- look and how it tells its nest it's gone). A snake rises out of its hole, waits, then leaps at its target panel
-- and strikes it, or flies on off the field when it has none. Each game's nest sends them out (BN6's
-- chips/snake/nest, BN5's chips/snake), from the holes the scan here finds, at the targets `target` picks.
--
-- A game makes its snake kind with `snake.make_kind(id, look)`.

local regions = require("@bn6/lib/regions")
local trajectory = require("@bn6/lib/trajectory")

local snake = {}

-- A game's snake: its sprite and sounds, the ticks it waits before its leap, its strike's element, spark, hit
-- modifier and collision types, and how it tells its nest it's out (`joined`, at its init) and gone (`left`, at its
-- end).
export type Look = SnakeLook

export type State = SnakeState

-- Rising two pixels a tick up to 10; then the wait; then the leap, 16 pixels a tick. The strike: 8 pixels up,
-- resolving while dimmed.
local RISE, RISE_TO, SPEED = 0x2_0000, 10, 0x10_0000
local STRIKE_Z = 0x8_0000
local ANIM_LEAP = 1

-- ---- The nest's scan (`sub_80D2CBE`, `sub_80D2D40`: both games' the same code) ---------------------------------

-- A hole on each side's area (`byte_80D2D30`): the side's own panel, not solid, with nothing on it or reserved.
local HOLE = { [0] = { require = 0x00, forbid = 0x0F8800B0 }, [1] = { require = 0x20, forbid = 0x0F880090 } }
-- The panels a snake aims at (`byte_80D2E28`, any of them): the other side's bodies and other bodies, or a neutral
-- object.
local TARGET = { [0] = 0x05800000, [1] = 0x0A800000 }

-- A nest's scan: where it is (Param3, Param4: bytes, the column stepping off the field when the scan ends).
export type Scan = SnakeScan

-- `sub_80D2D40`: the panel a snake from (hx, hy) aims at: of the panels from the column in front of the hole
-- forward (rows 1 to 3, until the column leaves the field) that hold a target, the first in that order, replaced by
-- one in a nearer row, then by one in a nearer column, then (a slip: the candidate's column against the best's row)
-- by one whose column isn't past the best's row. 0xFF when none.
function snake.target(me: Object, hx: number, hy: number): (number, number)
    local mask = TARGET[me.alliance]
    local candidates: { { number } } = {}
    local x = hx + me.facing
    while true do
        local stop = false
        for y = 1, 3 do
            if not field.valid(x, y) then
                stop = true
                break
            end
            if bit32.btest(field.flags(x, y), mask) then
                table.insert(candidates, { x, y })
            end
        end
        if stop then
            break
        end
        x += me.facing
    end
    if #candidates == 0 then
        return 0xFF, 0
    end
    local bx, by = 0, 0
    for _, c in candidates do
        local cx, cy = c[1], c[2]
        local take = true
        if bx ~= 0 then
            local d, bd = math.abs(cy - hy), math.abs(by - hy)
            if d > bd then
                take = false
            elseif d == bd then
                local e, be = math.abs(cx - hx), math.abs(bx - hx)
                if e > be then
                    take = false
                elseif e == be then
                    take = cx <= by
                end
            end
        end
        if take then
            bx, by = cx, cy
        end
    end
    return bx, by
end

-- `sub_80D2CBE`: from the scan's place on (rows 1 to 3, then the next column back), the first hole; the scan
-- resumes past it. The scan gives up once its column, plus one, passes 7 (a side 0 nest scans columns 6 to 1, then
-- 0 and 0xFF off the field; side 1's, 1 to 6). nil when none, else the hole and its target.
function snake.next_hole(me: Object, s: Scan): (number?, number, number, number)
    local cond = HOLE[me.alliance]
    local step = -me.facing
    local found = false
    local hx, hy = 0, 0
    while true do
        if found then
            break
        end
        if field.check(s.scan_x, s.scan_y, cond.require, cond.forbid) then
            found, hx, hy = true, s.scan_x, s.scan_y
        end
        s.scan_y += 1
        if s.scan_y > 3 then
            s.scan_y = 1
            local x = s.scan_x + step
            s.scan_x = x
            if x + 1 > 7 or x + 1 < 0 then
                break
            end
        end
    end
    if not found then
        return nil, 0, 0, 0
    end
    local tx, ty = snake.target(me, hx, hy)
    return hx, hy, tx, ty
end

-- ---- The snake --------------------------------------------------------------------------------------------------

-- `sub_80D3058`: 16 pixels a tick toward the target panel's center, or straight ahead without one.
local function aim(me: Object)
    local s = me.state :: State
    local x, y = field.center(me.panel_x, me.panel_y)
    if s.target_x == 0xFF then
        me.vel = Vec3.new(me.facing * SPEED, 0, me.vel.z)
        return
    end
    local tx, ty = field.center(s.target_x, s.target_y)
    local angle = trajectory.angle(int.i32(tx - x), int.i32(ty - y))
    local vx, vy = trajectory.toward(angle, SPEED)
    me.vel = Vec3.new(vx, vy, me.vel.z)
end

-- `sub_80D2F70`: rise out of the hole (its phase is the init flag).
local function rise(me: Object, look: Look)
    if me.phase == 0 then
        me.vel = Vec3.new(me.vel.x, me.vel.y, RISE)
        battle.play_sound(look.rise_sound)
        me.phase = 4
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, p.z + me.vel.z)
    if bit32.band(bit32.rshift(me.pos.z, 16), 0xFFFF) >= RISE_TO then
        me.action, me.phase, me.phase_init = 4, 0, 0
    end
end

-- `sub_80D2FA0`: its wait.
local function wait(me: Object, look: Look)
    if me.phase == 0 then
        me.timer = look.wait
        me.phase = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me.action, me.phase, me.phase_init = 8, 0, 0
    end
end

-- `sub_80D3004`: fly; on the target panel, strike it and go.
local function fly(me: Object, look: Look)
    local s = me.state :: State
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y + me.vel.y, p.z)
    me:set_panel_from_coordinates()
    if me.panel_x ~= s.target_x or me.panel_y ~= s.target_y then
        return
    end
    local h = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = look.strike_element,
        z = STRIKE_Z,
        region = regions.single,
        hit_effect = look.strike_spark,
        target = look.hits,
        self_type = look.is,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = look.strike_hit_mod,
    })
    if h then
        h.run_while_dimmed = true
    end
    me:set_lifecycle("destroy")
end

-- `sub_80D2FC2`: the leap, by phase: its animation; the sound at the animation's end; the flight.
local function leap(me: Object, look: Look)
    local phase = me.phase
    if phase == 0 then
        me.anim = ANIM_LEAP
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            battle.play_sound(look.leap_sound)
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        fly(me, look)
    else
        error(string.format("Snake's snake: leap phase %#x reads past off_80D2FD4", phase))
    end
end

-- `sub_80D2F3C`: gone once off the field; else the action and the sprite.
local function tick(me: Object, look: Look)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == 0 then
        rise(me, look)
    elseif a == 4 then
        wait(me, look)
    elseif a == 8 then
        leap(me, look)
    else
        -- off_80D2F60's fourth entry is a data word, not a routine.
        error(string.format("Snake's snake: action %#x reads past off_80D2F60", a))
    end
    me:update_sprite()
end

-- `sub_80D2EDC`: on a panel of the field, it joins its nest and shows on its hole, aimed; off it, it is freed at
-- once.
local function init(me: Object, look: Look)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    look.joined(me)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.visible = true
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "with_sprite"
    me.anim = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    aim(me)
    me:set_lifecycle("update")
    tick(me, look)
end

-- A game's snake kind `id`, of `look`.
function snake.make_kind(id: string, look: Look): Kind
    return define.kind {
        id = id,
        pool = "attack",
        state = { target_x = "u32", target_y = "u32", nest = "object", slot = "u8" },
        -- `sub_80D2EBC`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                -- sub_80D3048: out of the nest; freed.
                look.left(me)
                me:free()
            end
        end,
    }
end

-- `sub_80D30A4`: a snake of `kind` from `nest` (its side and facing) in the hole (`x`, `y`), leaping at (`tx`,
-- `ty`) (`tx` 0xFF: none), with `nest`'s damage word, in its nest's slot `slot`. (Spawned at the spawner's
-- registers; its init places it.)
function snake.spawn(kind: Kind, nest: Object, x: number, y: number, tx: number, ty: number, slot: number): Object?
    local o = battle.spawn(kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        local s = o.state :: State
        s.target_x, s.target_y, s.nest, s.slot = tx, ty, nest, slot
        o:set_damage_word(bit32.bor(nest.damage, bit32.lshift(nest.stamina, 16)))
        o.alliance, o.flip = nest.alliance, nest.flip
    end
    return o
end

return snake
