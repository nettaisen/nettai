--!strict
-- The TwnFngs' fang (attack object #0x51, 0x080D79B8: EXE4's own): from its user's panel it slides a row down (its
-- first parameter 0) or up (1), 4 pixels a tick for 6 ticks, an attack hitting navis with hit modifier 1; then on that
-- panel (its region the panel) it turns into an attack of hit modifier 3 (its second parameter 0; else 1) and slides
-- forward 10 pixels a tick, gone at its first hit or off the screen; and gone in a puff as the battle ends. Its sprite
-- 10-27 (its shadow on the ground), its sound 0xA6.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local fang = {}

local SPRITE = asset.sprite("twin-fang")
local SOUND = asset.sound("twin-fang")
-- The puff it goes in as the battle ends (effect row 0x15).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
-- Its speeds, 16.16: across, and up or down.
local ACROSS, ALONG = 0xA_0000, 0x4_0000
local ROW_TICKS = 6
local ROW_HIT_MOD, HIT_MOD, WEAK_HIT_MOD = 1, 3, 1

local TURN, SLIDE = 0x0, 0x4

type State = {
    -- Its first parameter (+4): up a row first, not down; its second (+5): its hits weaker once it slides on.
    up: boolean,
    weak: boolean,
}

-- Off its panels, and gone at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D7AAC: down or up a row (a hit takes it off its panels); there, an attack of its hit modifier on its panel.
local function turn(me: Object)
    local c = me.collision
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if me.phase_init == 0 then
        me.timer = ROW_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        local p = me.pos
        me.pos = Vec3.new(p.x, int.i32(p.y + me.vel.y), p.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    me:set_coordinates_from_panel()
    local hit_mod = if (me.state :: State).weak then WEAK_HIT_MOD else HIT_MOD
    me:reset_collision_types(collision.attack, collision.hits_navis, hit_mod)
    c:set_region(regions.single)
    me.action = SLIDE
    me.phase, me.phase_init = 0, 0
end

-- 0x080D7B08: forward, gone at its first hit or off the screen.
local function slide(me: Object)
    if me.collision.hit_flags ~= 0 then
        finish(me)
        return
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    -- (Its X in pixels, +0x36, signed: off the screen's 320 around the middle.)
    local px = int.i16(int.lsr(me.pos.x, 16))
    if int.u32(px + 0xA0) >= 0x140 then
        finish(me)
    end
end

-- 0x080D7A64: the hits against it resolve and spark; the battle's end puffs it away; its action; present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, PUFF)
        c:set_region(nil)
        me:set_lifecycle("destroy")
        c:present()
        return
    end
    local a = me.action
    if a == TURN then
        turn(me)
    elseif a == SLIDE then
        slide(me)
    else
        error(string.format("a fang's action %#x reads past its table (0x080D7AA4)", a))
    end
    c:present()
end

-- 0x080D79DC: gone at once if the battle is over; its sprite (its shadow on the ground) on its panel, on the ground,
-- turned to its side; its speeds; an attack hitting navis (hit modifier 1, the plain spark); its sound; its first tick.
local function init(me: Object)
    if battle.over() then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    sprite:set_flip(me.alliance)
    local dy = if (me.state :: State).up then -1 else 1
    me.vel = Vec3.new(me.facing * ACROSS, dy * ALONG, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, ROW_HIT_MOD)
    me.collision:set_hit_effect(sparks.plain)
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

local kind = new.kind {
    id = "twnfng/fang",
    pool = "attack",
    state = { up = "bool", weak = "bool" },
    -- 0x080D79B8: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080D7B3A: a fang of `user`'s on (x, y), going down or up a row first, with the attack's element and damage word;
-- weak once across (its second parameter). (Its position is the spawner's registers until its init places it.)
function fang.spawn(user: Object, x: number, y: number, element: number, damage: number, up: boolean, weak: boolean)
    local o = battle.spawn(kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.up, s.weak = up, weak
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = user.alliance
        o:set_damage_word(damage)
    end
end

return fang
