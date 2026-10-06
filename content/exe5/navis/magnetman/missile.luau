--!strict
-- MagnetMan's magnet missile (EXE5's attack object #0x63, 0x080D24F0; EXE6 keeps its code, `sub_80D0A40`, which no
-- EXE6 content spawns): a missile (sprite 10-2F, no shadow) 16 pixels up, with HP by its variant (its first
-- parameter), which the other side's attacks take. It waits (15 ticks, or 35 by its second parameter), then flies
-- forward 4 pixels a tick (a sound), lighting the solid panels under it. As it passes a panel's center it may
-- turn, once: with a target, when the target isn't ahead any more and is in another row; without one, toward a
-- panel of its column that holds an enemy navi's body, picked with a draw (only when the column has one). Turned,
-- it flies up or down its column 2.4 pixels a tick (animation 1 up, 2 down). A body it hits ends it, and so do the
-- field's edge, the battle's end and its HP running out (an explosion). It doesn't move while the battle is
-- dimmed. In the operation battle it marks the panel ahead each tick (lib/navi_chips/marker).

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local marker = require("../../lib/navi_chips/marker")
local navi_chips = require("@exelib/navi_chips/navi")
local sparks = require("../../lib/sparks")

local missile = {}

export type State = {
    -- Its first parameter (its HP's row, and its speeds': the same for every row), and its second (its wait's).
    variant: number,
    slow: number,
    -- +6: it has turned.
    turned: boolean,
    -- +0x50: what it follows (none: an enemy navi of its column).
    target: Object?,
}

local SPRITE = asset.sprite("magnet-missile")
local SOUND = asset.sound("junk-shot")
-- Its HP by the variant (0x080D24E4), and its wait by the second parameter (0x080D261C).
local HP: { [number]: number } = { [0] = 5, 12, 16, 20, 20 }
local WAIT: { [number]: number } = { [0] = 15, 35 }
-- Its speeds, 16.16 a tick (0x080D26B8 and 0x080D275C: every row the same).
local SPEED, TURNED_SPEED = 0x4_0000, 0x2_6666
local ANIM_UP, ANIM_DOWN = 1, 2
-- A panel holding the other side's navi's body, by side (0x080D27D8), and a body among what it hit.
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local BODY_HIT = 0x0F80_0000
local SOLID = 0x10

-- Its turn up or down its column toward row `y`, from the panel's center `cx`: none toward its own row.
local function turn(me: Object, y: number, cx: number)
    if y == me.panel_y then
        return
    end
    local down = y > me.panel_y
    me.vel = Vec3.new(0, if down then TURNED_SPEED else -TURNED_SPEED, me.vel.z)
    ;(me.state :: State).turned = true
    me.anim = if down then ANIM_DOWN else ANIM_UP
    local p = me.pos
    me.pos = Vec3.new(cx, p.y, p.z)
end

-- 0x080D2708: toward its target, once the target isn't ahead of it.
local function turn_to_target(me: Object, target: Object, cx: number)
    if (me.state :: State).turned then
        return
    end
    if (target.panel_x - me.panel_x) * me.facing > 0 then
        return
    end
    turn(me, target.panel_y, cx)
end

-- 0x080D2770: toward one of its column's panels that hold an enemy navi's body, by a draw.
local function turn_to_column(me: Object, cx: number)
    if (me.state :: State).turned then
        return
    end
    local rows = {}
    for y = 1, 3 do
        if field.check(me.panel_x, y, ENEMY_NAVI[me.alliance], 0) then
            table.insert(rows, y)
        end
    end
    if #rows == 0 then
        return
    end
    turn(me, rows[battle.rng_positive() % #rows + 1], cx)
end

-- 0x080D27E0: the operation battle's mark: the panel ahead in its row (its own at the field's edge columns), or,
-- turned, its column's middle row in another style.
local function mark(me: Object)
    local v = me.vel
    local dir = if v.x == 0 then 0 elseif v.x > 0 then 1 else -1
    local x = me.panel_x + dir
    if x < 1 or x > 6 then
        x = me.panel_x
    end
    if v.y ~= 0 then
        marker.spawn(me, x, 2, 1, 4, true)
    else
        marker.spawn(me, x, me.panel_y, 1, 1, true)
    end
end

-- Action 0 (0x080D25EC): its wait.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local ticks = WAIT[(me.state :: State).slow]
        if ticks == nil then
            error(string.format("a magnet missile's wait %d reads past its table (0x080D261C)", (me.state :: State).slow))
        end
        me.timer = ticks
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me.action = 4
        me.phase, me.phase_init = 0, 0
    end
end

-- Action 4 (0x080D2620): its flight.
local function fly(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND)
        me.vel = Vec3.new(SPEED * me.facing, me.vel.y, me.vel.z)
        return
    end
    local cx = field.center(me.panel_x, me.panel_y)
    local p, v = me.pos, me.vel
    local x = int.i32(p.x + v.x)
    me.pos = Vec3.new(x, int.i32(p.y + v.y), p.z)
    if cx ~= p.x and navi_chips.between(x, p.x, cx) then
        local target = s.target
        if target then
            turn_to_target(me, target, cx)
        else
            turn_to_column(me, cx)
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    mark(me)
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        -- (The state byte alone.)
        me.lifecycle = "destroy"
        return
    end
    if bit32.band(field.flags(me.panel_x, me.panel_y), SOLID) ~= 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
end

local ACTIONS = { [0] = wait, [4] = fly }

-- 0x080D258A: the hits against it resolve and spark; it goes at the battle's end, when its HP runs out (an
-- explosion) and when it hit a body; else its action (not while dimmed).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local gone = battle.over()
    if not gone and me:take_damage(0) < 0 then
        battle.effect(me.pos, effects.explosion)
        gone = true
    end
    if gone or bit32.band(c.hit_flags, BODY_HIT) ~= 0 then
        c:set_region(nil)
        -- (The state byte alone.)
        me.lifecycle = "destroy"
        return
    end
    if not battle.dimmed() then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("a magnet missile's action %#x reads past its table (0x080D25E0)", me.action))
        end
        action(me)
        if me.lifecycle ~= "update" then
            return
        end
    end
    c:present()
end

-- 0x080D2514: the panel under it; its sprite (animation 0, no shadow), shown, facing its side's way; its collision
-- (an object of its side, which the other side's attacks and blockers meet; hit modifier 3, the elec spark); its
-- HP by the variant.
local function init(me: Object)
    local s = me.state :: State
    me:set_panel_from_coordinates()
    s.turned = false
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_object, collision.hit_by_other_side_or_blockers, 3)
    me.collision:set_hit_effect(sparks.elec)
    me.collision:present()
    local hp = HP[s.variant]
    if hp == nil then
        error(string.format("a magnet missile's variant %d reads past its HP table (0x080D24E4)", s.variant))
    end
    me.hp, me.max_hp = hp, hp
    me:set_lifecycle("update")
end

missile.kind = new.kind {
    id = "magnetman/missile",
    pool = "attack",
    state = { variant = "u8", slow = "u8", turned = "bool", target = "object" },
    -- 0x080D24F0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            -- 0x080D26CC.
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080D26D4: a missile of `owner`'s side and facing at (x, y), 16 pixels up, with the element and the damage
-- word, of `variant`, following `target` if any; it is called while the battle is dimmed too.
function missile.spawn(owner: Object, x: number, y: number, element: number, variant: number, damage: number, target: Object?): Object?
    local o = battle.spawn(missile.kind, Vec3.new(x, y, 0x10_0000))
    if o then
        local s = o.state :: State
        s.variant, s.slow = variant, 0
        s.target = target
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return missile
