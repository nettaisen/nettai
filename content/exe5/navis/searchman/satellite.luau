--!strict
-- Satelity's satellite, its beam and the beam's flares (EXE5's effect objects #0x61 and #0x62, 0x080E767C and
-- 0x080E7888, and attack object #0xAF, 0x080DC010; EXE5's own). The satellite appears three panels behind its target,
-- 24 pixels further back: 8 ticks, 12 opening (animation 1), 10 aiming (animation 2), then it fires (animation 3) for
-- 60 ticks, its sound at the start and every ninth tick after. The beam (as long, from the same place) cracks the
-- target's panel, hits it for the beam's ticks, and sends a flare to each of the four panels beside it: a flare moves
-- a panel out in 10 ticks, then hits its panel for the rest of the beam's ticks, and a hit ends it.
--
-- Their first parameter says whether they run while the battle is dimmed (4 and up: Satelity's), their second the
-- beam's hit's ticks (60). In the operation battle the satellite marks its target as it aims and fires
-- (lib/navi-chips/marker).

local collision = require("../../rules/collision")
local marker = require("../../lib/navi-chips/marker")
local regions = require("@exelib/regions")

local satellite = {}

export type State = {
    -- Param1 is 4 or more: it runs while the battle is dimmed.
    while_dimmed: boolean,
    -- Param2: the ticks the beam's hit and its flares last.
    ticks: number,
}

export type FlareState = {
    while_dimmed: boolean,
    ticks: number,
    -- Param3: which way it goes (0 right, 1 left, 2 down, 3 up).
    way: number,
}

local SATELLITE = asset.sprite("satellite")
local BEAM = asset.sprite("satellite-beam")
local FLARE = asset.sprite("satellite-flare")
-- (0x111: 0x080E77C4 loads it as 18 + 255.)
local SOUND_FIRE = asset.sound("moon-beam")

-- The panels from the satellite to its target, and how far behind its panel it hangs (16.16).
local REACH = 3
local BACK = 0x18_0000
-- The satellite's actions (0x080E772C) and their ticks.
local APPEAR, OPEN, AIM, FIRE = 0x0, 0x4, 0x8, 0xC
local APPEAR_TICKS, OPEN_TICKS, AIM_TICKS, FIRE_TICKS = 8, 12, 10, 60
local SOUND_EVERY = 8
-- The operation battle's marks: a tick each, the aim's of style 1, the fire's of style 9.
local MARK_AIM, MARK_FIRE = 1, 9
-- The beam's own ticks (0x080E7906: not its parameter).
local BEAM_TICKS = 60
-- A flare's actions (0x080DC124), its animation and its step a tick (16.16) by its way (0x080DC0D0, 0x080DC0D8),
-- and the ticks it moves.
local MOVE, STAY = 0x0, 0x4
local FLARE_ANIMS = { [0] = 0, 0, 1, 2 }
local FLARE_STEPS = { [0] = { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 } }
local FLARE_SPEED_X, FLARE_SPEED_Y = 0x4_0000, 0x2_8000
local FLARE_MOVE_TICKS = 10

-- The beam's and the flares' kinds (below).
local beam_kind: Kind
local flare_kind: Kind

-- An object's sprite after its update: `object_updateSprite` for one that runs while dimmed, else
-- `object_updateSpritePaused`.
local function update_sprite(me: Object, while_dimmed: boolean)
    if while_dimmed then
        me:update_sprite()
    else
        me:update_sprite_while_paused()
    end
end

-- Count the timer down; true when it runs out.
local function count(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Behind its panel, on the floor: where the satellite and its beam are drawn from.
local function hang(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x - me.facing * BACK, p.y, 0)
end

-- 0x080DC194: a flare of `beam` from (x, y), going `way`.
local function spawn_flare(beam: Object, x: number, y: number, way: number, s: State): Object?
    local o = battle.spawn(flare_kind, Vec3.zero)
    if o then
        local f = o.state :: FlareState
        f.while_dimmed, f.ticks, f.way = s.while_dimmed, s.ticks, way
        o.panel_x, o.panel_y = x, y
        o.element = beam.element
        o.related1 = beam
        o:set_damage_word(bit32.bor(beam.damage, bit32.lshift(beam.stamina, 16)))
        o.alliance, o.flip = beam.alliance, beam.flip
        o.run_while_dimmed = true
    end
    return o
end

-- 0x080E7988: the beam of `sat`, from its panel.
local function spawn_beam(sat: Object, s: State): Object?
    local o = battle.spawn(beam_kind, Vec3.zero)
    if o then
        local b = o.state :: State
        b.while_dimmed, b.ticks = s.while_dimmed, s.ticks
        o.panel_x, o.panel_y = sat.panel_x, sat.panel_y
        o.element = sat.element
        o:set_damage_word(bit32.bor(sat.damage, bit32.lshift(sat.stamina, 16)))
        o.related1 = sat.related1
        o.alliance, o.flip = sat.alliance, sat.flip
        if s.while_dimmed then
            o.run_while_dimmed = true
        end
    end
    return o
end

-- 0x080E76AC: behind its panel, its shadow on the ground.
local function satellite_init(me: Object)
    hang(me)
    local sprite = me.sprite
    sprite:load(SATELLITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- 0x080E773C to 0x080E77B4: its four actions.
local function satellite_act(me: Object, s: State)
    local action = me.action
    local tx, ty = me.panel_x + REACH * me.facing, me.panel_y
    if action == APPEAR then
        if me.phase == 0 then
            me.phase = 4
            me.timer = APPEAR_TICKS
        elseif count(me) then
            me:set_action(OPEN)
        end
    elseif action == OPEN then
        if me.phase == 0 then
            me.phase = 4
            me.anim = 1
            me.timer = OPEN_TICKS
        elseif count(me) then
            me:set_action(AIM)
        end
    elseif action == AIM then
        if me.phase == 0 then
            me.phase = 4
            me.anim = 2
            me.timer = AIM_TICKS
            return
        end
        -- 0x080E7842.
        marker.spawn(me, tx, ty, 1, MARK_AIM, false)
        if count(me) then
            me:set_action(FIRE)
        end
    elseif action == FIRE then
        if me.phase == 0 then
            me.phase = 4
            me.anim = 3
            battle.play_sound(SOUND_FIRE)
            spawn_beam(me, s)
            me.timer = FIRE_TICKS
            me.timer2 = 0
            return
        end
        -- 0x080E7862.
        marker.spawn(me, tx, ty, 1, MARK_FIRE, true)
        local n = me.timer2 + 1
        me.timer2 = n
        if n > SOUND_EVERY then
            battle.play_sound(SOUND_FIRE)
            me.timer2 = 0
        end
        if count(me) then
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("Satelity's satellite's action %#x reads past its table (0x080E772C)", action))
    end
end

-- 0x080E76FE: gone with the battle; still while the battle is dimmed unless it runs then.
local function satellite_tick(me: Object, s: State)
    if battle.over() then
        me:free()
        return
    end
    if not s.while_dimmed and battle.dimmed() then
        return
    end
    satellite_act(me, s)
end

satellite.kind = define.kind {
    id = "searchman/satelity/satellite",
    pool = "effect",
    state = { while_dimmed = "bool", ticks = "u8" },
    -- 0x080E767C.
    update = function(me: Object)
        local s = me.state :: State
        local l = me.lifecycle
        if l == "init" then
            satellite_init(me)
        elseif l == "update" then
            satellite_tick(me, s)
            if not me.active then
                return
            end
        else
            me:free()
            return
        end
        update_sprite(me, s.while_dimmed)
    end,
}

-- 0x080E78B8: from where its satellite hangs; its target's panel cracked and hit, and the four flares.
local function beam_init(me: Object, s: State)
    hang(me)
    local sprite = me.sprite
    sprite:load(BEAM)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = BEAM_TICKS
    local tx, ty = me.panel_x + REACH * me.facing, me.panel_y
    field.crack(tx, ty)
    -- 0x080C4DC6's word 0x1905FF01: its panel, no spark, a curse's hit on navis; hit modifier 3; it resolves while
    -- dimmed.
    local box = battle.hitbox(me, {
        panel_x = tx,
        panel_y = ty,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.curse,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
        box.timer = s.ticks
    end
    for way = 0, 3 do
        spawn_flare(me, tx, ty, way, s)
    end
    me:set_lifecycle("update")
end

-- 0x080E7964: its ticks; gone with the battle.
local function beam_tick(me: Object, s: State)
    if battle.over() then
        me:free()
        return
    end
    if not s.while_dimmed and battle.dimmed() then
        return
    end
    if count(me) then
        me:free()
    end
end

beam_kind = define.kind {
    id = "searchman/satelity/beam",
    pool = "effect",
    state = { while_dimmed = "bool", ticks = "u8" },
    -- 0x080E7888.
    update = function(me: Object)
        local s = me.state :: State
        local l = me.lifecycle
        if l == "init" then
            beam_init(me, s)
        elseif l == "update" then
            beam_tick(me, s)
            if not me.active then
                return
            end
        else
            me:free()
            return
        end
        update_sprite(me, s.while_dimmed)
    end,
}

-- 0x080DC040: on the target's panel, facing its way (the one going left flipped), an attack that hits navis with no
-- region yet.
local function flare_init(me: Object, f: FlareState)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(FLARE)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = FLARE_ANIMS[f.way]
    local step = FLARE_STEPS[f.way]
    if anim == nil or step == nil then
        error(string.format("Satelity's flare's way %d reads past its tables (0x080DC0D0)", f.way))
    end
    me.anim, me.anim_loaded = anim, anim
    sprite:set_animation(anim)
    sprite.palette = 0
    if f.way == 1 then
        sprite:set_flip(1)
    end
    me.vel = Vec3.new(step[1] * FLARE_SPEED_X, step[2] * FLARE_SPEED_Y, 0)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, 3)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
end

-- 0x080DC12C and 0x080DC16E: a panel out in 10 ticks, then its hit there for the rest of the beam's ticks.
local function flare_act(me: Object, f: FlareState)
    local action = me.action
    if action == MOVE then
        if me.phase == 0 then
            me.phase = 4
            me.timer = FLARE_MOVE_TICKS
            return
        end
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
        if count(me) then
            me.collision:set_region(regions.single)
            me:set_action(STAY)
        end
        me:set_panel_from_coordinates()
        me:update_collision_panels()
    elseif action == STAY then
        if me.phase == 0 then
            me.phase = 4
            me.timer = int.u16(f.ticks - FLARE_MOVE_TICKS)
            return
        end
        if count(me) then
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("Satelity's flare's action %#x reads past its table (0x080DC124)", action))
    end
end

-- 0x080DC0E0: its hits resolved; the battle's end or a hit ends it; still while the battle is dimmed unless it runs
-- then.
local function flare_tick(me: Object, f: FlareState)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or c.hit_flags ~= 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if f.while_dimmed or not battle.dimmed() then
        flare_act(me, f)
    end
    c:present()
end

flare_kind = define.kind {
    id = "searchman/satelity/flare",
    pool = "attack",
    state = { while_dimmed = "bool", ticks = "u8", way = "u8" },
    -- 0x080DC010.
    update = function(me: Object)
        local f = me.state :: FlareState
        local l = me.lifecycle
        if l == "init" then
            flare_init(me, f)
            if not me.active then
                return
            end
        elseif l == "update" then
            flare_tick(me, f)
        else
            -- 0x080138F2.
            me:destroy()
            return
        end
        update_sprite(me, f.while_dimmed)
    end,
}

-- 0x080E7810: a satellite for `owner` (a dimming controller, or whoever calls it down) on panel (x, y), with its
-- element and damage word; `while_dimmed` and `ticks` are its parameters.
function satellite.spawn(owner: Object, x: number, y: number, element: number, damage: number, while_dimmed: boolean, ticks: number): Object?
    local o = battle.spawn(satellite.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.while_dimmed, s.ticks = while_dimmed, ticks
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        if while_dimmed then
            o.run_while_dimmed = true
        end
    end
    return o
end

return satellite
