--!strict
-- AqWhirl's whirlpool (EXE5's attack object #0x5D, 0x080D16E0; EXE5's own: EXE6 keeps the object's frame unused), set
-- down ahead by AqWhirl (@exelib/airspin/action's EXE5 variant 0). An attack at navis that hits each panel it comes
-- into (a hit turning a solid panel it is on to sea), it goes round a loop at its speed (its second parameter, times
-- 0x1000 a tick, both ways): down its column to the field's edge, forward to the first column holding an enemy navi
-- (or the edge), up to the edge, back to its first column and down to its first row; there it is gone, as it is off
-- a solid panel. Its palette is its first parameter's double.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local whirlpool = {}

export type State = {
    -- Its palette (Param0) and speed (Param1), and the panel it heads for (+0x60, +0x64).
    palette: number,
    speed: number,
    tx: number,
    ty: number,
}

local SPRITE = asset.sprite("aqua-whirl")
local SOUND = asset.sound("spout-beast-charge")
local HIT_MOD = 3
-- 0x080D1A78: the panel flag of an enemy navi, by its side.
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- The actions: down (or up) its column, forward, up, back, home.
local DOWN, FORWARD, UP, BACK, HOME = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_800E6E8`: `center` lies past `from`, up to and including `to`.
local function crossed(to: number, from: number, center: number): boolean
    if to > from then
        return center > from and center <= to
    end
    return center > to and center <= from
end

local function sign(v: number): number
    return if v > 0 then 1 else -1
end

-- The panel past its own in its column (vertical) or row, the way it moves; whether it is on the field.
local function aim(me: Object, vertical: boolean): boolean
    local s = me.state :: State
    if vertical then
        s.tx, s.ty = me.panel_x, me.panel_y + sign(me.vel.y)
    else
        s.tx, s.ty = me.panel_x + sign(me.vel.x), me.panel_y
    end
    return field.valid(s.tx, s.ty)
end

-- Its move this tick: whether it crossed its target panel's center.
local function move(me: Object, vertical: boolean): boolean
    local s = me.state :: State
    local cx, cy = field.center(s.tx, s.ty)
    local p, v = me.pos, me.vel
    if vertical then
        local y = int.i32(p.y + v.y)
        me.pos = Vec3.new(p.x, y, p.z)
        return crossed(y, p.y, cy)
    end
    local x = int.i32(p.x + v.x)
    me.pos = Vec3.new(x, p.y, p.z)
    return crossed(x, p.x, cx)
end

local function moved(me: Object)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- Onto its panel's center, the way it moved turned back, and on to `next`.
local function turn(me: Object, vertical: boolean, next: number)
    me:set_coordinates_from_panel()
    local v = me.vel
    if vertical then
        me.vel = Vec3.new(v.x, -v.y, v.z)
    else
        me.vel = Vec3.new(-v.x, v.y, v.z)
    end
    me:set_action(next)
end

local function gone(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D1A4A: column `x` holds an enemy navi (rows 1 to 3).
local function enemy_in_column(me: Object, x: number): boolean
    local flag = ENEMY_NAVI[me.alliance]
    for y = 1, 3 do
        if bit32.btest(field.flags(x, y), flag) then
            return true
        end
    end
    return false
end

-- A leg of its loop: `vertical` or not; past each panel's center, `stop` (if any) may end it at once (`true`: turn,
-- or "gone"); at the field's edge it turns, on to `next` (`nil`: gone instead).
local function leg(vertical: boolean, next: number?, stop: ((Object) -> (boolean | string))?): (Object) -> ()
    return function(me: Object)
        local function finish()
            if next then
                turn(me, vertical, next)
            else
                gone(me)
            end
        end
        if me.phase_init == 0 then
            me.phase_init = 4
            if not aim(me, vertical) then
                return finish()
            end
        end
        if not move(me, vertical) then
            return moved(me)
        end
        if stop then
            local r = stop(me)
            if r == "gone" then
                return gone(me)
            elseif r then
                return finish()
            end
        end
        if not aim(me, vertical) then
            return finish()
        end
        moved(me)
    end
end

local ACTIONS: { [number]: (Object) -> () } = {
    -- 0x080D17BC: down its column to the edge; then forward.
    [DOWN] = leg(true, FORWARD, nil),
    -- 0x080D1834: forward to the first column with an enemy navi, or the edge; then up.
    [FORWARD] = leg(false, UP, function(me: Object): boolean | string
        return enemy_in_column(me, me.panel_x)
    end),
    -- 0x080D18B4: up its column to the edge; then back.
    [UP] = leg(true, BACK, nil),
    -- 0x080D192C: back to its first column (gone there if on its first panel too); then down (or up).
    [BACK] = leg(false, HOME, function(me: Object): boolean | string
        if me.future_panel_x ~= me.panel_x then
            return false
        end
        return if me.future_panel_y == me.panel_y then "gone" else true
    end),
    -- 0x080D19BE: along its first column to its first row; gone there, or at the edge.
    [HOME] = leg(true, nil, function(me: Object): boolean | string
        return me.future_panel_y == me.panel_y
    end),
}

-- 0x080D174C: a hit lands (its region off, the panel it is on turned to sea if solid); it moves; on a new solid
-- panel it hits there; off a solid panel it is gone.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
        if field.solid(me.panel_x, me.panel_y) then
            field.set_type(me.panel_x, me.panel_y, "sea")
        end
    end
    local x, y = me.panel_x, me.panel_y
    local act = ACTIONS[me.action]
    if act == nil then
        error(string.format("AqWhirl's whirlpool: action %#x reads past its table (0x080D17A8)", me.action))
    end
    act(me)
    if field.solid(me.panel_x, me.panel_y) then
        if me.lifecycle ~= "destroy" and (me.panel_x ~= x or me.panel_y ~= y) then
            c:set_region(regions.single)
        end
    else
        gone(me)
    end
    c:present()
end

-- 0x080D1704: its sprite (with its shadow), an attack at navis; on its panel's center, on the ground, heading for
-- it; moving at its speed down and forward; its palette; a sound.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        error("AqWhirl's whirlpool: no collision slot (0x0800F1A0; the game reads through a null pointer)")
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.collision:present()
    local s = me.state :: State
    local speed = s.speed * 0x1000
    me.vel = Vec3.new(me.facing * speed, speed, me.vel.z)
    sprite.palette = s.palette * 2
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

whirlpool.kind = new.kind {
    id = "aqwhirl/whirlpool",
    pool = "attack",
    state = { palette = "u8", speed = "u8", tx = "i32", ty = "i32" },
    -- 0x080D16E0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        if me.active then
            me:update_sprite()
        end
    end,
}

-- 0x080D1A34 (with 0x0800F1F6): a whirlpool of `owner`'s on (x, y) in `palette` at `speed`, with the damage word
-- `damage`. Its position is the spawner's registers (the panel Y, the element and 0) until its init places it.
function whirlpool.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    palette: number,
    speed: number,
    damage: number
): Object?
    local o = battle.spawn(whirlpool.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.palette, s.speed = palette, speed
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- What AqWhirl sends: a whirlpool in `palette` at `speed` (the chip's parameters).
function whirlpool.sender(palette: number, speed: number): AirSpinSender
    return {
        send = function(user: Object, x: number, y: number, element: number, damage: number): Object?
            return whirlpool.spawn(user, x, y, element, palette, speed, damage)
        end,
    }
end

return whirlpool
