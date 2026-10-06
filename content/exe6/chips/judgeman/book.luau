--!strict
-- JudgeMan's book (the original's attack object #0x94, `sub_80D7ACC`): it
-- appears on a panel the other side took from his (his sprite), waits its
-- turn (12 ticks apart), then flies at an enemy navi, 2.25 pixels a tick
-- along one axis at a time, turning at panel centers; it hits what it meets
-- and ends on the other side's body or object, at its target's panel, or
-- off solid ground. Going (or failing to start), it counts itself done in
-- JudgeMan's state. It acts while dimmed. See docs/engine/chips.md §3.6.29.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local book = {}

export type State = {
    -- Its turn among the books (Param1): it waits 12 ticks more per turn.
    turn: number,
    -- The axis it last flew along (Param2): 0 x, 1 y.
    axis: number,
    -- Whose `books_done` it adds to (ExtraVars[0], a pointer to it).
    judge: Object?,
    -- Its target panel (ExtraVars[1] and [2]).
    target_x: number,
    target_y: number,
}

-- What a book's judge keeps.
type Judge = { books_done: number }

local SPRITE = asset.sprite("judgeman")
local SOUND_APPEAR = asset.sound("appear")
local ANIM_WAIT, ANIM_FLY = 0xA, 0xB

-- Its speed (16.16).
local SPEED = 0x2_4000

-- What ends it when it hits, by side (`byte_80D7BA8`: the other side's body
-- or object, and neutral ones).
local ENDS = { [0] = 0x0580_0000, [1] = 0x0A80_0000 }

-- Where it aims, by side (`off_80D7D44`: an enemy navi's body).
local ENEMY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }

-- Its actions.
local WAIT, FLY = 0, 4

-- Count one book done.
local function count(judge: Object?)
    if judge then
        local j = judge.state :: Judge
        j.books_done += 1
    end
end

-- `sub_80D7AF0`: on its panel, on the ground (Z's whole part cleared);
-- collision on (without it, it counts itself done and goes).
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = ANIM_WAIT
    me.anim_loaded = ANIM_WAIT
    sprite:set_animation(ANIM_WAIT)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        count((me.state :: State).judge)
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, 1)
    me.collision:set_hit_effect(sparks.heavy)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80D7CCC`: its target: the first enemy navi along its row from its
-- side's back column; else the first column (from there) with one, its
-- first row with one; else past the far edge in its own row.
local function find_target(me: Object): (number, number)
    local front = me.facing
    local enemy = ENEMY[me.alliance]
    local back = if me.alliance == 0 then 1 else 6
    local x = back
    while true do
        if field.check(x, me.panel_y, enemy, 0) then
            return x, me.panel_y
        end
        x += front
        if not field.valid(x, me.panel_y) then
            break
        end
    end
    x = back
    while true do
        for y = 1, 3 do
            if field.check(x, y, enemy, 0) then
                return x, y
            end
        end
        x += front
        if x < 1 or x > 6 then
            break
        end
    end
    return if me.alliance == 0 then 7 else 0, me.panel_y
end

-- `sub_80D7CB0`: face the way it flies (unchanged when it flies up or
-- down; raw flips, not by side).
local function face(me: Object)
    local vx = me.vel.x
    if vx > 0 then
        me.sprite:set_flip(0)
    elseif vx < 0 then
        me.sprite:set_flip(1)
    end
end

-- `sub_80D7D4C`: its heading toward the target, back on its panel's
-- center: along the axis other than the one it flew if the target is off
-- along it, else the same; after two tries, turn back.
local function head(me: Object)
    local s = me.state :: State
    local tries = 0
    local along_x = s.axis ~= 0
    while true do
        if tries >= 2 then
            local v = me.vel
            me.vel = Vec3.new(-v.x, -v.y, v.z)
            break
        end
        tries += 1
        if along_x then
            if s.target_x ~= me.panel_x then
                s.axis = 0
                local vx = if s.target_x > me.panel_x then SPEED else -SPEED
                me.vel = Vec3.new(vx, 0, me.vel.z)
                break
            end
        else
            if s.target_y ~= me.panel_y then
                s.axis = 1
                local vy = if s.target_y > me.panel_y then SPEED else -SPEED
                me.vel = Vec3.new(0, vy, me.vel.z)
                break
            end
        end
        along_x = not along_x
    end
    me:set_coordinates_from_panel()
    face(me)
end

-- `sub_800E708`: `c` lies between `a` and `b` (either may equal it).
local function between(a: number, b: number, c: number): boolean
    if a == c or b == c then
        return true
    end
    if a <= b then
        return c > a and c <= b
    end
    return c >= b and c <= a
end

-- `sub_800E258`: the panel under a position.
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- Region off, and gone (a byte store of the lifecycle: the action stays).
local function finish(me: Object)
    me.collision:set_region(nil)
    me.lifecycle = "destroy"
end

-- `sub_80D7BB0`: wait 30 + 12 ticks a turn (to -1).
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_APPEAR)
        me.timer = (me.state :: State).turn * 0xC + 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_action(FLY)
    end
end

-- `sub_80D7BE0`: fly; at each panel center on its axis it has arrived (its
-- target's panel: it goes) or heads again; off solid ground it goes.
local function fly(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        me.anim = ANIM_FLY
        s.target_x, s.target_y = find_target(me)
        head(me)
    end
    local cx, cy = field.center(me.panel_x, me.panel_y)
    local along_x = s.axis == 0
    local center = if along_x then cx else cy
    local p, v = me.pos, me.vel
    local nx, ny = int.i32(p.x + v.x), int.i32(p.y + v.y)
    me.pos = Vec3.new(nx, ny, p.z)
    local old, new = p.y, ny
    if along_x then
        old, new = p.x, nx
    end
    if center ~= old and between(new, old, center) then
        local px, py = panel_at(nx, ny)
        if px == s.target_x and py == s.target_y then
            return finish(me)
        end
        head(me)
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not bit32.btest(field.flags(me.panel_x, me.panel_y), 0x10) then
        finish(me)
    end
end

-- `sub_80D7B5C`: resolve the hits (hitting the other side's body or object
-- ends it), act, register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or bit32.btest(c.hit_flags, ENDS[me.alliance]) then
        return finish(me)
    end
    local a = me.action
    if a == WAIT then
        wait(me)
    elseif a == FLY then
        fly(me)
    else
        error(string.format("JudgeMan's book action %#x reads past its table (off_80D7B9C)", a))
    end
    c:present()
end

book.kind = new.kind {
    id = "judgeman/book",
    pool = "attack",
    state = { turn = "u8", axis = "u8", judge = "object", target_x = "u32", target_y = "u32" },
    -- `sub_80D7ACC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80D7C6E
            count((me.state :: State).judge)
            me:destroy()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80D7C7E`: a book of `judge`'s on (x, y), its `turn`th, flying along
-- x first (Param2 1), with `element` and the damage word, counting itself
-- done in `judge`'s state (at once if it can't spawn). `z` is the
-- spawner's register that becomes its Z, of which the init keeps the
-- fraction.
function book.place(judge: Object, x: number, y: number, turn: number, element: number, damage: number, z: number): Object?
    local o = battle.spawn(book.kind, Vec3.new(y, element, z))
    if not o then
        count(judge)
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.related1 = judge
    o:set_damage_word(damage)
    o.alliance, o.flip = judge.alliance, judge.flip
    o.run_while_dimmed = true
    local s = o.state :: State
    s.turn, s.axis, s.judge = turn, 1, judge
    return o
end

return book
