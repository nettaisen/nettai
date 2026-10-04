--!strict
-- NrthWind (BN5 chip 0x08a): a north wind blows over the field for 100 ticks, then strips every barrier off it: the
-- dimming handler's subtype 21 (0x080E4854), whose controller (effect object #0x39, 0x080E4804), wind (attack object
-- #0x5A, 0x080D122C) and streaks (effect object #0x3A) are BN6's code that no BN6 chip uses (`sub_80E47D4`,
-- `sub_80CFC08`, `sub_80E4848`): a streak every 8 ticks somewhere along the user's back edge, and at the end the
-- wind's whole-field region (row 0x1B, which wind hits remove barriers by).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")

-- A streak (effect object #0x3A, BN6's `sub_80E4848`): sheet 0c-00's animation 0 (in palette 0), blown forward
-- 16 pixels a tick, thrown up and falling, for 30 ticks.
local STREAK_SPRITE = asset.sprite("bn5:sword")
local STREAK_SPEED, STREAK_RISE, STREAK_GRAVITY, STREAK_TICKS = 0x10_0000, int.i32(0xFFF98000), 0x3C00, 0x1E

local function streak_tick(me: Object)
    me:update_sprite()
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z + v.z))
    me.vel = Vec3.new(v.x, v.y, int.i32(v.z + STREAK_GRAVITY))
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me:set_lifecycle("destroy")
    end
end

local STREAK = define.kind {
    id = "bn5:nrthwind/streak",
    pool = "effect",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- BN6's `sub_80E4868`.
            me.visible = true
            local sprite = me.sprite
            sprite:load(STREAK_SPRITE)
            sprite.shadow = "hidden"
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite.palette = 0
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.vel = Vec3.new(me.facing * STREAK_SPEED, me.vel.y, STREAK_RISE)
            me.timer = STREAK_TICKS
            me.lifecycle = "update"
            streak_tick(me)
        elseif l == "update" then
            streak_tick(me)
        else
            me:free()
        end
    end,
}

-- BN6's `sub_80CFCA8` and `sub_80E48F8`: a streak at random along the back edge of the wind's side's view (one
-- GetRNG2 draw: x from its low 5 bits, -0x98 pixels and on, the other edge facing left; y from its top 7, -0x40 and
-- on), 40 pixels up.
local function streak(wind: Object)
    local r = int.u32(battle.rng())
    local x = bit32.band(r, 0x1F) - 0x98
    if bit32.bxor(wind.alliance, wind.flip) ~= 0 then
        x += 0x110
    end
    local y = bit32.rshift(r, 25) - 0x40
    local o = battle.spawn(STREAK, Vec3.new(int.i32(x * 0x10000), int.i32(y * 0x10000), 0x28_0000))
    if o then
        o.alliance, o.flip = wind.alliance, wind.flip
    end
end

-- The wind (attack object #0x5A, BN6's `sub_80CFC08`): 100 ticks of streaks, one every 8; then its region over the
-- whole field for a tick, its holder told it is done.
type WindState = { holder: Object? }
local WIND_TICKS, STREAK_EVERY = 0x64, 8
local WIND_SOUND = asset.sound("bn5:dustbrk")

local function wind_tick(me: Object)
    local c = me.collision
    c:remove()
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 == 0 then
        streak(me)
        me.timer2 = STREAK_EVERY
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        c:set_region(regions.field)
        local holder = (me.state :: WindState).holder
        if holder then
            (holder.state :: { busy: boolean }).busy = false
        end
        me:set_lifecycle("destroy")
    end
    c:present()
end

local WIND = define.kind {
    id = "bn5:nrthwind/wind",
    pool = "attack",
    state = { holder = "object" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- BN6's `sub_80CFC28`.
            me.timer = WIND_TICKS
            me.timer2 = STREAK_EVERY
            if not me:create_collision() then
                me:free()
                return
            end
            me:setup_collision(collision.collision_1b, collision.nothing, 0)
            me.collision:set_region(nil)
            battle.play_sound(WIND_SOUND)
            me.lifecycle = "update"
            wind_tick(me)
        elseif l == "update" then
            wind_tick(me)
        else
            -- BN6's `sub_80CFC96`.
            me.collision:remove()
            me:free()
        end
    end,
}

-- The controller (effect object #0x39, BN6's `sub_80E47D4`): the wind (BN6's `sub_80CFCD6`: of the controller's side
-- and facing, running while dimmed); once it is done, undim.
type ControllerState = { busy: boolean }

local function effect(me: Object)
    if me.phase_init == 0 then
        local o = battle.spawn(WIND)
        if o then
            (o.state :: WindState).holder = me
            o.alliance, o.flip = me.alliance, me.flip
            o.run_while_dimmed = true
        end
        (me.state :: ControllerState).busy = true
        me.timer = 0x3C
        me.phase_init = 4
    end
    if not (me.state :: ControllerState).busy then
        dimming_chips.done(me)
    end
end

local CONTROLLER = define.kind {
    id = "bn5:nrthwind/controller",
    pool = "effect",
    state = { busy = "bool" },
    update = dimming_chips.phases { name = "NrthWind", effect = effect },
}

return define.chip {
    id = "bn5:nrthwind",
    codes = { "D", "Q", "V", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 33,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(CONTROLLER, user, spec)
    end,
}
