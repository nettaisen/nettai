--!strict
-- Django, the navi of the Django chips (the original's actor object
-- #0x12). The US games have no routine for him: their navi chip table's
-- entry 19 (`off_802CD5C[19]`) is null and the actor slot a placeholder.
-- This is the Japanese games' code (EXE6 Falzar 0x080BD3B8, EXE6 Gregar
-- 0x080BEC18; the two are the same but for where things are), brought by
-- the navi chip controller. On his bike he drops in from behind his side's
-- edge onto its back column, then rides along his row across the field,
-- hitting each column's panel as he enters it, until he rides off the far
-- edge (or crashes into a panel that isn't there). If his user has put in
-- L, L, L, A meanwhile, he comes back on foot in front of the nearest
-- enemy and slashes it. See docs/engine/chips.md §3.6.38.
--
-- (CrosOver's Django, chips/crosover/navi, is another kind: the US games'
-- code, which these share nothing with but the sprite.)
--
-- Each chip's `navi` hook is `django.summon { slash_damage = n }` (EXE6
-- Falzar 0x080BD6A2): the slash's damage is the chip's parameter byte (50,
-- 80, 120); the ride's hits deal the chip's damage.

local collision = require("../../rules/collision")
local attachment = require("../../objects/attachment/attachment")
local effects = require("../../lib/effects")
local panels = require("@common/panels")
local regions = require("@common/regions")
local trajectory = require("../../lib/trajectory")
local navi_chips = require("../../lib/navi-chips/navi")

local django = {}

-- What a chip summons him with.
export type Summon = {
    -- The slash's damage (the chip's first parameter byte, his Param1).
    slash_damage: number,
}

export type State = {
    -- The controller waiting for him (its flag's pointer sits in his
    -- collision data slot).
    controller: Object?,
    slash_damage: number,
    -- His bike (the original keeps it in RelatedObject2).
    held: Object?,
    -- The command: how much of it is in (ExtraVars[0]), the ticks left to
    -- finish it (ExtraVars[1]), and that it is in (Param2).
    presses: number,
    press_ticks: number,
    command: boolean,
    -- He found someone to slash (Param3).
    target: boolean,
}

local SPRITE = asset.sprite("bn6:django")
local SOUND_APPEAR, SOUND_BIKE = asset.sound("bn6:appear"), asset.sound("bn6:django-bike")
local SOUND_COMMAND, SOUND_SWING = asset.sound("bn6:ok-8b"), asset.sound("bn6:sword-swing")

-- The controller calls his spawner through r3, which his position takes:
-- EXE6 Falzar's address with the Thumb bit (EXE6 Gregar's is 0x080BEF03),
-- whose fraction his Z keeps throughout.
local SPAWNER_ADDRESS = 0x080BD6A3

-- His bike: his own sprite's animation 7 (the Japanese games' attachment
-- row 0x0C, raised a pixel), going on while dimmed.
local BIKE = attachment.look { sprite = SPRITE, palette = 0, lift = 1 }
local BIKE_ANIM = 7

-- Where he starts, by side (not by facing): the column behind the edge,
-- 60 pixels up. The drop onto the back column: 10 ticks, falling by 0.375
-- pixels a tick (16.16).
local START_COLUMN = { [0] = 0, [1] = 7 }
local START_HEIGHT = 0x3C
local DROP_TICKS = 0xA
local GRAVITY = -0x6000

-- The ride: 5 pixels a tick, for the field's width (six 40-pixel
-- columns: 48 ticks).
local RIDE_SPEED = 0x5_0000
local RIDE_TICKS = 0x30

-- The smoke where he crashes, 16 pixels up.
local SMOKE_HEIGHT = 0x10_0000

-- The hits: the ride's on his panel (a breaking attack), the slash's on
-- the panel ahead (an attack); no spark; hit modifier 3.
local HIT_MOD = 3

-- The command, a key at a time (`word_80BD71C`, EXE6 Falzar), with 60
-- ticks from its first to put in the rest.
local COMMAND: { [number]: Key } = { [0] = "l", [1] = "l", [2] = "l", [3] = "a" }
local COMMAND_TICKS = 0x3C

-- The slash's search, by side: an enemy's body; the panel in front of it
-- solid, with no object on it (a navi's body may be); the rows tried at
-- each column, nearest first.
local ENEMY_BODY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local STAND_ON, STAND_FORBID = 0x10, 0x0380_0000
local ROWS = { 0, -1, 1, -2, 2 }

-- His actions.
local DROP_IN, RIDE, APPEAR, STAND, SLASH, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14

-- Z's whole part (a halfword store), its fraction kept.
local function set_height(me: Object, pixels: number)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), bit32.lshift(bit32.band(pixels, 0xFFFF), 16)))
end

-- The init: behind his side's edge, 60 pixels up (Z's fraction kept), his
-- sprite without a shadow, riding (animation 6); the command not in, a
-- target assumed; his bike.
local function init(me: Object)
    local s = me.state :: State
    me.panel_x = START_COLUMN[me.alliance]
    me:set_coordinates_from_panel()
    set_height(me, START_HEIGHT)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 6
    me.anim_loaded = 6
    sprite:set_animation(6)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    s.command = false
    s.presses, s.press_ticks = 0, 0
    s.target = true
    attachment.attach(me, BIKE, "held", { anim = BIKE_ANIM, while_dimmed = true })
    me:set_lifecycle("update")
end

-- Enter an action's first phase: phase 4 with an animation and a timer;
-- `word` is the game's word store, which clears Timer2 too.
local function enter(me: Object, anim: number, ticks: number, word: boolean): boolean
    if me.phase ~= 0 then
        return false
    end
    me.phase = 4
    me.anim = anim
    me.timer = ticks
    if word then
        me.timer2 = 0
    end
    return true
end

-- Count the timer down; true at 0 or below.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- He lets go of his bike (it goes).
local function let_go_bike(me: Object)
    local s = me.state :: State
    local bike = s.held
    if bike then
        bike:set_lifecycle("destroy")
        s.held = nil
    end
end

-- He goes at once (freed, not destroyed); the controller moves on.
local function gone(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

-- A puff of smoke 16 pixels above him.
local function smoke(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + SMOKE_HEIGHT)), effects.smoke)
end

-- A hit on (x, y), landing while dimmed (`sub_80C53A6`).
local function hit(me: Object, x: number, y: number, is: CollisionType, damage: number, stamina: number)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = is,
        damage = damage,
        stamina = stamina,
        hit_mod = HIT_MOD,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- The drop onto his side's back column in his row: an arc of 10 ticks
-- (it moves on its first tick too); landed on a panel, the camera shakes
-- and he rides; on none, he crashes: his bike goes, smoke, and so does he.
local function drop_in(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local cx, cy = field.center(me.panel_x + panels.side_direction(me.alliance), me.panel_y)
        me.vel = trajectory.in_ticks(me.pos, cx, cy, 0, DROP_TICKS, GRAVITY)
        me.timer, me.timer2 = DROP_TICKS, 0
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
    me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
    me:set_panel_from_coordinates()
    if not ran_out(me) then
        return
    end
    if field.solid(me.panel_x, me.panel_y) then
        battle.shake_camera(1, 0x14)
        me:set_action(RIDE)
        return
    end
    let_go_bike(me)
    smoke(me)
    gone(me)
end

-- The command, checked each tick of the ride until it is in: the next key
-- pressed (a new press, `pressed`) moves it on, and from its first key it
-- must be finished within 60 ticks, else it starts over.
local function check_command(me: Object)
    local s = me.state :: State
    if s.command then
        return
    end
    if s.presses ~= 0 then
        local left = s.press_ticks - 1
        s.press_ticks = left
        if left <= 0 then
            s.presses, s.press_ticks = 0, 0
        end
    end
    local key = COMMAND[s.presses]
    if not (me.related1 :: Object):pressed(key) then
        return
    end
    s.presses += 1
    if s.presses >= 4 then
        s.command = true
        battle.play_sound(SOUND_COMMAND)
        return
    end
    if s.presses == 1 then
        s.press_ticks = COMMAND_TICKS
    end
end

-- The ride's end: his bike goes; with the command in, he comes back on
-- foot, else he goes.
local function end_ride(me: Object)
    let_go_bike(me)
    if (me.state :: State).command then
        me:set_action(APPEAR)
        return
    end
    gone(me)
end

-- The ride along his row, 48 ticks: entering a column's panel, a hit on
-- it (his damage word); onto a panel that isn't there he crashes (smoke)
-- and the ride ends; off the field he rides on.
local function ride(me: Object)
    check_command(me)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_BIKE)
        local v = me.vel
        me.vel = Vec3.new(int.i32(panels.side_direction(me.alliance) * RIDE_SPEED), v.y, v.z)
        me.timer, me.timer2 = RIDE_TICKS, 0
        return
    end
    local from = me.panel_x
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    if field.solid(me.panel_x, me.panel_y) then
        if me.panel_x ~= from then
            hit(me, me.panel_x, me.panel_y, collision.breaking, me.damage, me.stamina)
        end
    elseif field.valid(me.panel_x, me.panel_y) then
        smoke(me)
        return end_ride(me)
    end
    if ran_out(me) then
        end_ride(me)
    end
end

-- The slash's target (`sub_80BD782` in EXE6 Falzar's numbering): from his
-- user's column forward, the first enemy's body in the rows tried (his
-- user's row, then above, below, two above, two below) with a panel in
-- front of it he can stand on; that panel, or (0, 0) for none (also once
-- the search leaves the field).
local function find_target(me: Object): (number, number)
    local dir = panels.side_direction(me.alliance)
    local user = me.related1 :: Object
    local ux, uy = user.panel_x, user.panel_y
    local enemy = ENEMY_BODY[me.alliance]
    for i = 0, 5 do
        local x = ux + dir * i
        for _, dy in ROWS do
            if x > 6 or x < 1 then
                return 0, 0
            end
            local y = uy + dy
            if field.check(x, y, enemy, 0) and field.check(x - dir, y, STAND_ON, STAND_FORBID) then
                return x - dir, y
            end
        end
    end
    return 0, 0
end

-- He comes back on foot in front of his target, or on his user's panel
-- with none; 3 ticks.
local function appear(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        local x, y = find_target(me)
        if x == 0 then
            s.target = false
            local user = me.related1 :: Object
            x, y = user.panel_x, user.panel_y
        end
        me.panel_x, me.panel_y = x, y
        me:set_coordinates_from_panel()
        me.anim = 1
        me.sprite.shadow = "ground"
        battle.play_sound(SOUND_APPEAR)
        me.timer, me.timer2 = 3, 0
        return
    end
    if ran_out(me) then
        me:set_action(STAND)
    end
end

-- 20 ticks; then the slash, or he goes with no target.
local function stand(me: Object)
    if enter(me, 0, 0x14, true) then
        return
    end
    if ran_out(me) then
        me:set_action(if (me.state :: State).target then SLASH else LEAVE)
    end
end

-- The slash: 30 ticks; at 20 left, the hit on the panel ahead (the chip's
-- slash damage with his damage word's flags, no hit parameter) and the
-- swing's sound.
local function slash(me: Object)
    if enter(me, 5, 0x1E, false) then
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x14 then
        local s = me.state :: State
        local damage = bit32.bor(s.slash_damage, bit32.band(me.damage, 0xF000))
        hit(me, me.panel_x + panels.side_direction(me.alliance), me.panel_y, collision.attack, damage, 0)
        battle.play_sound(SOUND_SWING)
        return
    end
    if left <= 0 then
        me:set_action(LEAVE)
    end
end

-- 3 ticks (to -1), then he goes and the controller moves on.
local function leave(me: Object)
    if enter(me, 2, 3, true) then
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    gone(me)
end

local ACTIONS: { [number]: (me: Object) -> () } = {
    [DROP_IN] = drop_in,
    [RIDE] = ride,
    [APPEAR] = appear,
    [STAND] = stand,
    [SLASH] = slash,
    [LEAVE] = leave,
}

django.kind = define.kind {
    id = "bn6:django/navi",
    pool = "actor",
    state = {
        controller = "object",
        slash_damage = "u8",
        held = "object",
        presses = "u8",
        press_ticks = "u8",
        command = "bool",
        target = "bool",
    },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("Django: action %#x reads past his table", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- A chip's `navi` hook: Django for the user, on the controller's panel's
-- row, reporting to `controller`.
function django.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(django.kind, user, spec, SPAWNER_ADDRESS)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.slash_damage = args.slash_damage
        return o
    end
end

return django
