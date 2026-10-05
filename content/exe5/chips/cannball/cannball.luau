--!strict
-- EXE5's cannonball (attack object #0x35, 0x080CB5C8; EXE5's own, thrown by action 0x12's subtype 4, 0x080CB7D4):
-- lobbed in a 50-tick arc at the panel three ahead, it can hit on its way (its region set once it lands); landing
-- on a solid panel it hits it, breaks it (cracks it while something stands on it) with an impact and a crack's
-- sound; off solid ground it ends in a puff, and a hit ends it at once. Its kind (Param1) 2 is another's: a plain
-- attack that bursts over the column with explosions and the bomb's sound (no chip throws it).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local trajectory = require("../../lib/trajectory")

local cannball = {}

export type State = {
    -- Its kind (Param1).
    which: number,
    -- The panel it aims at (Param3 and Param4).
    target_x: number,
    target_y: number,
}

-- 0x080CB5EC: the bomb's sprite in animation 1, palette 4 (kind 2: 0).
local SPRITE = asset.sprite("bomb")
local FLIGHT_TICKS, GRAVITY = 50, -0x3000

-- What the navi holds throwing one (the attachment table's row 0x1A): the bomb in palette 4.
cannball.held = attachment.look { sprite = SPRITE, palette = 4 }
local HIT_MOD = 3
-- Its region once landed, by kind (0x080CB7A0).
local REGIONS = { [0] = regions.single, [1] = regions.single, [2] = regions.wide, [3] = regions.region_20 }
-- Effects 0x14 (a puff), 0x11 (an impact) and 0 (an explosion).
local PUFF = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local IMPACT = define.effect { sprite = asset.sprite("hit-sparks"), anim = 1, palette = 0 }
local EXPLOSION = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local SOUND_CRACK, SOUND_BOMB = asset.sound("panel-crack"), asset.sound("hit-bomb-1")

-- 0x080CB6BC: fly to the target panel; land there.
local function fly(me: Object, s: State)
    if me.phase_init == 0 then
        local tx, ty = field.center(s.target_x, s.target_y)
        me.vel = trajectory.in_ticks(me.pos, tx, ty, 0, FLIGHT_TICKS, GRAVITY)
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t >= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y + v.y, int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local x, y = me.panel_x, me.panel_y
    if not field.solid(x, y) then
        battle.effect(me.pos, PUFF)
        me:set_lifecycle("destroy")
        return
    end
    local region = REGIONS[s.which]
    if region == nil then
        error(string.format("the cannonball's kind %d reads past its regions", s.which))
    end
    me.collision:set_region(region)
    if s.which == 2 then
        battle.region_effects(x, y, regions.wide, me.alliance, EXPLOSION, 0)
        battle.play_sound(SOUND_BOMB)
    else
        field.break_panel(x, y)
        battle.effect(me.pos, IMPACT)
        battle.play_sound(SOUND_CRACK)
    end
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- 0x080CB65A: resolve last tick's hits; the battle over, a puff; a hit, gone; else fly, then go.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, PUFF)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if me.collision.hit_flags ~= 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == 0 then
        fly(me, me.state :: State)
    elseif a == 4 then
        -- 0x080CB7A4.
        me.collision:set_region(nil)
        me.visible = false
        me:set_lifecycle("destroy")
    else
        error(string.format("the cannonball's action %#x reads past its table", a))
    end
    me.collision:present()
end

cannball.kind = define.kind {
    id = "cannball/ball",
    pool = "attack",
    state = { which = "u8", target_x = "u8", target_y = "u8" },
    -- 0x080CB5C8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080CB5EC.
            local s = me.state :: State
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "ground"
            me.visible = true
            me.anim, me.anim_loaded = 1, 1
            sprite:set_animation(1)
            if s.which ~= 2 then
                sprite.palette = 4
            end
            me:set_panel_from_coordinates()
            if not me:create_collision() then
                me:free()
                return
            end
            me:setup_collision(if s.which == 2 then collision.attack else collision.thrown_break, collision.hits_navis, HIT_MOD)
            me.collision:set_hit_effect(nil)
            me.collision:set_region(nil)
            me.collision:present()
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080CB7D4: a thrower of kind `which` cannonballs at the panel three ahead of the user.
function cannball.thrower(which: number): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local o = battle.spawn(cannball.kind, pos)
            if o then
                o.alliance = user.alliance
                o.element = 0
                o:set_damage_word(damage)
                local s = o.state :: State
                s.which = which
                s.target_x, s.target_y = user.panel_x + 3 * user.facing, user.panel_y
            end
            return o
        end,
    }
end

return cannball
