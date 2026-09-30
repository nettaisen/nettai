--!strict
-- BassAnly, the navi of the BassAnly chip (actor object #0x50,
-- `sub_80C3CE8`), brought by its navi chip controller. He appears (with his
-- cloak, a form overlay) on his user's panel, waits 51 ticks, raises his
-- arm, and fires four dark balls in turn, up and down the slant, that
-- home in on enemy navis (objects/bass-anly-shot). Then he goes in a
-- burst, and the controller moves on. He goes at once if his panel is off
-- the field. See docs/engine/chips.md §3.6.35.
--
-- The chip's script (navi chip subtype 27): `navi_chip` is his spawner,
-- `off_802CD5C[27]`.

local bass_anly_shot = require("../bass-anly-shot/bass_anly_shot")

export type State = {
    -- The controller waiting for him (its pointer sits in his
    -- XVelocity).
    controller: Object?,
}

local bass_anly = {
    state = { controller = "object" },
}

local SPRITE = "08-13"

-- His cloak: a form overlay on his sprite, stepping while dimmed, on his
-- animation + 0x14 (`dword_80C3D98`).
local CLOAK: FormOverlaySpec = { sprite = SPRITE, stepping = 1, anim_offset = 0x14 }

-- When his shots go (ticks left of his firing), each up (0) or down (1)
-- the slant.
local SHOTS = { { 0x69, 0 }, { 0x55, 1 }, { 0x41, 0 }, { 0x2D, 1 } }

-- `sub_80C3E98`: BassAnly on the user's panel, reporting to `controller`.
-- (His position is the spawner's registers, all of which his init
-- overwrites; he keeps no user.)
function bass_anly.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("bass-anly", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80C3EBC`: a shot from his panel, up (0) or down (1) the slant,
-- homing (Param3 2).
local function shoot(me: Object, down: number)
    local slant = bit32.band(2 * down - 1, 0xFF)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    bass_anly_shot.spawn(me, me.panel_x, me.panel_y, 0, { 3, slant, 2 }, damage, 0)
end

-- `sub_80C3DC8`: waiting, 51 ticks (to -1).
local function wait(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_action(4)
    end
end

-- `sub_80C3E00`: raising his arm (animation 8) until its last frame ends;
-- then his firing stance (sound 0xFD), 31 ticks.
local function raise(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim = 8
    end
    if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
        return
    end
    me.anim = 0xD
    battle.play_sound(0xFD)
    me.timer = 0x1E
    me.phase, me.phase_init = 4, 0
end

-- `sub_80C3E2E`: 31 ticks (to -1), then firing (animation 9), 121 ticks.
local function aim(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me.anim = 9
    me.timer = 0x78
    me.phase, me.phase_init = 8, 0
end

-- `sub_80C3E46`: the shots, at 105, 85, 65 and 45 left; then 21 ticks.
local function fire(me: Object)
    local t = me.timer
    for _, shot in SHOTS do
        if t == shot[1] then
            shoot(me, shot[2])
            break
        end
    end
    local left = t - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me.anim = 0
    me.timer = 0x14
    me.phase, me.phase_init = 0xC, 0
end

-- `sub_80C3E88`: 21 ticks (to -1), then he goes.
local function finish(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

local PHASES = { [0] = raise, [4] = aim, [8] = fire, [0xC] = finish }

-- `sub_80C3DDC`: the attack, by phase.
local function attack(me: Object)
    local f = PHASES[me.phase]
    if not f then
        error(string.format("BassAnly phase %#x reads past his table (off_80C3DF0)", me.phase))
    end
    f(me)
end

local ACTIONS = { [0] = wait, [4] = attack }

-- `sub_80C3D9C`: act; off the field, he goes.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("BassAnly action %#x reads past his table (off_80C3DC0)", me.action))
    end
    action(me)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C3D32`: on his panel, on the ground, with his cloak (kept in
-- related1); sound 0x94; then his first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.related1 = battle.form_overlay(me, CLOAK)
    battle.play_sound(0x94)
    me.timer = 0x32
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C3D0C`: a burst (effect 0x12) 16 pixels above him, his cloak
-- off, the controller moves on, and he goes.
local function leave(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), 0x12)
    -- (Without one the game writes into ROM.)
    local cloak = me.related1
    if cloak then
        cloak:set_lifecycle("destroy")
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

-- `sub_80C3CE8`.
function bass_anly.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        leave(me)
    end
    me:update_sprite_while_dimmed()
end

return bass_anly
