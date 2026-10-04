--!strict
-- BN5's CacDanc1-3 (action 0x1A's instant effect 23, 0x080E7E86): BN5's own. An unseen dancer (effect object #0x65,
-- 0x080E7E10: its side's field object of class 1 with 1 HP, NameID 0xD2, which doesn't run while the battle is dimmed)
-- drops a cactus (attack object #0xB1, 0x080DC60C) every 15 ticks, twelve in all, until it is replaced. A drop
-- (0x080E7EAC) lands on a random panel with one of the other side's navis on it, or, on a coin flip (the random
-- word's bit 5), on a random solid panel beside that one. The cactus falls from 160 pixels, hits its panel, then
-- bounces onto a panel beside it (one with the other side's navi, else any) in 10 ticks and goes in a puff; a hit, or
-- landing off solid ground or the field, ends it at once.
--
-- (In the operation battle, never in a netbattle, the cactus also marks its panel and the next with 0x080E9FA4's effect
-- object #0x83, which isn't ported.)

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local cacdanc = {}

-- ---- The cactus -----------------------------------------------------------------------------------------------------

-- Its look (`sub_8011504`'s word 0x01041030: sheet 10-30, animation 4, a ground shadow) and collision (0x0A050100: a
-- thrown thing that hits navis, hit modifier 1).
local SPRITE = asset.sprite("cactus")
local ANIM = 4
local HIT_MOD = 1
local FALL_SOUND, BOUNCE_SOUND = asset.sound("drop"), asset.sound("cactus-hop")
-- It starts 160 pixels up and falls 0.25 pixels a tick faster each tick; the bounce takes 10 ticks, falling a pixel a
-- tick faster each.
local START_Z, FALL_GRAVITY = 0xA0_0000, int.i32(0xFFFFC000)
local BOUNCE_TICKS, BOUNCE_GRAVITY = 10, int.i32(0xFFFF0000)
-- The puff it goes in (effect row 0x14).
local PUFF = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
-- The other side's navis (0x080DC860, 0x080E7F54) and the panels beside one (0x080DC868, 0x080E7F5C).
local TARGETS = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local SOLID = 0x10

-- Its actions.
local FALL, BOUNCE = 0, 4

-- Off its panels, freed next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080DC7B0: the panel it bounces to: a random one beside it (left, then right) with one of the other side's navis,
-- else a random one beside it on the field.
local function next_panel(me: Object): (number, number)
    local targets = TARGETS[me.alliance]
    local list: { number } = {}
    for _, dx in { -1, 1 } do
        local x, y = me.panel_x + dx, me.panel_y
        local f = field.flags(x, y)
        if f ~= 0 and bit32.btest(f, targets) then
            list[#list + 1] = panels.pack(x, y)
        end
    end
    if #list == 0 then
        for _, dx in { -1, 1 } do
            local x, y = me.panel_x + dx, me.panel_y
            if field.flags(x, y) ~= 0 then
                list[#list + 1] = panels.pack(x, y)
            end
        end
    end
    if #list == 0 then
        error("CacDanc's cactus: no panel beside it is on the field (0x080DC7B0 returns column 0 and a stale row)")
    end
    local p = list[battle.rng_positive() % #list + 1]
    return panels.x(p), panels.y(p)
end

-- 0x080DC6AC: the fall; on the ground, its hit on its panel, and the bounce next.
local function fall(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(FALL_SOUND)
        me.vel = Vec3.new(me.vel.x, me.vel.y, 0)
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    local z = int.i32(p.z + v.z)
    if z > 0 then
        me.pos = Vec3.new(p.x, p.y, z)
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + FALL_GRAVITY))
        return
    end
    me.collision:set_region(regions.single)
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_action(BOUNCE)
end

-- Gone in a puff where it is.
local function puff(me: Object)
    battle.effect(me.pos, PUFF)
    finish(me)
end

-- 0x080DC6F4: off solid ground, gone in a puff; else a bounce onto the next panel, and a puff there.
local function bounce(me: Object)
    if me.phase_init == 0 then
        if not field.solid(me.panel_x, me.panel_y) then
            puff(me)
            return
        end
        local x, y = next_panel(me)
        me.future_panel_x, me.future_panel_y = x, y
        local cx, cy = field.center(x, y)
        me.vel = trajectory.in_ticks(me.pos, cx, cy, 0, BOUNCE_TICKS, BOUNCE_GRAVITY)
        me.timer = BOUNCE_TICKS
        battle.play_sound(BOUNCE_SOUND)
        me.phase_init = 4
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + BOUNCE_GRAVITY))
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    puff(me)
end

-- 0x080DC66E: the hits against it resolve and spark; a hit ends it; else its action, and off the field it ends.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        finish(me)
    else
        local a = me.action
        if a == FALL then
            fall(me)
        elseif a == BOUNCE then
            bounce(me)
        else
            error(string.format("CacDanc's cactus: action %#x reads past 0x080DC6A4", a))
        end
        if me.lifecycle ~= "destroy" and not field.valid(me.panel_x, me.panel_y) then
            finish(me)
        end
    end
    c:present()
end

-- 0x080DC630: over its panel, 160 pixels up; its look and collision, the spark of its element, its palette (its first
-- parameter, the chip's, times 2); the first update at once.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, START_Z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = ANIM, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:set_region(nil)
    sprite.palette = (me.state :: CactusState).palette * 2
    me.lifecycle = "update"
    tick(me)
end

export type CactusState = {
    -- Its palette parameter (its first parameter: the chip's).
    palette: number,
}

cacdanc.cactus = define.kind {
    id = "cacdanc/cactus",
    pool = "attack",
    state = { palette = "u8" },
    -- 0x080DC60C: by its lifecycle, then a sprite step.
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

-- ---- The dancer -----------------------------------------------------------------------------------------------------

-- Its identity: a field object nothing swallows, leaves as junk or throws (NameID 0xD2), with no look.
local DANCER = define.identity {
    id = "cacdanc-dancer",
    class = "field_object",
    version = 0,
    ai_index = 1,
    scrap = false,
    throwable = false,
    look = { anim = 0, palette = 0, shadow = false },
}
local DROPS, EVERY = 12, 15

export type DancerState = {
    -- The chip's first parameter, the cactuses' palette.
    palette: number,
}

-- 0x080E7EAC: a cactus on a random panel with one of the other side's navis on it, or on a coin flip on a random
-- solid one beside that.
local function drop(me: Object)
    local list = panels.filtered(TARGETS[me.alliance], 0)
    if #list == 0 then
        return
    end
    local p = list[battle.rng_positive() % #list + 1]
    local x, y = panels.x(p), panels.y(p)
    if bit32.btest(battle.rng_positive(), 0x20) then
        local beside: { number } = {}
        for _, dx in { 1, -1 } do
            if field.check(x + dx, y, SOLID, 0) then
                beside[#beside + 1] = panels.pack(x + dx, y)
            end
        end
        if #beside ~= 0 then
            local q = beside[battle.rng_positive() % #beside + 1]
            x, y = panels.x(q), panels.y(q)
        end
    end
    -- 0x080DC79A: its position is the spawner's registers until its init places it.
    local o = battle.spawn(cacdanc.cactus, Vec3.new(y, me.element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = me.element
        o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        o.alliance, o.flip = me.alliance, me.flip
        o.related1 = me
        ;(o.state :: CactusState).palette = (me.state :: DancerState).palette
    end
end

-- Gone: out of the field-object registry, freed next update.
local function leave(me: Object)
    obstacle.unregister(me)
    me:set_lifecycle("destroy")
end

-- 0x080E7E46: replaced (no HP), gone; else every 15 ticks a cactus (the first at once), twelve.
local function update(me: Object)
    if me.hp == 0 then
        leave(me)
        return
    end
    if me.phase_init == 0 then
        me.timer, me.timer2 = 0, DROPS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.timer = EVERY
    drop(me)
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left <= 0 then
        leave(me)
    end
end

cacdanc.dancer = define.kind {
    id = "cacdanc/dancer",
    pool = "effect",
    state = { palette = "u8" },
    -- 0x080E7E10.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E7E30
            me.identity = DANCER
            me.hp, me.max_hp = 1, 1
            me:set_lifecycle("update")
            update(me)
        elseif l == "update" then
            update(me)
        else
            me:free()
        end
    end,
}

-- The chips' `instant` hook (0x080E7E86): the dancer on the user's panel, its side and facing, with the chip's
-- element and damage word and `palette` (the record's first parameter); its side's field object of class 1. (Its
-- position is the dispatcher's registers: the user's row, the element and the user's height.)
function cacdanc.instant(palette: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local o = battle.spawn(cacdanc.dancer, Vec3.new(spec.panel_y, spec.element, spec.z))
        if not o then
            return
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o:set_damage_word(spec.damage)
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        o.run_while_dimmed = false
        ;(o.state :: DancerState).palette = palette
        obstacle.register(o, user.alliance, 1)
    end
end

return cacdanc
