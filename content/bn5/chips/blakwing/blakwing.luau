--!strict
-- BN5's BlakWing (the dimming handler's subtype 32): BN5's own, on BN6's leftover code. Its controller (effect object
-- #0x54, 0x080E6920: the usual dimming phases; its effect, 0x080E6964) sends out the flock (attack object #0x7E,
-- 0x080D6230), waits for it, then 60 ticks, and is done.
--
-- The flock marks each column of the enemy's area from its front (0x080D633E: those with panels of the other side,
-- 0x080D63AC) with a perch (effect object #0x55, 0x080E69F0) on the row above the field, and with none is gone at
-- once. Every 12 ticks, 18 times, it sends a wing (attack object #0x7F, 0x080D63EC) down a perch's column other than
-- the last one's, at random; its flap sounds every 16 ticks. Done, it tells the perches to go and lets the controller
-- go. A wing falls 1.6 pixels a tick (a panel each 15), hitting navis (row 0x25, hit modifier 1) until a hit or the
-- field's bottom, where it goes in a puff.

local collision = require("../../rules/collision")
local dimming_chips = require("@common/dimming")
local panels = require("@common/panels")
local sparks = require("../../lib/sparks")

local blakwing = {}

local SPRITE = asset.sprite("bn5:sprite-0c-42")
local PUFF = define.effect { sprite = asset.sprite("bn5:puff"), anim = 2, palette = 0 }
local SOUND_START, SOUND_FLAP = asset.sound("bn5:place"), asset.sound("bn5:sound-130")

-- ---- The perch ------------------------------------------------------------------------------------------------------

export type PerchState = {
    -- Told to go (Param2).
    going: boolean,
}

-- 0x080E6A48: shows (animation 0), then perched (1) until told to go, then leaves (2) and is gone.
local function perch_update(me: Object)
    local s = me.state :: PerchState
    local p = me.phase
    if p == 0 then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.anim = 1
            me.phase = 4
        end
    elseif p == 4 then
        if s.going then
            me.anim = 2
            me.phase = 8
        end
    elseif p == 8 then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.visible = false
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("BlakWing's perch: phase %#x reads past 0x080E6A60", p))
    end
end

blakwing.perch = define.kind {
    id = "bn5:blakwing/perch",
    pool = "effect",
    state = { going = "bool" },
    -- 0x080E69F0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E6A10: shown, 0c-42's animation 0 stepped once, on its panel.
            me.visible = true
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite:step()
            me:set_coordinates_from_panel()
            me:set_lifecycle("update")
            perch_update(me)
        elseif l == "update" then
            perch_update(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- ---- The wing -------------------------------------------------------------------------------------------------------

local FALL, PANEL_TICKS = 0x1_9999, 15
local HIT_MOD = 1

-- 0x080D64C4: down; a panel every 15 ticks (snapped to its center), off the field's bottom gone in a puff.
local function fall(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x, int.i32(p.y + FALL), p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t ~= 0 then
        return
    end
    me:set_coordinates_from_panel()
    me.timer = PANEL_TICKS
    if field.valid(me.panel_x, me.panel_y) then
        return
    end
    battle.effect(me.pos, PUFF)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- 0x080D6480: the hits against it resolve and spark; a hit ends it; else it falls.
local function wing_tick(me: Object)
    -- (`sprite_clearFinalPalette`: drawn only.)
    me.sprite.white = false
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        me.visible = false
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if me.action ~= 0 then
        error(string.format("BlakWing's wing: action %#x reads past 0x080D64C0", me.action))
    end
    fall(me)
    me:update_sprite_while_dimmed()
    c:present()
end

blakwing.wing = define.kind {
    id = "bn5:blakwing/wing",
    pool = "attack",
    -- 0x080D63EC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080D640C: shown on its panel, 0c-42's animation 3, falling; its collision (row 0x25, hits navis,
            -- hit modifier 1, the plain spark); a panel's ticks; the first update at once.
            me:set_coordinates_from_panel()
            me.visible = true
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 3, 3
            sprite:set_animation(3)
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_coordinates_from_panel()
            if not me:create_collision() then
                me:free()
                return
            end
            me:setup_collision(collision.attack_480, collision.hits_navis, HIT_MOD)
            me.collision:set_hit_effect(sparks.plain)
            me.collision:present()
            me.timer = PANEL_TICKS
            me:set_lifecycle("update")
            wing_tick(me)
        elseif l == "update" then
            wing_tick(me)
        else
            me:destroy()
        end
    end,
}

-- ---- The flock ------------------------------------------------------------------------------------------------------

-- The enemy's area (0x080D63AC: the panels a side's flock looks for, require and forbid) and the column it starts
-- from (0x080D63BC, by side and flip).
local AREA = { [0] = { 0x20, 0 }, [1] = { 0, 0x20 } }
local START = { [0] = 4, [1] = 3 }
local MAX_PERCHES = 3

local function get_perch(s: FlockState, i: number): Object?
    return if i == 1 then s.perch1 elseif i == 2 then s.perch2 else s.perch3
end

local function set_perch(s: FlockState, i: number, o: Object)
    if i == 1 then
        s.perch1 = o
    elseif i == 2 then
        s.perch2 = o
    else
        s.perch3 = o
    end
end

export type FlockState = {
    -- Each round's ticks (+0x60), the perches (+0x64, +0x68, +0x6C) and how many (+0x70), the last one's index
    -- (+0x74), the flap's countdown (+0x78).
    every: number,
    perch1: Object?,
    perch2: Object?,
    perch3: Object?,
    count: number,
    last: number,
    flap: number,
    -- The controller it lets go (its +0x4C: the address of the controller's flag).
    controller: Object?,
}

-- 0x080D633E: a perch over each column of the enemy's area, from its front.
local function perch_columns(me: Object, s: FlockState): number
    local area = AREA[me.alliance]
    local x = START[bit32.bxor(me.alliance, me.flip)]
    local n = 0
    while field.valid(x, 1) do
        local any = false
        for y = 1, 3 do
            if field.check(x, y, area[1], area[2]) then
                any = true
            end
        end
        if any then
            -- 0x080E6AAE: its position is the spawner's registers.
            local o = battle.spawn(blakwing.perch, Vec3.new(0, 0, 0))
            if o then
                o.panel_x, o.panel_y = x, 0
                o.alliance, o.flip = me.alliance, me.flip
                if n >= MAX_PERCHES then
                    error("BlakWing's flock: a fourth perch runs past its list (+0x64)")
                end
                set_perch(s, n + 1, o)
                n += 1
            end
        end
        x += me.facing
    end
    return n
end

-- 0x080D62F2: a wing down a perch's column other than the last one's.
local function strike(me: Object, s: FlockState)
    local r = battle.rng_positive()
    local step = s.count - 1
    if step ~= 0 then
        step = r % step + 1
    end
    local index = (s.last + step) % s.count
    s.last = index
    local k = index
    for i = 1, MAX_PERCHES do
        local perch = get_perch(s, i)
        if perch then
            k -= 1
            if k < 0 then
                -- 0x080D650E: its position is the spawner's registers until its init places it.
                local o = battle.spawn(blakwing.wing, Vec3.new(0, me.element, 0))
                if o then
                    o.panel_x, o.panel_y = perch.panel_x, 0
                    o.element = me.element
                    o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
                    o.alliance = me.alliance
                    o.run_while_dimmed = true
                end
                return
            end
        end
    end
end

-- 0x080D629A: the flap every 16 ticks; a wing every round, 18; then done.
local function flock_tick(me: Object)
    local s = me.state :: FlockState
    s.flap -= 1
    if s.flap == 0 then
        s.flap = 16
        battle.play_sound(SOUND_FLAP)
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t ~= 0 then
        return
    end
    me.timer = s.every
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if int.i16(me.timer2) < 0 then
        me:set_lifecycle("destroy")
        return
    end
    strike(me, s)
end

-- 0x080D62D0: the controller let go, the perches told to go, gone.
local function flock_end(me: Object)
    local s = me.state :: FlockState
    local c = s.controller
    if c then
        (c.state :: ControllerState).busy = false
    end
    for i = MAX_PERCHES, 1, -1 do
        local perch = get_perch(s, i)
        if perch then
            (perch.state :: PerchState).going = true
        end
    end
    me:destroy()
end

blakwing.flock = define.kind {
    id = "bn5:blakwing/flock",
    pool = "attack",
    state = {
        every = "u16",
        perch1 = "object",
        perch2 = "object",
        perch3 = "object",
        count = "u32",
        last = "u32",
        flap = "u32",
        controller = "object",
    },
    -- 0x080D6230.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080D6250: no perch, gone (the controller let go); else the first index at random, 30 ticks to the
            -- first wing, the start's sound.
            local s = me.state :: FlockState
            me.vel = Vec3.new(-me.facing, me.vel.y, me.vel.z)
            local n = perch_columns(me, s)
            if n == 0 then
                local c = s.controller
                if c then
                    (c.state :: ControllerState).busy = false
                end
                me:free()
                return
            end
            s.count = n
            s.last = battle.rng_positive() % n
            me.timer = 30
            s.flap = 1
            battle.play_sound(SOUND_START)
            me:set_lifecycle("update")
            flock_tick(me)
        elseif l == "update" then
            flock_tick(me)
        else
            flock_end(me)
        end
    end,
}

-- ---- The controller -------------------------------------------------------------------------------------------------

export type ControllerState = {
    -- The Atk+ bonus (+0x32): it adds to the flock's damage word.
    bonus: number,
    -- The flock is out (+0x0C, which it clears).
    busy: boolean,
}

-- 0x080E6964: phase 0 (0x080E6980) the flock out (0x080D63BE: 12 ticks a round, 18 rounds), until it lets go; phase
-- 4 (0x080E69AA) 60 ticks; done.
local function effect(me: Object)
    local s = me.state :: ControllerState
    local p = me.phase
    if p == 0 then
        if me.phase_init == 0 then
            local o = battle.spawn(blakwing.flock, Vec3.new(18, 0, 0))
            if o then
                local fs = o.state :: FlockState
                fs.every = 12
                o.timer, o.timer2 = 12, 18
                o:set_damage_word(dimming_chips.damage_plus(me, s.bonus))
                o.alliance, o.flip = me.alliance, me.flip
                fs.controller = me
                o.run_while_dimmed = true
                s.busy = true
            end
            me.phase_init = 4
        end
        if not s.busy then
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        if me.phase_init == 0 then
            me.timer = 60
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t < 0 then
            dimming_chips.done(me)
        end
    else
        error(string.format("BlakWing's controller: phase %#x reads past 0x080E6978", p))
    end
end

blakwing.controller = define.kind {
    id = "bn5:blakwing/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- 0x080E6920.
    update = dimming_chips.phases { name = "BlakWing", effect = effect },
}

-- 0x080E69CC: the chip's `dimming` hook.
function blakwing.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(blakwing.controller, user, spec)
    if o then
        (o.state :: ControllerState).bonus = spec.bonus
    end
    return o
end

return blakwing
