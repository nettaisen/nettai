--!strict
-- Colonel, the navi of the Colonel chips and of CrossDiv (actor object
-- #0x01, `sub_80B8210`), brought by their navi chip controller. He fades
-- in on his user's panel with his cape (a form overlay of his own sprite)
-- and leaves at once if the panel isn't solid; he draws his sword (the
-- attachment 0x1F in his first related slot, which held his user) and
-- strikes: with Param3 0 (Colonel's ScreenDivide), an X of hit regions
-- (0x29) over the middle row of the first column ahead that is wholly the
-- enemy's; with Param3 set (CrossDiv), a cross (regions 0x0B and 0x01) on
-- the first panel ahead holding an enemy navi (or three columns ahead). A
-- flash, and a camera shake. Then he goes, and the controller moves on.
-- See docs/engine/chips.md §3.6.22.
--
-- The chips' script (navi chip subtype 17): `navi_chip` is his spawner,
-- `off_802CD5C[17]`.

local attachment = require("../attachment/attachment")

export type State = {
    -- The grey of his colour shader, fading in (object +0x19).
    grey: number,
    -- His cape (ExtraVars[0]).
    cape: Object?,
    -- The controller waiting for him (its pointer sits in ExtraVars[1]).
    controller: Object?,
}

local colonel = {
    state = { grey = "u8", cape = "object", controller = "object" },
}

local SPRITE = "08-12"

-- His palette by Param1 (`byte_80B8294`).
local PALETTES = { [0] = 0, 0, 0, 0, 3, 0, 0, 0 }

-- His cape (`dword_80B85C4`): his own sprite, stepping while dimmed, his
-- animation plus 0x0D, his palette.
local CAPE: FormOverlaySpec =
    { sprite = SPRITE, stepping = "while_dimmed", anim_offset = 0x0D, nudged = false, owner_palette = true }

-- His sword (`dword_80B85C8`: attachment 0x1F, animation 0x1C, animating
-- while dimmed).
local SWORD = { 0x1F, 0x1C, 1, 0 }

-- His fade-in's starting grey.
local FADE_GREY = 0x1F

-- The controller calls his spawner through r3, which his position takes
-- (his init sets it all).
local SPAWNER_ADDRESS = 0x080B84ED

-- A panel he stands on must be solid (as a word).
local SOLID = 0x10

-- The column ScreenDivide looks for (`byte_80B8494`): all three of its
-- panels the enemy's (panel flag 0x20 set for side 0, clear for side 1).
local ENEMY_COLUMN = { [0] = { 0x20, 0 }, { 0, 0x20 } }

-- What CrossDiv looks for (`byte_80B84E4`: an enemy navi's body).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- When his strike lands (the timer before its tick's count down).
local STRIKE_TICK = 0x20

-- `sub_80B84EC`: Colonel for `user`, reporting to `controller`.
function colonel.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("colonel", Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS), spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.alliance, o.flip = user.alliance, user.flip
    o.related1 = user
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80B8230`: on his user's panel, faded out (grey 0x7FFF shader),
-- with his cape.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local palette = PALETTES[me:param(1)]
    if palette == nil then
        error(string.format("Colonel's palette %d reads past byte_80B8294", me:param(1)))
    end
    sprite.palette = palette
    sprite.color_shader = 0x7FFF;
    (me.state :: State).cape = battle.form_overlay(me, CAPE)
    me:set_lifecycle("update")
end

-- Action 0 (`sub_80B82C4`): appear.
-- `sub_80B82E0`: shown; he fades in from grey 31, 2 a tick, flickering.
local function fade_in(me: Object)
    local s = me.state :: State
    me.visible = true
    if me.phase_init == 0 then
        s.grey = FADE_GREY
        battle.play_sound(0x94)
        me.timer = 0
        me.phase_init = 4
        return
    end
    local t = me.timer + 1
    me.timer = t
    if bit32.band(t, 1) ~= 0 then
        me.visible = false
    end
    local grey = s.grey - 2
    s.grey = grey
    if grey > 0 then
        return
    end
    me.visible = true
    s.grey = 0
    me.phase, me.phase_init = 4, 0
end

-- `sub_80B8338`: 30 ticks; on to the strike, or away if his panel isn't
-- solid.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 8)
end

-- `sub_80B845C`: ScreenDivide's column: the first ahead wholly the
-- enemy's. (The game searches on past the field forever if there is
-- none.)
local function enemy_column(me: Object): number
    local want = ENEMY_COLUMN[me.alliance]
    local x = me.panel_x + me.facing
    while true do
        local n = 0
        for y = 1, 3 do
            if field.check(x, y, want[1], want[2]) then
                n += 1
            end
        end
        if n >= 3 then
            return x
        end
        x += me.facing
        if x < -1 or x > 8 then
            error("Colonel's column search (sub_80B845C) runs off the field")
        end
    end
end

-- `sub_80B84A4`: CrossDiv's panel: the first ahead in his row holding an
-- enemy navi, or three columns ahead.
local function enemy_panel(me: Object): number
    local want = ENEMY_NAVI[me.alliance]
    local x = me.panel_x + me.facing
    while true do
        if bit32.band(field.flags(x, me.panel_y), want) ~= 0 then
            return x
        end
        x += me.facing
        if x == 0 or x > 6 then
            return me.panel_x + me.facing * 3
        end
        if x < 0 then
            -- Started left of column 1 facing left (never at his side's
            -- own columns): the game loops forever.
            error("CrossDiv's search (sub_80B84A4) never ends")
        end
    end
end

-- A hit region on (x, y) 16 pixels up, running while dimmed
-- (`sub_80C53A6`): `region`, no hit spark.
local function hit(me: Object, x: number, y: number, region: number)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0x10_0000,
        region = region,
        hit_effect = 0xFF,
        target = 5,
        self_type = 7,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- The column of the enemy's back row, where he doesn't strike.
local function at_back(me: Object): boolean
    return bit32.bxor(me.alliance, 1) * 5 + 1 == me.panel_x
end

-- `sub_80B8512`: ScreenDivide: an X over the middle of the enemy's first
-- column (region 0x29), its effect (0x38) a column on, a 20-tick flash.
local function screen_divide(me: Object)
    if at_back(me) then
        return
    end
    local x = enemy_column(me)
    local y = 2
    hit(me, x, y, 0x29)
    local cx, cy = field.center(x + me.facing, y)
    battle.effect(Vec3.new(cx, cy, 0), 0x38, me.alliance)
    battle.palette_flash(0, 0x14, true, false)
end

-- `sub_80B8564`: CrossDiv: a cross on the enemy's panel (regions 0x0B and
-- 0x01), its two effects (0x36, 0x37) 12 pixels up, a 40-tick flash.
local function cross_divide(me: Object)
    if at_back(me) then
        return
    end
    local x, y = enemy_panel(me), me.panel_y
    hit(me, x, y, 0x0B)
    hit(me, x, y, 0x01)
    local cx, cy = field.center(x, y)
    local pos = Vec3.new(cx, cy, 0xC_0000)
    battle.effect(pos, 0x36, me.alliance)
    battle.effect(pos, 0x37, me.alliance)
    battle.palette_flash(0, 0x28, true, false)
end

-- Action 4 (`sub_80B8376`): the strike.
-- `sub_80B8390`: 40 ticks raising his hand (animation 6), then his sword
-- out (animation 5).
local function draw(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.timer = 0x28
    end
    me.anim = 6
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.anim = 5
    me.phase, me.phase_init = 4, 0
    attachment.spawn_with(me, SWORD, "related")
end

-- `sub_80B83C0`: sound 0xC7; at 32 left the strike (and a camera shake,
-- `camera_initShakeEffect_80302a8(3, 35)`: presentation, not simulated);
-- 40 ticks.
local function strike(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(0xC7)
        me.timer = 0x28
        me.phase_init = 4
    end
    if me.timer == STRIKE_TICK then
        if me:param(3) == 0 then
            screen_divide(me)
        else
            cross_divide(me)
        end
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (`sub_80B8406`): he goes (his sword with his first related
-- slot); 5 ticks, then hidden, his cape off.
local function leave(me: Object)
    if me.phase == 0 then
        me.related1 = nil
        me.anim, me.anim_loaded = 4, 0
        me.timer = 5
        me.phase = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.visible = false
    local cape = (me.state :: State).cape
    if cape then
        cape:set_lifecycle("destroy")
    end
    me:set_lifecycle("destroy")
end

local PHASES: { [number]: { [number]: (Object) -> () } } = {
    [0] = { [0] = fade_in, [4] = ready },
    [4] = { [0] = draw, [4] = strike },
}

-- `sub_80B829C`: the action (and phase); his sprite, while dimmed too; his
-- colour shader from the grey (`sub_80B844A`).
local function update_state(me: Object)
    if me.action == 8 then
        leave(me)
    else
        local phases = PHASES[me.action]
        if not phases then
            error(string.format("Colonel action %#x reads past his table (off_80B82B4)", me.action))
        end
        local f = phases[me.phase]
        if not f then
            error(string.format("Colonel action %#x phase %#x reads past its table", me.action, me.phase))
        end
        f(me)
    end
    me:update_sprite_while_dimmed()
    me.sprite.color_shader = (me.state :: State).grey * 0x421
end

-- `sub_80B843C`: the controller moves on, and he is freed.
local function finish(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

-- `sub_80B8210`.
function colonel.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        update_state(me)
    else
        finish(me)
    end
end

return colonel
