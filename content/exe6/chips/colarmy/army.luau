--!strict
-- ColArmy's soldiers (the original's attack object #0x30, `sub_80C9D00`)
-- and the chip's effect (instant chip effect 22, `sub_80C9ECE`): every
-- obstacle on the field (but NameID 0xDA and those already leaving) blinks
-- out and a soldier takes its panel. Hidden for 18 ticks, it appears, and 30
-- ticks later fires three shots (objects/bullet, the vulcan's hit, at a
-- random height) at the panel in front of it, 10 ticks apart, then leaves
-- when its animation ends. It faces the other side's navi if there is one
-- ahead on its row, else turns around if one is anywhere on its row (a
-- draw) or it stands on the other side's back column. It holds its panel
-- and stands still while dimmed.

local bullet = require("../../objects/bullet")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local army = {}

local SPRITE = asset.sprite("col-army")
local APPEAR_SOUND, SHOT_SOUND = asset.sound("col-army"), asset.sound("col-army-2")
-- Its collision: an object of its side that the other side's attacks,
-- objects and bodies (and blockers) reach; hit modifier 3; the plain
-- spark.
local HIT_MOD = 3
-- The other side's navis (by its own side), for its facing
-- (`off_80C9F64`, `off_8109784`).
local NAVIS = { [0] = 0x04000000, [1] = 0x08000000 }
-- The shots: bullets (row 0x11), three of them 10 ticks apart, at one of
-- four heights (`dword_80C9E84`, in pixels).
local SHOT = bullet.variants.col_army
local SHOTS, SHOT_TICKS = 3, 0xA
local HEIGHTS = { [0] = 0x08, 0x10, 0x18, 0x20 }
local WAIT_TICKS, SHOW_AT = 0x30, 0x1E

-- The actions.
local WAIT, FIRE, LEAVE = 0, 4, 8

-- `sub_800D120`: the farthest of the other side's navis ahead on its row
-- (from its own panel on, the way it faces), unless that is its own panel.
local function navi_ahead(me: Object): boolean
    local step = me.facing
    local x, y = me.panel_x, me.panel_y
    local found = x
    local require = NAVIS[me.alliance]
    while true do
        if field.check(x, y, require, 0) then
            found = x
        end
        x += step
        if not field.valid(x, y) then
            break
        end
    end
    return found ~= me.panel_x
end

-- `sub_810971A`: whether any of the other side's navis is on its row (one
-- draw, picking one of them, if so).
local function navi_on_row(me: Object): boolean
    local require = NAVIS[me.alliance]
    local n = 0
    for x = 1, 6 do
        if field.check(x, me.panel_y, require, 0) then
            n += 1
        end
    end
    if n == 0 then
        return false
    end
    battle.rng_positive()
    return true
end

-- `sub_80C9F18`: which way it faces; the sprite's flip.
local function face(me: Object): number
    local flip = 0
    if not navi_ahead(me) then
        if navi_on_row(me) or bit32.bxor(me.alliance, 1) * 5 + 1 == me.panel_x then
            flip = 1
        end
    end
    me.flip = flip
    return bit32.bxor(flip, me.alliance)
end

-- `sub_80C6ADA` and `sub_801155A`: a shot at (x, y), `z` up.
local function shot(me: Object, x: number, y: number, z: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    bullet.fire(me, x, y, z, SHOT, me.element, damage)
end

-- `sub_80C9DE0`: hidden, then shown; then fire.
local function wait(me: Object)
    if me.timer == SHOW_AT then
        me.visible = true
        me.anim, me.anim_loaded = 0, 0xFF
        me.collision:set_region(nil)
        battle.play_sound(APPEAR_SOUND)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.anim = 1
    me.timer2 = SHOTS
    me.action = FIRE
    me.phase, me.phase_init = 0, 0
end

-- `sub_80C9E20`: three shots.
local function fire(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.timer = SHOT_TICKS
        local z = bit32.lshift(HEIGHTS[bit32.band(battle.rng_positive(), 3)], 16)
        shot(me, me.panel_x + me.facing, me.panel_y, z)
        battle.play_sound(SHOT_SOUND)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local shots = me.timer2 - 1
    me.timer2 = shots
    if shots > 0 then
        me.phase = 0
        return
    end
    me.anim = 2
    me.action = LEAVE
    me.phase, me.phase_init = 0, 0
end

-- `sub_80C9E88`: gone when its animation ends.
local function leave(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C9D9E`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if not battle.dimmed() then
        local a = me.action
        if a == WAIT then
            wait(me)
        elseif a == FIRE then
            fire(me)
        elseif a == LEAVE then
            leave(me)
        else
            error(string.format("ColArmy soldier action %#x reads past its table", a))
        end
    end
    c:present()
end

-- `sub_80C9D24`.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    sprite:set_flip(face(me))
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    me.timer = WAIT_TICKS
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_object, collision.hit_by_other_side_or_blockers, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.plain)
    c:set_region(nil)
    c:present()
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    me:set_lifecycle("update")
    tick(me)
end

army.kind = new.kind {
    id = "colarmy/army",
    pool = "attack",
    -- `sub_80C9D00`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_paused()
    end,
}

-- `sub_80C9EA6`: a soldier of `user`'s on (x, y), with the damage word
-- `damage`; it runs while dimmed. (It spawns at the registers' values,
-- which its init replaces.)
local function spawn(user: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(army.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
        o.related1 = user
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80C9ECE`, instant chip effect 22: the registry's obstacles
-- (`sub_80C9EE6`).
function army.instant(user: Object, spec: InstantChipSpec)
    for _, o in field.all_objects() do
        -- (Not what nothing absorbs: a mine.)
        local identity = o.identity
        if not (identity and identity.absorbable == false) and obstacle.present(o) then
            obstacle.vanish(o)
            spawn(user, o.panel_x, o.panel_y, spec.element, spec.damage)
        end
    end
end

return army
