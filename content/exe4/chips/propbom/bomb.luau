--!strict
-- The PropBoms' propeller bomb (attack object #0x8C, 0x080DEB70: EXE4's own): a field object of its user's side (class
-- 1, 0x0800B230) flying 20 pixels up from the panel ahead, with 10 HP: an own object the other side's hits reach (hit
-- modifier 1, no damage of its own, no spark), running while dimmed but holding still then. It hovers 6 ticks
-- (animation 1), then flies on (animation 0) at its speed (its second parameter, 1/16 pixels a tick, toward its
-- facing) until it leaves the screen, where it bursts over the other side's back two columns (the column at 5, or 2
-- for side 1, and the one beyond: region 0x11) with its damage word (hit modifier 3, no spark), an explosion on each
-- panel. Hit by a body, an attack or a navi (the hit flags 0x3F800000) or its HP gone, it explodes where it is with
-- the obstacle's damage sound; it goes in a puff as the battle ends. Its sprite 10-2b, its sound 0x133.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local bomb = {}

local SPRITE = asset.sprite("prop-bomb")
local SOUND = asset.sound("prop-bomb")
local BROKEN = asset.sound("damage")
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local HP = 10
local HOVER_TICKS = 5
-- What breaks it (its hit flags: bodies, attacks, navis).
local BREAKS = 0x3F800000
local HIT_MOD, BURST_HIT_MOD = 1, 3
-- Where it bursts: the other side's column 5 (side 0's bomb) or 2 (side 1's), row 2.
local BURST_ROW = 2
-- Off the screen: 140 pixels either side of the middle.
local EDGE = 0x8C

local HOVER, FLY, BURST = 0x0, 0x4, 0x8

type State = {
    -- Its speed (its second parameter: 1/16 pixels a tick).
    speed: number,
}

-- 0x080DEC80: off the panels and out of the registry, gone.
local function finish(me: Object)
    me.collision:set_region(nil)
    obstacle.unregister(me)
    me:set_lifecycle("destroy")
end

-- 0x080DECA4: 6 ticks hovering; then animation 0, flying.
local function hover(me: Object)
    if me.phase_init == 0 then
        me.timer = HOVER_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    me.anim = 0
    me.action = FLY
    me.phase, me.phase_init = 0, 0
end

-- 0x080DECC8: on at its speed; off the screen, the burst next.
local function fly(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    -- (Its X in pixels, +0x36, signed.)
    local px = int.i16(int.lsr(me.pos.x, 16))
    if int.u32(px + EDGE) >= 2 * EDGE then
        me.collision:set_region(nil)
        me.action = BURST
        me.phase, me.phase_init = 0, 0
    end
end

-- 0x080DECF6: the burst over the other side's back two columns, then gone.
local function burst(me: Object)
    local x = 5 - 3 * me.alliance
    battle.hitbox(me, {
        panel_x = x,
        panel_y = BURST_ROW,
        element = me.element,
        z = 0,
        region = regions.wide_long,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = BURST_HIT_MOD,
    })
    battle.region_effects(x, BURST_ROW, regions.wide_long, me.alliance, EXPLOSION, 0)
    obstacle.unregister(me)
    me:set_lifecycle("destroy")
end

-- 0x080DEC16: the hits against it resolve and spark; the battle's end puffs it away; broken it explodes; while not
-- dimmed, its action; present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, PUFF)
        finish(me)
        return
    end
    if bit32.btest(c.hit_flags, BREAKS) or me:take_damage(0) < 0 then
        battle.effect(me.pos, EXPLOSION)
        battle.play_sound(BROKEN)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == HOVER then
            hover(me)
        elseif a == FLY then
            fly(me)
        elseif a == BURST then
            burst(me)
        else
            error(string.format("a propeller bomb's action %#x reads past its table (0x080DEC98)", a))
        end
    end
    c:present()
end

-- 0x080DEB98: its sprite (its shadow on the ground) hovering, 10 HP, on its panel, flying at its speed toward its
-- facing, turned to its side; its collision; its sound; its first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 1, 0xFF
    me.hp, me.max_hp = HP, HP
    me:set_coordinates_from_panel()
    me.vel = Vec3.new(me.facing * (me.state :: State).speed * 0x1000, me.vel.y, me.vel.z)
    sprite:set_flip(me.alliance)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_object, collision.hit_by_other_side, HIT_MOD)
    local c = me.collision
    c.self_damage = 0
    c:set_hit_effect(nil)
    c:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

local kind = new.kind {
    id = "propbom/bomb",
    pool = "attack",
    state = { speed = "u8" },
    -- 0x080DEB70: then the sprite, but while dimmed.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if not battle.dimmed() then
            me:update_sprite()
        end
    end,
}

-- 0x080DED30: a propeller bomb of `user`'s on (x, y) at height `z`, of the attack's element and damage word, flying at
-- `speed`; running while dimmed; its user's; one of its side's field objects (class 1). (Its position is the
-- spawner's registers until its init places it: Y, the element and the height.)
function bomb.spawn(user: Object, x: number, y: number, z: number, element: number, damage: number, speed: number)
    local o = battle.spawn(kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = user.alliance
        o:set_damage_word(damage)
        o.run_while_dimmed = true
        o.related1 = user;
        (o.state :: State).speed = speed
        obstacle.register(o, user.alliance, 1)
    end
end

return bomb
