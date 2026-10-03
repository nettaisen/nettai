--!strict
-- BN5's CusVolt1-3 (action 0x27, 0x080ED1A2): BN5's own. The navi holds out its volt gun (the attachment table's row
-- 0x36, 0c-39) and, unless the custom gauge is full, the battle is of mode 6 or a link battle, drains half the custom
-- gauge; on the action's sixth tick an electric beam (attack object #0xB8, 0x080DD3D0) comes out 40 pixels ahead of
-- it, over its panel and the column ahead, and follows it until the action's end, 31 ticks in. Its damage is the
-- custom gauge's (BN5's formulas 73 to 75).

local attachment = require("../attachment/attachment")
local collision = require("../../rules/collision")
local panels = require("@common/panels")
local regions = require("@common/regions")

local cusvolt = {}

-- ---- The beam -------------------------------------------------------------------------------------------------------

-- Its look (`sub_8011504`'s word 0x00000C40: sheet 0c-40, animation 0, no shadow) and collision (0x0A050300: a thrown
-- thing that hits navis, hit modifier 3), over its panel and the column ahead (region 0x15).
local SPRITE = asset.sprite("bn5:sprite-0c-40")
local HIT_MOD = 3
-- 40 pixels ahead of its navi, at the navi's attach point 26.
local AHEAD, ATTACH_POINT = 40, 26

export type BeamState = {
    -- The navi whose related1 it is (its +0x60: the address of the navi's related1).
    navi: Object?,
}

-- 0x080DD482: ahead of its navi, at the navi's attach point; its panel from where it is.
local function follow(me: Object)
    local navi = me.related1 :: Object
    local p = navi.pos
    -- (The game stores the position at +0x36, which the store's alignment makes +0x34, its own.)
    local x = int.i32(p.x + int.shl(AHEAD * me.facing, 16))
    local dx, dz = navi:attach_point(ATTACH_POINT)
    me.pos = Vec3.new(int.i32(x + dx * 0x10000), p.y, int.i32(p.z + dz * 0x10000))
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- 0x080DD41A: a hit takes its region off; its navi let go of it, gone; else it follows, its region back on a new
-- panel, the region's panels highlighted while it has one.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local navi = (me.state :: BeamState).navi
    if navi == nil or navi.related1 == nil then
        c:set_region(nil)
        me:set_lifecycle("destroy")
    else
        local was_x, was_y = me.panel_x, me.panel_y
        follow(me)
        if me.panel_x ~= was_x or me.panel_y ~= was_y then
            c:set_region(regions.fan)
        end
        if c.region ~= nil then
            panels.highlight_region(me.panel_x, me.panel_y, regions.fan, me.facing)
        end
    end
    c:present()
end

-- 0x080DD3F4: its look and collision over the fan; shown; ahead of its navi; the first update at once.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.collision:set_region(regions.fan)
    me.visible = true
    follow(me)
    me.lifecycle = "update"
    tick(me)
end

cusvolt.beam = define.kind {
    id = "bn5:cusvolt/beam",
    pool = "attack",
    state = { navi = "object" },
    -- 0x080DD3D0: by its lifecycle, then a sprite step.
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
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- ---- The action -----------------------------------------------------------------------------------------------------

export type State = {
    -- Ticks into the shot, then the recovery's (AIAttackVars+0x10).
    timer: number,
}

local FIRE, LOWER = 0x0, 0x4
local ANIM = 10
local GUN = attachment.look { sprite = asset.sprite("bn5:sprite-0c-39"), palette = 0, attach_point = 0x19 }
local SOUND = asset.sound("bn5:elec-man")
-- The beam on this tick; the action ends after this one.
local BEAM_TICK, LAST_TICK = 5, 30
-- What the gun drains of the custom gauge (half).
local DRAIN = 0x2000

-- 0x080ED1BC: the gun out, half the gauge drained (but in a link battle, a mode 6 one or with the gauge full); the
-- beam on the sixth tick; on after the 31st.
local function fire(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        attachment.attach(me, GUN, "overlay")
        me:set_status("using_action", true)
        if not battle.gauge_full() and battle.mode() ~= 6 and not battle.link() then
            battle.drain_custom_gauge(DRAIN)
        end
        me.step_init = 4
        battle.play_sound(SOUND)
        s.timer = 0
    end
    if s.timer == BEAM_TICK then
        -- 0x080DD468: the beam (its panel and position the dispatcher's registers until its init places it: the
        -- timer, the handler's address, the element and the bonus), the navi's related1 it.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        local o = battle.spawn(cusvolt.beam, Vec3.new(0x080ED1BD, me.attack_element, me.extra))
        if o then
            o.panel_x, o.panel_y = BEAM_TICK, 0xBD
            o.element = me.attack_element
            o:set_damage_word(damage)
            o.alliance, o.flip = me.alliance, me.flip
            o.related1 = me
            ;(o.state :: BeamState).navi = me
            me.related1 = o
        end
        local gun = me.overlay
        if gun then
            gun.anim += 1
        end
    end
    s.timer += 1
    if s.timer > LAST_TICK then
        me.step, me.step_init = LOWER, 0
    end
end

-- 0x080ED254: the gun's animation back, the beam let go; the next tick, back to idle.
local function lower(me: Object, s: State)
    if me.step_init == 0 then
        local gun = me.overlay
        if gun then
            gun.anim -= 1
        end
        s.timer = 0
        me.related1 = nil
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me.overlay = nil
        me:exit_attack()
    end
end

-- A CusVolt's action.
function cusvolt.action(id: string): Action
    return define.action {
        id = id,
        state = { timer = "u16" },
        -- 0x080ED1A2.
        update = function(me: Object, s: State)
            local step = me.step
            if step == FIRE then
                fire(me, s)
            elseif step == LOWER then
                lower(me, s)
            else
                error(string.format("CusVolt phase %#x reads past 0x080ED1B4", step))
            end
        end,
    }
end

return cusvolt
