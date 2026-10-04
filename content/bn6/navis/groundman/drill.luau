--!strict
-- GroundMan's flying drill (the original's attack object #0xC6,
-- `sub_80DDDF0`): his charged attack's (./charge). It waits on its panel
-- 20 pixels up, then flies ahead 8 pixels a tick for 10 ticks (two
-- panels), hitting what it meets; where that is an enemy's body or
-- object, or where its flight ends, it drills on for 30 ticks, hitting
-- its panel twice. It is gone off the field, when the battle is over, and
-- as soon as GroundMan leaves the action he threw it in. It stands still
-- while dimmed.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local drill = {}

export type State = {
    -- It is one of the side drills (the original's first parameter, which
    -- picks its animations), and the ticks it waits (the second).
    side: boolean,
    wait: number,
    -- The action its owner threw it in.
    action: Action?,
}

local SPRITE = asset.sprite("groundman")
local SOUND = asset.sound("drilarm")

-- Its actions.
local WAIT, FLY, DRILL = 0, 4, 8

-- Its animations waiting and flying (`byte_80DDE80`, `byte_80DDF84`): the
-- middle drill's, the side drills'.
local ANIM_WAIT, ANIM_FLY = { [false] = 0x15, [true] = 0x17 }, { [false] = 0x16, [true] = 0x18 }

-- Its height in pixels (a halfword store over Z's whole part), its speed,
-- 16.16, and how far it flies: two panels' width.
local HEIGHT, SPEED, REACH = 0x14, 0x8_0000, 2 * 0x28_0000

-- What stops it when hit (`byte_80DDEEC`), by side: an enemy's body or
-- object, or a neutral object.
local STOPS = { [0] = 0x0580_0000, 0x0A80_0000 }

local HIT_MOD = 1
-- The drilling: 30 ticks, a hit at 20 and at 10 left.
local DRILL_TICKS, FIRST_HIT, SECOND_HIT = 0x1E, 0x14, 0xA

-- `sub_80DDE14`: over its panel; a breaking attack at navis.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), bit32.lshift(HEIGHT, 16)))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    local anim = ANIM_WAIT[s.side]
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.breaking, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.breaking)
    me.collision:present()
    me:set_lifecycle("update")
end

-- Stop flying and drill (byte stores: the phase's entry flag stays).
local function start_drilling(me: Object)
    me.collision:set_region(nil)
    me.action = DRILL
    me.phase = 0
end

-- `sub_80DDEF4`: it waits its ticks.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = (me.state :: State).wait
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me.action = FLY
        me.phase = 0
    end
end

-- `sub_80DDF18`: it flies (from the tick it starts); gone off the field,
-- drilling where its flight ends.
local function fly(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = ANIM_FLY[(me.state :: State).side]
        battle.play_sound(SOUND)
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        me.timer = int.tdiv(REACH, SPEED)
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        start_drilling(me)
    end
end

-- `sub_80DDFDE`: a breaking hit on its panel.
local function hit(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.breaking,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
end

-- `sub_80DDF86`: it drills 30 ticks, hitting at 20 and at 10 left.
local function drill_on(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = DRILL_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t == FIRST_HIT or t == SECOND_HIT then
        hit(me)
    elseif t <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DDE82`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    if battle.over() or (me.related1 :: Object):navi_action() ~= s.action then
        c:set_region(nil)
        -- (A byte store: its action and phase stay.)
        me.lifecycle = "destroy"
        return
    end
    if bit32.btest(c.hit_flags, STOPS[me.alliance]) then
        start_drilling(me)
    end
    if not battle.dimmed() then
        local a = me.action
        if a == WAIT then
            wait(me)
        elseif a == FLY then
            fly(me)
        elseif a == DRILL then
            drill_on(me)
        else
            error(string.format("GroundMan's drill action %#x reads past its table", a))
        end
    end
    c:present()
end

drill.kind = define.kind {
    id = "groundman/drill",
    pool = "attack",
    state = { side = "bool", wait = "u8", action = "action" },
    -- `sub_80DDDF0`.
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

-- `sub_80DDFB6`: a drill for `owner` on (x, y) with `element` and the
-- damage word, following the action `owner` runs. `z` is the spawner's
-- register that becomes its Z, whose fraction it keeps.
function drill.spawn(owner: Object, x: number, y: number, element: number, side: boolean, wait_ticks: number, damage: number, z: number): Object?
    local o = battle.spawn(drill.kind, Vec3.new(y, element, z))
    if o then
        local s = o.state :: State
        s.side, s.wait = side, wait_ticks
        local action = owner:navi_action()
        if type(action) ~= "table" then
            error("GroundMan's drill follows an action that is no content action")
        end
        s.action = action :: Action
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return drill
