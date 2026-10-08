--!strict
-- EXE5's Quake bomb (attack object #0x51, 0x080CFE0C; EXE5's own, thrown by action 0x12's subtype 11, 0x080D007C):
-- lobbed at the panel three ahead at 4 pixels a tick, it hangs there 48 pixels up as a weight in its chip's colors
-- for 25 ticks, then drops 8 pixels a tick. Landing on a solid panel, it shakes the screen, cracks the panel and hits
-- its chip's region around it (Quake1 the panel, Quake2 the column, Quake3 the panel and its four neighbors), an
-- explosion on each panel; off solid ground it hits the panel alone. (The original reads both by the chip's level,
-- Param1: 0x080CFFE4, 0x080D0060.) Then it goes in a puff. Off the field, it goes
-- before it drops.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local trajectory = require("../../lib/trajectory")

local quake = {}

export type State = {
    -- Its chip's palette and region.
    palette: number,
    region: Region,
}

-- 0x080CFE2C: the thrown bomb, sprite 0c-02 in animation 1; 0x080CFF9A: the weight, 04-02 in animation 3.
local BOMB, WEIGHT = asset.sprite("bomb"), asset.sprite("quake-drop")
-- Its speed along the ground and its fall while lobbed (16.16), its height over the panel, and its drop.
local SPEED, GRAVITY, HANG_Z, DROP = 0x40000, -0x3C00, 0x30 * 0x10000, -0x80000
local HANG_TICKS = 25
-- Effect 0 (the explosion) on each panel it hits; effect 0x12 (a puff) as it goes.
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local SOUND_LAND = asset.sound("land")
-- What it is and hits: EXE5's rows 0x15 and 5, hit modifier 3.
local HIT_MOD = 3

-- 0x080CFED8: lobbed at the panel; over it, it hangs (else off the field it goes).
local function fly(me: Object)
    if me.phase_init == 0 then
        local p = me.pos
        me.pos = Vec3.new(p.x + me.facing * 4 * 0x10000, p.y, p.z + 0x30 * 0x10000)
        local tx, ty = field.center(me.panel_x, me.panel_y)
        p = me.pos
        local angle = trajectory.angle(int.i32(tx - p.x), int.i32(ty - p.y))
        local vx, vy = trajectory.toward(angle, SPEED)
        me.vel = Vec3.new(vx, vy, me.vel.z)
        me.pos = Vec3.new(p.x, p.y, HANG_Z)
        local rise, ticks = trajectory.at_speed(me.pos, vy, tx, ty, HANG_Z, SPEED, GRAVITY)
        me.vel = Vec3.new(vx, vy, rise)
        me.timer = ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t > 0 then
        local v = me.vel
        local vz = int.i32(v.z + GRAVITY)
        me.vel = Vec3.new(v.x, v.y, vz)
        local p = me.pos
        me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z + vz)
        return
    end
    if field.valid(me.panel_x, me.panel_y) then
        local cx, cy = field.center(me.panel_x, me.panel_y)
        me.pos = Vec3.new(cx, cy, HANG_Z)
        me.action = 4
        return
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080CFF9A: the weight in its chip's colors.
local function hang(me: Object, s: State)
    local sprite = me.sprite
    sprite:load(WEIGHT)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 3, 0xFF
    sprite:set_animation(0xFF)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = s.palette
    -- (In the operation battle, which a netbattle never sets, 0x080E9FA4 spawns effect object 0x83 at the panel for 60
    -- ticks: not ported, exe5-map.md §15.6.)
    me.action = 8
    me.phase, me.phase_init = 0, 0
end

-- 0x080CFFE8: 25 ticks, then the drop.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = HANG_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, DROP)
        me.action = 12
    end
end

-- 0x080D000A: the drop; landed, its hit.
local function drop(me: Object, s: State)
    local z = me.pos.z
    local next = int.u32(z + me.vel.z)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, int.i32(next))
    -- (Falling while the unsigned add carries and leaves more than nothing.)
    if int.u32(z) + int.u32(me.vel.z) > 0xFFFFFFFF and next ~= 0 then
        return
    end
    battle.play_sound(SOUND_LAND)
    me.pos = Vec3.new(p.x, p.y, 0)
    local region = regions.single
    if field.solid(me.panel_x, me.panel_y) then
        battle.shake_camera(2, 35)
        field.crack(me.panel_x, me.panel_y)
        region = s.region
    end
    local c = me.collision
    c:set_region(region)
    battle.region_effects(c.panel_x, c.panel_y, region, me.alliance, EXPLOSION, 0)
    me.action = 16
end

-- 0x080D0064: the region gone, a puff, and gone.
local function leave(me: Object)
    me.collision:set_region(nil)
    battle.effect(me.pos, PUFF)
    me:set_lifecycle("destroy")
end

-- 0x080CFEA4: this tick's step, between its collision's turns.
local function tick(me: Object)
    local s = me.state :: State
    me.collision:remove()
    local a = me.action
    if a == 0 then
        fly(me)
    elseif a == 4 then
        hang(me, s)
    elseif a == 8 then
        wait(me)
    elseif a == 12 then
        drop(me, s)
    elseif a == 16 then
        leave(me)
    else
        error(string.format("Quake's bomb action %#x reads past its table", a))
    end
    me:update_sprite()
    me.collision:present()
end

quake.kind = new.kind {
    id = "quake/bomb",
    pool = "attack",
    state = { palette = "u8", region = "region" },
    -- 0x080CFE0C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080CFE2C: the bomb, its collision (no region yet) on the panel three ahead.
            me.visible = true
            local sprite = me.sprite
            sprite:load(BOMB)
            me.anim, me.anim_loaded = 1, 0xFF
            sprite:set_animation(0xFF)
            sprite.shadow = "ground"
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            if not me:create_collision() then
                me:free()
                return
            end
            me:setup_collision(collision.thrown_break, collision.hits_navis, HIT_MOD)
            me.collision:set_region(nil)
            me.collision:present()
            me:set_panel_from_coordinates()
            local x, y = me.panel_x + 3 * me.facing, me.panel_y
            me.panel_x, me.panel_y = x, y
            me.collision.panel_x, me.collision.panel_y = x, y
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- 0x080D007C: a thrower of bombs whose weight is in `palette` and hits `region`.
function quake.thrower(palette: number, region: Region): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local o = battle.spawn(quake.kind, pos)
            if o then
                o.element = 0
                o.alliance, o.flip = user.alliance, user.flip
                o:set_damage_word(damage)
                o.related1 = user
                local s = o.state :: State
                s.palette, s.region = palette, region
            end
            return o
        end,
    }
end

return quake
