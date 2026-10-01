--!strict
-- BassAnly, the navi of the BassAnly chip (the original's actor object
-- #0x50, `sub_80C3CE8`), brought by its navi chip controller. He appears
-- (with his cloak, a form overlay) on his user's panel, waits 51 ticks,
-- raises his arm, and fires four dark balls in turn, up and down the
-- slant, that home in on enemy navis (./shot). Then he goes in a
-- burst, and the controller moves on. He goes at once if his panel is off
-- the field. See docs/engine/chips.md §3.6.35.

local shot = require("./shot")
local effects = require("../../lib/effects")

export type State = {
    -- The controller waiting for him (its pointer sits in his
    -- XVelocity).
    controller: Object?,
}

local bassanly = {}

local SPRITE = asset.sprite("bass-anly")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_BASS_ANLY = asset.sound("bass-anly")

-- His cloak: a form overlay on his sprite, stepping while dimmed, on his
-- animation + 0x14 (`dword_80C3D98`).
local CLOAK: FormOverlaySpec = { sprite = SPRITE, stepping = "while_dimmed", anim_offset = 0x14 }

-- When his shots go (ticks left of his firing), each up (-1) or down (1)
-- the slant.
local SHOTS = { { 0x69, -1 }, { 0x55, 1 }, { 0x41, -1 }, { 0x2D, 1 } }
-- His shots: the fastest, seeking enemy navis along the slant.
local SHOT_LEVEL = 3

-- `sub_80C3EBC`: a shot from his panel, up or down the slant, seeking.
local function shoot(me: Object, slant: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    shot.spawn(me, me.panel_x, me.panel_y, 0, { level = SHOT_LEVEL, slant = slant, homing = "seeking" }, damage, 0)
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
-- then his firing stance, 31 ticks.
local function raise(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim = 8
    end
    if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
        return
    end
    me.anim = 0xD
    battle.play_sound(SOUND_BASS_ANLY)
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
    for _, at in SHOTS do
        if t == at[1] then
            shoot(me, at[2])
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
-- related1); then his first update.
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
    battle.play_sound(SOUND_APPEAR)
    me.timer = 0x32
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C3D0C`: a burst of smoke 16 pixels above him, his cloak
-- off, the controller moves on, and he goes.
local function leave(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.smoke)
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

bassanly.kind = define.kind {
    id = "bassanly/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80C3CE8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            leave(me)
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80C3E98`: BassAnly on the user's panel, reporting to `controller`
-- (the BassAnly chip's navi hook, `off_802CD5C[27]`). (His position is the
-- spawner's registers, all of which his init overwrites; he keeps no
-- user.)
function bassanly.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn(bassanly.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(spec.damage)
    local s = o.state :: State
    s.controller = controller
    return o
end

return bassanly
