--!strict
-- The snowball's roller, as EXE6 and EXE5 share it (EXE6's attack object
-- #0xB7, `sub_80DC260`; EXE5's #0xB7, 0x080DD234; docs/engine/
-- dimming-chip-effects.md §10): an invisible collision 20 pixels ahead of
-- the snowball (./ball) that reaches every attack, object and body; of
-- what it touches, the first field object it may swallow
-- (`obstacle.swallowable`) is removed, and the ball grows. It goes with the
-- ball, at the field's end, or when the battle is over. Each game makes its
-- own kind (`roller.make_kind`: what it is and reaches, its table's rows:
-- EXE6's 0x58 and 0x49, EXE5's 0x48 and 0x49).

local regions = require("../regions")

local roller = {}

export type State = {
    -- It swallowed a field object this tick (the original's first
    -- parameter), which the ball reads next tick.
    swallowed: boolean,
}

-- A game's roller: its kind's key, and what it is and reaches.
export type Look = {
    id: string,
    self_type: CollisionType,
    target: CollisionType,
}

-- What it tells the ball.
type BallState = { roller: Object? }

-- How far ahead of the ball it sits: 4 pixels at first, then 20.
local START_AHEAD, AHEAD = 4, 20
-- How many of the objects it touched it looks through (`sub_801A4DC`'s
-- room: four).
local LOOKS_AT = 4
local SOLID = 0x10

-- Its X: the ball's whole pixels plus `px` toward its front (the
-- fraction its own).
local function follow(me: Object, ball: Object, px: number)
    local x16 = int.u16(bit32.rshift(ball.pos.x, 16) + px * me.facing)
    me.pos = Vec3.new(int.i32(bit32.bor(bit32.lshift(x16, 16), bit32.band(me.pos.x, 0xFFFF))), me.pos.y, me.pos.z)
end

-- The ball forgets it; it goes (destroyed next update).
local function leave(me: Object)
    me.collision:set_region(nil)
    local ball = me.related1
    if ball and ball.active then
        (ball.state :: BallState).roller = nil
    end
    me:set_lifecycle("destroy")
end

-- `sub_80DC310`: swallow the first field object among what it touched;
-- then on ahead of the ball, hitting again in a new column; gone where the
-- panel isn't solid or off the field.
local function roll(me: Object)
    local s = me.state :: State
    s.swallowed = false
    local c = me.collision
    if c.hit_flags ~= 0 then
        local found: Object? = nil
        for i, o in c:hit_by() do
            if i > LOOKS_AT then
                break
            end
            if obstacle.swallowable(o) then
                found = o
                break
            end
        end
        if found then
            me.related2 = found
            obstacle.remove(found)
            c:set_region(nil)
            s.swallowed = true
        else
            c:set_region(nil)
        end
    end
    local x = me.panel_x
    follow(me, me.related1 :: Object, AHEAD)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= x then
        c:set_region(regions.single)
    end
    if not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) or not field.valid(me.panel_x, me.panel_y) then
        leave(me)
    end
end

-- `sub_80DC2D4`: resolve last tick's hits; at the battle's end, gone; else
-- roll and present.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if battle.over() then
        leave(me)
        return
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("the snowball's roller: action %#x reads past off_80DC30C", a))
    end
    roll(me)
    me.collision:present()
end

-- `sub_80DC280`: just ahead of the ball, on its panel; its collision on
-- no panel yet. Without a collision, the ball forgets it and it is freed.
-- (The update first runs next tick.)
local function init(me: Object, look: Look)
    local ball = me.related1 :: Object
    follow(me, ball, START_AHEAD)
    local frac = function(v: number): number
        return bit32.band(v, 0xFFFF)
    end
    local b = ball.pos
    me.pos = Vec3.new(
        me.pos.x,
        int.i32(bit32.bor(bit32.band(b.y, 0xFFFF0000), frac(me.pos.y))),
        int.i32(bit32.bor(bit32.band(b.z, 0xFFFF0000), frac(me.pos.z)))
    )
    me:set_panel_from_coordinates()
    me:set_damage_word(0)
    ;(me.state :: State).swallowed = false
    if not me:create_collision() then
        (ball.state :: BallState).roller = nil
        me:free()
        return
    end
    me:setup_collision(look.self_type, look.target, 0)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    me:set_lifecycle("update")
end

-- A game's roller kind.
function roller.make_kind(look: Look): Kind
    return define.kind {
        id = look.id,
        pool = "attack",
        state = { swallowed = "bool" },
        -- `sub_80DC260`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
        end,
    }
end

-- `sub_80DC38A`: the roller (of `kind`) of `ball`, of its side and facing,
-- running while dimmed; the ball keeps it. Its position is the spawner's
-- registers, whose fractions it keeps: what the ball's sprite flip left
-- (the sprite's +0x16 byte, 8, with the flip's bits, and those bits; and
-- the sprite's RAM address, which the comparison skips).
function roller.spawn(kind: Kind, ball: Object): Object?
    local flip_bits = bit32.band(bit32.lshift(bit32.bxor(ball.alliance, ball.flip), 4), 0x30)
    local o = battle.spawn(kind, Vec3.new(bit32.bor(8, flip_bits), flip_bits, 0))
    if not o then
        return nil
    end
    o.related1 = ball
    ;(ball.state :: BallState).roller = o
    o.alliance, o.flip = ball.alliance, ball.flip
    o.run_while_dimmed = true
    return o
end

return roller
