--!strict
-- OmegaRkt's rocket (BN5's attack object #0xB9, 0x080DD4CC; BN5's own): on its side's back column, middle row, on
-- the ground (sprite 0C-51), its body hitting navis (breaking, hit modifier 3). It appears (a sound), 10 ticks on
-- launches (a sound) and flies forward 4 pixels a tick. What it hits it blows up at (the camera shaken, a white
-- flash, a sound): a one-tick hit on the 3x3 around it (its damage without the counter byte) and nine explosions
-- there. Should it fly off the screen instead, it blows up the enemy's back two columns: the camera shaken, a white
-- flash and a sound, then 2 and 5 ticks later a one-tick hit on each column (the farther first) with three
-- explosions. Its controller (./controller), which waits on it, then moves on.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@common/regions")

export type State = {
    -- The controller waiting on it (+0x60: it holds the rocket until it clears it).
    controller: Object?,
}

local rocket = {}

local SPRITE = asset.sprite("bn5:omega-rocket")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_LAUNCH = asset.sound("bn5:set-down")
local SOUND_BLAST = asset.sound("bn5:burst")
local SPEED = 0x4_0000
-- The screen's edge: past 155 pixels either side.
local EDGE = 0x9B
-- The 3x3 around it, x toward its front (0x080DD714).
local AROUND = { { -1, -1 }, { -1, 0 }, { -1, 1 }, { 0, -1 }, { 0, 0 }, { 0, 1 }, { 1, -1 }, { 1, 0 }, { 1, 1 } }

-- Its controller is done waiting on it.
local function release(me: Object)
    local c = (me.state :: State).controller
    if c then
        (c.state :: any).rocket = nil
    end
end

-- The blast's shake, flash and sound.
local function boom()
    battle.shake_camera(3, 0x2D)
    battle.play_sound(SOUND_BLAST)
    battle.palette_flash(0, 0x14, true)
end

-- A one-tick hit (running while dimmed) at (x, y) over `region`, with the damage word `damage`.
local function hit(me: Object, x: number, y: number, region: Region, damage: number)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = region,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = bit32.band(damage, 0xFFFF),
        stamina = bit32.rshift(damage, 16),
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- An explosion on (x, y)'s center.
local function explode(x: number, y: number)
    local cx, cy = field.center(int.u8(x), int.u8(y))
    battle.effect(Vec3.new(cx, cy, 0), effects.explosion)
end

-- 0x080DD670: a column's blast: a hit on it (a column of three from the middle row) and an explosion on each of its
-- panels.
local function blast_column(me: Object, x: number)
    hit(me, x, 2, regions.wide, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    for y = 1, 3 do
        explode(x, y)
    end
end

-- 0x080DD6AE: it blew up on what it hit: a hit on the 3x3 around it (the damage's halfword alone) and nine explosions.
local function blow_up(me: Object)
    boom()
    hit(me, me.panel_x, me.panel_y, regions.square, me.damage)
    for _, o in AROUND do
        explode(me.panel_x + o[1] * me.facing, me.panel_y + o[2])
    end
end

-- Phase 0 (0x080DD568): it appears (a sound); 10 ticks; then it launches (a sound).
local function ready(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0xA
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    battle.play_sound(SOUND_LAUNCH)
    me.phase, me.phase_init = 4, 0
end

-- Phase 4 (0x080DD5A2): it flies; past the screen's edge it is hidden, its region cleared.
local function fly(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local px = int.i16(bit32.rshift(me.pos.x, 16))
    if int.u32(px + EDGE) > EDGE * 2 then
        me.visible = false
        me.collision:set_region(nil)
        me.phase, me.phase_init = 8, 0
    end
end

-- Phase 8 (0x080DD5D6): the enemy's back columns blown up, 2 and 5 ticks after the blast; 31 ticks; then the
-- controller moves on, and it goes.
local function far_blast(me: Object)
    if me.phase_init == 0 then
        boom()
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local turn = bit32.bxor(me.alliance, me.flip)
    if me.timer == 0x1C then
        blast_column(me, 6 - 5 * turn)
    elseif me.timer == 0x19 then
        blast_column(me, 5 - 3 * turn)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        release(me)
        me:set_lifecycle("destroy")
    end
end

local PHASES = { [0] = ready, [4] = fly, [8] = far_blast }

-- 0x080DD514: the hits against it resolve; on one it blows up there, and the controller moves on; else its phase, its
-- sprite stepped, its collision again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        blow_up(me)
        c:set_region(nil)
        release(me)
        me:set_lifecycle("destroy")
        return
    end
    local phase = PHASES[me.phase]
    if not phase then
        error(string.format("OmegaRkt's rocket: phase %#x reads past its table (0x080DD55C)", me.phase))
    end
    phase(me)
    me:update_sprite()
    c:present()
end

-- 0x080DD4EC: on its panel, on the ground; its sprite (0C-51, animation 0 not yet loaded, its shadow), facing its way;
-- its collision (breaking, hitting navis, hit modifier 3); shown; its first update.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        error("OmegaRkt's rocket without a collision slot (0x080DD4FA doesn't check)")
    end
    me:setup_collision(collision.breaking, collision.hits_navis, 3)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

rocket.kind = define.kind {
    id = "bn5:omegarkt/rocket",
    pool = "attack",
    state = { controller = "object" },
    -- 0x080DD4CC.
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

-- 0x080DD64A: a rocket of `controller`'s side and facing on (x, y) with `element` and the damage word, which the
-- controller waits on; running while dimmed. (Its position is the spawner's registers: the row, the element and 0.)
function rocket.spawn(controller: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(rocket.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = controller.alliance, controller.flip
        o.related1 = controller
        local s = o.state :: State
        s.controller = controller
        o.run_while_dimmed = true
    end
    return o
end

return rocket
