--!strict
-- ElecMan, the navi of the ElecMan chips (actor object #0x08,
-- `sub_80B92B8`), brought by their navi chip controller. He appears and
-- calls down thunder twice, 60 ticks apart (objects/elec-thunder): first
-- on every enemy navi within two panels of him (the 5x5 region 0x26),
-- then on whatever stands on the panels there (Param1 2: those also hit
-- the panels around them). Then he leaves, and the controller moves on.
-- See docs/engine/chips.md §3.6.16.
--
-- The chips' script (navi chip subtype 3): `navi_chip` is his spawner,
-- `off_802CD5C[3]`.

local panels = require("../../lib/panels")
local elec_thunder = require("../elec-thunder/elec_thunder")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local elec_man = {
    state = { controller = "object" },
}

local SPRITE = "08-02"

-- Where he looks (a panel region around him, dx along his side's
-- direction), and what for, by strike and side (`byte_80B94AC`).
local REGION = 0x26
local TARGETS = { [0] = { [0] = 0x0401_0000, 0x0801_0000 }, { [0] = 0x0081_0000, 0x0081_0000 } }

-- `sub_80B9432`: ElecMan for `user`, reporting to `controller`. (His
-- position is the spawner's registers, all of which his init overwrites.)
function elec_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("elec-man", Vec3.zero, spec.params)
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

-- `sub_80B92DC`: on his panel, on the ground.
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

-- Enter the phase (phase 4; a word store of the timers when `word`), or
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

-- `sub_80B9340`: he appears; 3 ticks.
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

-- `sub_80B9376`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E, true, false) then
        me:set_action(8)
    end
end

-- `sub_80B939E`: 13 ticks raising his arms.
local function raise(me: Object)
    if me.phase == 0 then
        me.anim = 0x12
    end
    if countdown(me, 0xD, false, false) then
        me:set_action(0xC)
    end
end

-- What the region search leaves in r3, which the first thunderbolt's Z
-- takes: `_object_getPanelDataOffset`'s x - 1, or (x on the field) y - 1,
-- for the region's last panel.
local function search_register(me: Object, direction: number): number
    local r = data.regions[REGION]
    local last = r[#r]
    local x, y = me.panel_x + last[1] * direction, me.panel_y + last[2]
    local r3 = x - 1
    if bit32.band(r3, 0xFFFFFFFF) < 6 then
        r3 = y - 1
    end
    return r3
end

-- `sub_80B9458`: strike `n` (0 or 1): a thunderbolt (Param1 n + 1) on each
-- panel the region search finds, from the last found to the first.
local function strike(me: Object, n: number)
    local direction = if me.alliance == 0 then 1 else -1
    local found = panels.region(me.panel_x, me.panel_y, REGION, direction, TARGETS[n][me.alliance], 0)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    -- r3: the search's, then each spawn's (the spawned bolt's side and
    -- flip halfword, or the element if the pool was full).
    local r3 = search_register(me, direction)
    for i = #found, 1, -1 do
        local x, y = panels.unpack(found[i])
        local bolt = elec_thunder.spawn(me, x, y, me.element, { n + 1 }, damage, bit32.band(r3, 0xFFFF))
        r3 = if bolt then bit32.bor(me.alliance, bit32.lshift(me.flip, 8)) else me.element
    end
end

-- `sub_80B93C6`: the thunder (sound 0xC6), again at 60 left; 120 ticks.
local function thunder(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x13
        battle.play_sound(0xC6)
        strike(me, 0)
        me.timer = 0x78
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x3C then
        strike(me, 1)
        return
    end
    if left <= 0 then
        me:set_action(0x10)
    end
end

-- `sub_80B9408`: 4 ticks (to -1), then he goes and the controller moves
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

local ACTIONS = { [0] = appear, [4] = stand, [8] = raise, [0xC] = thunder, [0x10] = leave }

-- `sub_80B92B8`.
function elec_man.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("ElecMan action %#x reads past his table (off_80B932C)", me.action))
        end
        action(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return elec_man
