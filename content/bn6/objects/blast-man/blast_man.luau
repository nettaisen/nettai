--!strict
-- BlastMan, the navi of the BlastMan chips (actor object #0x06,
-- `sub_80B8EA0`), brought by their navi chip controller. He appears, winds
-- up, and sends a fire blast across the field along his row and the rows
-- above and below it (objects/blast-fire), from behind his side's edge.
-- Then he leaves, and the controller moves on. See docs/engine/chips.md
-- §3.6.14.
--
-- The chips' script (navi chip subtype 12): `navi_chip` is his spawner,
-- `off_802CD5C[12]`.

local blast_fire = require("../blast-fire/blast_fire")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local blast_man = {
    state = { controller = "object" },
}

local SPRITE = "08-0C"

-- His blasts' speed (their Param1).
local BLAST_SPEED = 4

-- `sub_80B9014`: BlastMan for `user`, reporting to `controller`. (His
-- position is the spawner's registers, all of which his init overwrites.)
function blast_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("blast-man", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80B8EC4`: on his panel, on the ground.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- Enter the phase (phase 4, a word store of the timers when `word`), or
-- count down; true once the timer is 0 or below (or, `past_zero`, below
-- 0).
local function countdown(me: Object, ticks: number, word: boolean, past_zero: boolean): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return if past_zero then left < 0 else left <= 0
end

-- `sub_80B8F30`: he appears; 3 ticks.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(0x94)
        me.visible = true
    end
    if countdown(me, 3, true, false) then
        me:set_action(4)
    end
end

-- `sub_80B8F66`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E, true, false) then
        me:set_action(8)
    end
end

-- `sub_80B8F8E`: 20 ticks winding up.
local function wind_up(me: Object)
    if me.phase == 0 then
        me.anim = 0xA
    end
    if countdown(me, 0x14, false, false) then
        me:set_action(0xC)
    end
end

-- `sub_80B903A`: a fire blast along each valid row of his three, from
-- behind his side's edge toward the other.
local function blast(me: Object)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    for dy = -1, 1 do
        local y = me.panel_y + dy
        if field.valid(me.panel_x, y) then
            local x, dir = 0, 1
            if me.alliance ~= 0 then
                x, dir = 7, 0
            end
            blast_fire.spawn(me, x, y, me.element, { BLAST_SPEED, dir }, damage)
        end
    end
end

-- `sub_80B8FB6`: the blasts, sound 0x17F; 61 ticks (to -1).
local function attack(me: Object)
    if me.phase == 0 then
        me.anim = 0xB
        blast(me)
        battle.play_sound(0x17F)
    end
    if countdown(me, 0x3C, false, true) then
        me:set_action(0x10)
    end
end

-- `sub_80B8FEA`: 4 ticks (to -1), then he goes and the controller moves
-- on.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim = 4
    end
    if not countdown(me, 3, true, true) then
        return
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = wind_up, [0xC] = attack, [0x10] = leave }

-- `sub_80B8EA0`.
function blast_man.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("BlastMan action %#x reads past his table (off_80B8F1C)", me.action))
        end
        action(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return blast_man
