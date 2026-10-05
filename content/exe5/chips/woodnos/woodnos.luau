--!strict
-- EXE5's WoodNos1-3 (action 0x28, 0x080ED286): EXE5's own. The navi raises its buster arm and, on the action's second
-- tick, a wooden wave (attack object #0xA6, 0x080DAFC4) leaves the panel ahead, 20 pixels up, rolling 6 pixels a
-- tick toward the other side and hitting navis for 10 (hit modifier 1); a hit ends it. Past the field's edge it
-- raises a geyser (attack object #0xA7, 0x080DB124) in every row where the other side's navi stands (the first panel
-- holding one, from the user's side's front edge on), or, with none, in its own row; each geyser rises at the far
-- edge and, 6 ticks later, hits its column's edge panel and the one before it with the chip's damage (hit modifier
-- 8), then lingers 30 ticks.

local arm_launch = require("../../lib/arm_launch")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local woodnos = {}

-- ---- The geyser ---------------------------------------------------------------------------------------------------

local GEYSER_SPRITE = asset.sprite("wood-geyser")
local GEYSER_SOUND = asset.sound("wood-geyser")
-- Its hit (0x080DB254's 0x04050401): the panel alone, the spark of row 4, an attack hitting navis; hit modifier 8.
local GEYSER_HIT_MOD = 8
local GEYSER_HIT_AT, GEYSER_TICKS, GEYSER_FADE = 6, 14, 30
-- Its actions.
local RISE, FADE = 0, 4

-- 0x080DB1B0: 6 ticks in, its two hits; at 15, it fades.
local function geyser_rise(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == GEYSER_HIT_AT then
        for _, x in { me.panel_x, me.panel_x + me.facing } do
            battle.hitbox(me, {
                panel_x = x,
                panel_y = me.panel_y,
                element = me.element,
                z = 0,
                region = regions.single,
                hit_effect = sparks.wood,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = GEYSER_HIT_MOD,
            })
        end
    end
    local t = me.timer + 1
    me.timer = t
    if t > GEYSER_TICKS then
        me.action, me.phase, me.phase_init = FADE, 0, 0
    end
end

-- 0x080DB204: 30 ticks (shown throughout: the game sets its shown flag either way), then gone.
local function geyser_fade(me: Object)
    if me.phase_init == 0 then
        me.timer = GEYSER_FADE
        me.phase_init = 4
    end
    me.visible = true
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

local function geyser_tick(me: Object)
    local a = me.action
    if a == RISE then
        geyser_rise(me)
    elseif a == FADE then
        geyser_fade(me)
    else
        error(string.format("WoodNos's geyser's action %#x reads past 0x080DB1A8", a))
    end
end

-- 0x080DB148: at the far edge's column (6 facing right, 1 left) of its row, 40 pixels past it, on the ground, turned
-- back; its look (sheet 10-21, no shadow); the sound.
local GEYSER = define.kind {
    id = "woodnos/geyser",
    pool = "attack",
    -- 0x080DB124.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            local facing = me.facing
            me.panel_x = if facing > 0 then 6 else 1
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(int.i32(p.x + facing * 40 * 0x10000), p.y, 0)
            me.flip = bit32.bxor(me.flip, 1)
            local sprite = me.sprite
            sprite:load(GEYSER_SPRITE)
            sprite.shadow = "hidden"
            me.anim, me.anim_loaded = 0, 0xFF
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.visible = true
            battle.play_sound(GEYSER_SOUND)
            me.lifecycle = "update"
            geyser_tick(me)
        elseif l == "update" then
            geyser_tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DB238 and `sub_801155A`: a geyser of `owner`'s in row `y` with the element and the damage word. (Its X is
-- what the spawn left in a register; its init places it.)
local function geyser(owner: Object, y: number, element: number, damage: number)
    local o = battle.spawn(GEYSER, Vec3.new(y, element, 0))
    if o then
        o.panel_y = y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
end

-- ---- The wave -----------------------------------------------------------------------------------------------------

export type WaveState = {
    -- The chip's damage word (+0x60): the geysers'.
    damage: number,
}

local WAVE_SPRITE = asset.sprite("wood-wave")
local WAVE_SOUND = asset.sound("wood-wave")
local WAVE_DAMAGE, WAVE_HIT_MOD = 10, 1
local WAVE_SPEED, EDGE = 0x6_0000, 150
-- The other side's navis' bodies, by WoodNos's wave's side (0x080DB104).
local TARGETS: { [number]: number } = { [0] = 0x0400_0000, [1] = 0x0800_0000 }

-- 0x080DB096: a geyser in each row where the other side's navi stands, from the user's side's front edge (column 5
-- for side 0, 2 for side 1) on; with none, in its own row.
local function geysers(me: Object)
    local targets = TARGETS[me.alliance]
    if not targets then
        error(string.format("WoodNos's wave's side %d reads past 0x080DB104", me.alliance))
    end
    local facing = me.facing
    local first = bit32.bxor(me.alliance, 1) * 3 + 2
    local damage = (me.state :: WaveState).damage
    local n = 0
    for y = 1, 3 do
        local x = first
        while field.valid(x, y) do
            if bit32.btest(field.flags(x, y), targets) then
                geyser(me, y, me.element, damage)
                n += 1
                break
            end
            x += facing
        end
    end
    if n == 0 then
        geyser(me, me.panel_y, me.element, damage)
    end
end

local function wave_gone(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080DB048: rolling; a hit ends it; past the edge, the geysers.
local function roll(me: Object)
    if me.collision.hit_flags ~= 0 then
        wave_gone(me)
        return
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if int.u32(int.asr(me.pos.x, 16) + EDGE) > 2 * EDGE then
        geysers(me)
        wave_gone(me)
    end
end

local function wave_tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local a = me.action
    if a ~= 0 then
        error(string.format("WoodNos's wave's action %#x reads past 0x080DB044", a))
    end
    roll(me)
    c:present()
end

-- 0x080DAFE8: the chip's damage kept for the geysers, its own 10; on its panel, its look (sheet 0c-54, with a
-- shadow) and collision (an attack hitting navis, hit modifier 1), the spark of its element; rolling forward; the
-- sound.
local WAVE = define.kind {
    id = "woodnos/wave",
    pool = "attack",
    state = { damage = "u32" },
    -- 0x080DAFC4.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            (me.state :: WaveState).damage = int.u32(me.damage + me.stamina * 0x10000)
            me.damage = WAVE_DAMAGE
            me:set_coordinates_from_panel()
            local sprite = me.sprite
            sprite:load(WAVE_SPRITE)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = 0, 0xFF
            if not me:create_collision() then
                error("WoodNos's wave has no collision: 0x080DAFE8 goes on through a null pointer")
            end
            me:setup_collision(collision.attack, collision.hits_navis, WAVE_HIT_MOD)
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.visible = true
            me.collision:set_hit_effect(sparks.of_element(me.element))
            me.vel = Vec3.new(me.facing * WAVE_SPEED, me.vel.y, me.vel.z)
            battle.play_sound(WAVE_SOUND)
            me.lifecycle = "update"
            wave_tick(me)
        elseif l == "update" then
            wave_tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DB07E and `sub_801155A`: a wave of `owner`'s on (x, y) with the element and the damage word, 20 pixels up.
-- (Its X and Y are the spawner's registers, the panel's Y and the element, until its init places it.)
local function wave(owner: Object, x: number, y: number, element: number, damage: number)
    local o = battle.spawn(WAVE, Vec3.new(y, element, 0x14_0000))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
end

-- 0x080ED286: the arm raised, the wave on the action's second tick (lib/arm_launch).
woodnos.action = arm_launch.action("0x080ED298", function(me: Object, damage: number)
    wave(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
end)

return woodnos
