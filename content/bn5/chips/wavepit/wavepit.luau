--!strict
-- BN5's WavePit, RedWave and MudWave (action 0x1A's instant chip effect 0x13, 0x080DAF18): BN5's own. In each row,
-- from the user's back column toward the other side, the first panel of the chip's kind (WavePit a sea panel, flag
-- 0x20000; RedWave a lava panel, 0x1000; MudWave a hole, a panel without the standable flag 0x10) starts a wave
-- there, and turns Normal (MudWave's hole stays). A row without one starts none.
--
-- The wave (attack object #0xA5, 0x080DADBC) goes a panel at a time: each segment stands on its panel (a solid one;
-- MudWave's first, in the hole, needn't) with its collision on, hitting navis (hit modifier 3), and 20 ticks later,
-- unless it has hit something, starts the next segment ahead; then it goes when its animation ends. The chips'
-- palettes are their first parameters.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local wavepit = {}

-- What starts a wave (0x080DAFA0's rows, by the chip's first parameter): the panel flags it needs and those it
-- mustn't have, whether the panel turns Normal, and the wave's palette.
export type Pit = { require: number, forbid: number, mends: boolean, palette: number }
wavepit.pits = {
    wavepit = { require = 0x0002_0000, forbid = 0, mends = true, palette = 0 } :: Pit,
    redwave = { require = 0x0000_1000, forbid = 0, mends = true, palette = 1 } :: Pit,
    mudwave = { require = 0, forbid = 0x10, mends = false, palette = 2 } :: Pit,
}

export type State = {
    -- Its palette (its first parameter), and whether it may stand off solid ground (its second: MudWave's first
    -- segment).
    palette: number,
    anywhere: boolean,
}

-- Its look: sheet 0c-3a, animation 0, no shadow (0x080DAE18's 0x00000C3A); its collision (0x04050300): an attack
-- hitting navis, hit modifier 3.
local SPRITE = asset.sprite("pit-wave")
local SOUND = asset.sound("pit-wave")
local HIT_MOD = 3
local SOLID = 0x10
local WAIT = 20

-- Its actions.
local WAIT_ACTION, SPREAD, FADE = 0, 4, 8

local function solid(me: Object): boolean
    return field.check(me.panel_x, me.panel_y, SOLID, 0)
end

-- 0x080DAE98: off solid ground (but where it may stand anywhere), gone; else 20 ticks.
local function wait(me: Object)
    if not (me.state :: State).anywhere and not solid(me) then
        me:set_lifecycle("destroy")
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t == 0 then
        me.action = SPREAD
    end
end

-- 0x080DAEC2: unless its hit took it off its panel, the next segment ahead (standing on solid ground only); then it
-- fades.
local function spread(me: Object)
    if me.collision.region then
        local damage = int.u32(me.damage + me.stamina * 0x10000)
        wavepit.spawn(me, me.panel_x + me.facing, me.panel_y, me.element, damage, (me.state :: State).palette, false)
    end
    me.action = FADE
end

-- 0x080DAEEC: at its animation's end, gone.
local function fade(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x81) then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- 0x080DAE50: the hits against it resolve and spark (a hit takes it off its panel); its action; its panel lit while
-- it covers it; then present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local a = me.action
    if a == WAIT_ACTION then
        wait(me)
    elseif a == SPREAD then
        spread(me)
    elseif a == FADE then
        fade(me)
    else
        error(string.format("the wave's action %#x reads past 0x080DAE8C", a))
    end
    if c.region then
        field.highlight(me.panel_x, me.panel_y)
    end
    c:present()
end

-- 0x080DADE0: gone with the battle, off the field or off solid ground (but where it may stand anywhere); else on its
-- panel, on the ground, its look and collision, its palette, the spark of its element; the sound.
local function init(me: Object)
    local s = me.state :: State
    if battle.over() or not field.valid(me.panel_x, me.panel_y) or (not s.anywhere and not solid(me)) then
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        error("the wave has no collision: 0x080DADE0 goes on through a null pointer")
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = s.palette
    me.timer = WAIT
    me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    battle.play_sound(SOUND)
    me.lifecycle = "update"
    tick(me)
end

wavepit.kind = define.kind {
    id = "wavepit/wave",
    pool = "attack",
    state = { palette = "u8", anywhere = "bool" },
    -- 0x080DADBC.
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

-- 0x080DAF02 and `sub_801155A`: a segment of `owner`'s on (x, y) with the element and the damage word, in `palette`,
-- standing off solid ground too when `anywhere`. (Its position is the spawner's registers until its init places
-- it.)
function wavepit.spawn(owner: Object, x: number, y: number, element: number, damage: number, palette: number, anywhere: boolean): Object?
    local o = battle.spawn(wavepit.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.palette, s.anywhere = palette, anywhere
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- 0x080DAF18: a chip's `instant` hook, for its pit.
function wavepit.instant(pit: Pit): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local facing = user.facing
        local back = if facing >= 0 then 1 else 6
        for y = 1, 3 do
            local x = back
            while field.valid(x, y) do
                if field.check(x, y, pit.require, pit.forbid) then
                    wavepit.spawn(user, x, y, spec.element, spec.damage, pit.palette, not pit.mends)
                    if pit.mends then
                        field.set_type(x, y, "normal")
                    end
                    break
                end
                x += facing
            end
        end
    end
end

return wavepit
