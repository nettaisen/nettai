--!strict
-- Django, the navi of EXE5's Django chips (EXE5's actor object #0x24, 0x080BF214; EXE5's own: the US EXE6's Django
-- slot is empty, and EXE6's Japanese Django rides a bike), brought by the navi chip controller. He appears and
-- shuts the first enemy navi ahead in his row in his coffin (./coffin; the navi vanishes), raises his sun
-- (./sun), which lands with rubble round the coffin (./rubble) and a shaking camera, and purifies it: for as many
-- ticks as the chip's damage, a hit over the coffin's 3x3 each tick (1 damage, 2 with Otenko on his side; the
-- first one bugging), under his lights (./lights). Then the navi is let out, and he goes. Without a target, or
-- standing off a solid panel, he goes after appearing.
--
-- Waiting on the engine: the first hit's bugs (0xFD with 0x11, DS's 0xFC) are EXE5's own codes, which EXE5's bug
-- dispatch (0x0801103E) turns into an HP drain on a dark MegaMan (0xFD) or a light one (0xFC, his light/dark
-- value 500 or more); the engine reads EXE6's dispatch (`sub_80139F6`), which has neither.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local navi_chips = require("@exelib/navi_chips/navi")
local coffin = require("./coffin")
local sun = require("./sun")
local lights = require("./lights")
local rubble = require("./rubble")
local otenko = require("../otenko/identity")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- His chip's palette, and the bug its first purifying hit gives, with its argument (the original's by the chip's
    -- level, its Param1: DjangoDS's 4 the dark palette, 0x080BF290, and bug).
    palette: number,
    bug: number,
    bug_arg: number,
    -- Otenko blesses his side (+5): the drain doubles, the sun and lights brighten.
    blessed: boolean,
    -- How long the drain lasts (+0x70: the chip's damage).
    drain_ticks: number,
    -- His sun (+0x60), lights (+0x64), coffin (+0x68) and its prisoner (+0x6C).
    sun: Object?,
    lights: Object?,
    coffin: Object?,
    prisoner: Object?,
}

local django = {}

local SPRITE = asset.sprite("django")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_CHARGE = asset.sound("django-charge")
local SOUND_LAND = asset.sound("rockfall")
local SOUND_BEAM = asset.sound("sun-beam")

-- What he looks for ahead in his row (0x080BF594): an enemy navi's body.
local ENEMY_NAVI = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
-- Where the rubble falls (0x080BF608): beside the coffin's panel.
local RUBBLE_AT = { { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } }
-- The purifying hits: element 5 (the sun's), on the coffin's 3x3, no spark; the first bugs its chip's.
local SUN_ELEMENT = 5
local SOLID = 0x10

-- 0x080BF542: the panel of the first enemy navi ahead in his row; x 0 when none is before the edge.
local function find_target(me: Object): (number, number)
    local x, y = me.panel_x + me.facing, me.panel_y
    local enemy = ENEMY_NAVI[me.alliance]
    while field.valid(x, y) do
        if bit32.band(field.flags(x, y), enemy) ~= 0 then
            return x, y
        end
        x += me.facing
    end
    return 0, y
end

-- 0x080BF59C: Otenko blesses his side (its statue in the side's field object slot for class 1).
local function blessed(me: Object): boolean
    for _, o in field.objects(me.alliance) do
        if o.identity == otenko then
            return true
        end
    end
    return false
end

-- `sub_800AE54`: the first of `side`'s actors standing on (x, y).
local function actor_at(x: number, y: number, side: number): Object?
    for _, o in battle.alive_actors(side) do
        if o.panel_x == x and o.panel_y == y then
            return o
        end
    end
    return nil
end

-- 0x080E155C / 0x080E1572 (EXE6's `sub_80E14EC`, `sub_80E1502`): the prisoner vanishes into the coffin, and is
-- let out.
local function shut_in(o: Object)
    if o.actor_type == "player" then
        dimming.hide_user(o)
    else
        dimming.hide_actor(o)
    end
end
local function let_out(o: Object)
    if o.actor_type == "player" then
        dimming.show_user(o)
    else
        dimming.show_actor(o)
    end
end

-- A hit over the coffin (`sub_80C53A6`: it runs while dimmed).
local function purify(me: Object, x: number, y: number, bug: number, bug_arg: number)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = SUN_ELEMENT,
        z = 0,
        region = regions.square,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.thrown_piercing_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
        bug = bug,
        bug_arg = bug_arg,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- Count the timer down; true at 0 or below.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080BF2BC): he appears; on a solid panel, his coffin on his target and its prisoner shut in; 4 ticks.
local function appear(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer, me.timer2 = 4, 0
        if field.solid(me.panel_x, me.panel_y) then
            local tx, ty = me.future_panel_x, me.future_panel_y
            if tx ~= 0 then
                s.coffin = coffin.spawn(me, tx, ty)
                local o = actor_at(tx, ty, 1 - me.alliance)
                if o then
                    s.prisoner = o
                    shut_in(o)
                end
            end
        end
        me.phase_init = 4
    end
    if ran_out(me) then
        me:set_action(4)
    end
end

-- Action 4 (0x080BF320): 20 ticks; then the sun, or he goes without a target or a solid panel.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0x14
        me.phase_init = 4
    end
    if ran_out(me) then
        me:set_action(if me.future_panel_x ~= 0 and field.solid(me.panel_x, me.panel_y) then 8 else 0xC)
    end
end

-- Action 8, phase 0 (0x080BF37C): he raises his sun (at 4); 8 ticks.
local function raise(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 3
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == 4 then
        s.sun = sun.spawn(me, s.blessed)
    end
    me.timer = int.u16(me.timer + 1)
    if me.timer >= 8 then
        me.phase, me.phase_init = 4, 0
    end
end

-- 0x080BF5BE: the sun lands: the camera shakes, and rubble falls beside the coffin.
local function land(me: Object)
    battle.shake_camera(2, 0x1E)
    battle.play_sound(SOUND_LAND)
    local tx, ty = me.future_panel_x, me.future_panel_y
    for _, d in RUBBLE_AT do
        local x, y = field.center(tx + d[1], ty + d[2])
        rubble.spawn(me, Vec3.new(x, y, (24 * ty + 40) * 0x1_0000))
    end
end

-- Phase 4 (0x080BF3AE): the sun charges (its sound every 16 ticks), landing at 20; past 60 he puts it away.
local function charge(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 4
        me.timer = 0
        me.timer2 = 0x10
        me.phase_init = 4
    end
    if me.timer == 0x14 then
        land(me)
    end
    me.timer = int.u16(me.timer + 1)
    if me.timer > 0x3C then
        local o = s.sun
        if o then
            sun.remove(o)
        end
        me.phase, me.phase_init = 8, 0
        return
    end
    me.timer2 = int.u16(me.timer2 + 1)
    if me.timer2 >= 0x10 then
        me.timer2 = 0
        battle.play_sound(SOUND_CHARGE)
    end
end

-- Phase 8 (0x080BF3FE): 20 ticks.
local function pause(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0x14
        me.phase_init = 4
    end
    if ran_out(me) then
        me.phase, me.phase_init = 0xC, 0
    end
end

-- Phase 0xC (0x080BF420): the purification: his lights, the coffin lit, and a hit over it each tick for the
-- drain's ticks (the beam's sound every 11).
local function drain(me: Object)
    local s = me.state :: State
    local tx, ty = me.future_panel_x, me.future_panel_y
    if me.phase_init == 0 then
        me.anim = 5
        me.timer = int.u16(s.drain_ticks)
        me.timer2 = 0xB
        s.lights = lights.spawn(me, tx, ty, s.blessed)
        purify(me, tx, ty, s.bug, s.bug_arg)
        local c = s.coffin
        if c then
            coffin.set_anim(c, 2)
        end
        me.phase_init = 4
        return
    end
    if ran_out(me) then
        me.phase, me.phase_init = 0x10, 0
        return
    end
    purify(me, tx, ty, 0, 0)
    me.timer2 = int.u16(me.timer2 + 1)
    if me.timer2 >= 0xB then
        me.timer2 = 0
        battle.play_sound(SOUND_BEAM)
    end
end

-- Phase 0x10 (0x080BF4AC): the lights go, the coffin dims; 30 ticks; then the coffin goes and its prisoner is out.
local function release(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0x1E
        local l = s.lights
        if l then
            lights.remove(l)
        end
        local c = s.coffin
        if c then
            coffin.set_anim(c, 1)
        end
        me.phase_init = 4
    end
    if ran_out(me) then
        local c = s.coffin
        if c then
            coffin.remove(c)
        end
        local o = s.prisoner
        if o then
            let_out(o)
        end
        me:set_action(0xC)
    end
end

local PHASES = { [0] = raise, [4] = charge, [8] = pause, [0xC] = drain, [0x10] = release }

-- Action 8 (0x080BF356): by phase.
local function attack(me: Object)
    local phase = PHASES[me.phase]
    if not phase then
        error(string.format("Django attack phase %#x reads past its table (0x080BF368)", me.phase))
    end
    phase(me)
end

-- Action 0xC (0x080BF4F0): he goes (animation 2; his user forgotten); 4 ticks, then the controller moves on.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.related1 = nil
        me.anim = 2
        me.timer = 4
        me.phase_init = 4
    end
    if ran_out(me) then
        local controller = (me.state :: State).controller
        if controller then
            navi_chip.navi_left(controller)
        end
        me.lifecycle = "destroy"
    end
end

local ACTIONS = { [0] = appear, [4] = ready, [8] = attack, [0xC] = leave }

-- 0x080BF298.
local function act(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("Django action %#x reads past his table (0x080BF2AC)", me.action))
    end
    action(me)
end

-- 0x080BF238: his look (animation 1, loaded with the sprite update), his chip's palette, on his panel; his
-- target, the drain's ticks (his damage: 1 a hit, 2 with Otenko's blessing); and his first action at once.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 1, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = s.palette
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local tx, ty = find_target(me)
    me.future_panel_x, me.future_panel_y = tx, ty
    s.drain_ticks = bit32.band(me.damage, 0xFFF)
    me.damage = 1
    if blessed(me) then
        s.blessed = true
        me.damage = bit32.band(me.damage, 0xF000) + bit32.band(me.damage, 0xFFF) * 2
    end
    me:set_lifecycle("update")
    act(me)
end

django.kind = new.kind {
    id = "django/navi",
    pool = "actor",
    state = {
        controller = "object",
        palette = "u8",
        bug = "u8",
        bug_arg = "u8",
        blessed = "bool",
        drain_ticks = "u16",
        sun = "object",
        lights = "object",
        coffin = "object",
        prisoner = "object",
    },
    -- 0x080BF214.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- What a chip's Django is: his palette, and the bug his first purifying hit gives, with its argument.
export type Summon = { palette: number, bug: number, bug_arg: number }

-- 0x080BF51C: Django for `user`, reporting to `controller` (the summon table's entry 19), as his chip's `summon` says.
function django.summon(summon: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(django.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.palette, s.bug, s.bug_arg = summon.palette, summon.bug, summon.bug_arg
        return o
    end
end

return django
