--!strict
-- AntiDmg's shuriken (the original's attack object #0xC2, `sub_80DD764`):
-- what its counter throws. It flies from the user, 160 pixels up, onto
-- the target's panel in 15 ticks, highlighting it, hits it for two ticks,
-- lies there 30 ticks and blinks out over 20.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")

local shuriken = {}

export type State = {
    -- Where it flies from (the thrower's X and Y, its own Z) and to (the
    -- panel's center): ExtraVars+0 to +0x10.
    from_x: number,
    from_y: number,
    from_z: number,
    to_x: number,
    to_y: number,
    -- Its hit modifier (the original's first parameter).
    hit_mod: number,
}

local SPRITE = asset.sprite("bn6:shuriken")
local FLIGHT = 0xF
local LIE, BLINK = 0x1E, 0x14
local SOUND_LAND = asset.sound("bn6:panel-crack")
-- What it is (row 0x1C: a thrown slash) and hits.
local IS = collision.thrown_slash
local HITS = collision.hits_navis
local LANDED_ANIM = 1

-- `sub_801164C`: `from` plus (`to` - `from`) * t / n, the BIOS's division.
local function between(from: number, to: number, t: number, n: number): number
    return int.i32(from + int.tdiv(int.mul32(int.i32(to - from), t), n))
end

-- `sub_80DD848`: on its way, from the thrower, with a puff.
local function start(me: Object)
    local s = me.state :: State
    me.visible = true
    me.timer = 0
    me.pos = Vec3.new(between(s.from_x, s.to_x, 0, FLIGHT), me.pos.y, between(s.from_z, 0, 0, FLIGHT))
    battle.effect(me.pos, effects.bubbles)
    me.action = 4
end

-- `sub_80DD87C`: fly 15 ticks, highlighting the panel; then land and hit.
local function fly(me: Object)
    local s = me.state :: State
    field.highlight(me.panel_x, me.panel_y)
    local t = me.timer + 1
    me.timer = t
    if t > FLIGHT then
        me.anim = LANDED_ANIM
        me.collision:set_region(regions.single)
        battle.play_sound(SOUND_LAND)
        me.action = 8
        me.timer = LIE
        return
    end
    local x = between(s.from_x, s.to_x, t, FLIGHT)
    local z = between(s.from_z, 0, t, FLIGHT)
    local y = between(s.from_y, s.to_y, t, FLIGHT)
    me.pos = Vec3.new(x, y, z)
end

-- `sub_80DD8D2`: its hit lasts two ticks; it lies there 30.
local function lie(me: Object)
    if me.timer == LIE - 2 then
        me.collision:set_region(nil)
    end
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.timer, me.timer2 = BLINK, 0
    me.action = 0xC
end

-- `sub_80DD8F0`: blink out.
local function blink(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me:set_lifecycle("destroy")
        return
    end
    me.visible = bit32.btest(t, 2)
end

local ACTIONS: { [number]: (Object) -> () } = { [0] = start, [4] = fly, [8] = lie, [0xC] = blink }

-- `sub_80DD7FC`: resolve last tick's hits (a hit ends its region); at the
-- battle's end, gone; else its action, the sprite, present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if battle.over() then
        c:set_region(nil)
        me.lifecycle = "destroy"
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("AntiDmg's shuriken: action %#x reads past off_80DD838", me.action))
    end
    action(me)
    me:update_sprite()
    c:present()
end

-- `sub_80DD784`: unless the battle is over or its panel is off the field:
-- its flight from its thrower, high up, to its panel; its collision on no
-- panel yet. On at once.
local function init(me: Object)
    if battle.over() or not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    local s = me.state :: State
    local thrower = me.related1 :: Object
    s.from_x, s.from_y = thrower.pos.x, thrower.pos.y
    s.from_z = me.pos.z
    s.to_x, s.to_y = field.center(me.panel_x, me.panel_y)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, s.hit_mod)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

shuriken.kind = define.kind {
    id = "bn6:antidmg/shuriken",
    pool = "attack",
    state = {
        from_x = "i32",
        from_y = "i32",
        from_z = "i32",
        to_x = "i32",
        to_y = "i32",
        hit_mod = "u8",
    },
    -- `sub_80DD764`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80DD91A`: a shuriken onto (x, y) from `thrower` with `element`,
-- `hit_mod` and the damage word, of the thrower's side and facing, its
-- flip `flip` (the way it flies). Its position is the spawner's registers
-- (the panel's Y, the element, and 160 pixels up, the height it starts
-- from).
function shuriken.spawn(thrower: Object, x: number, y: number, element: number, hit_mod: number, damage: number, flip: number): Object?
    local o = battle.spawn(shuriken.kind, Vec3.new(y, element, 0xA0_0000))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance = thrower.alliance
    o.related1 = thrower
    o.flip = flip
    ;(o.state :: State).hit_mod = hit_mod
    return o
end

return shuriken
