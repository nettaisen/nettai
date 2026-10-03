--!strict
-- ElecMan, the navi of the ElecMan chips (the original's actor object
-- #0x08, `sub_80B92B8`), brought by their navi chip controller. He appears
-- and calls down thunder twice, 60 ticks apart (./thunder): first on every
-- enemy navi within two panels of him (the 5x5 region), then on whatever
-- stands on the panels there (those also hit the panels around them). Then
-- he leaves, and the controller moves on. See docs/engine/chips.md
-- §3.6.16.

local navi_chips = require("../../lib/navi-chips/navi")
local panels = require("../../lib/panels")
local regions = require("../../lib/regions")
local thunder = require("./thunder")

local elecman = {}

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local SPRITE = asset.sprite("bn6:elecman")
local SOUND_APPEAR = asset.sound("bn6:appear")
local SOUND_ELEC_MAN = asset.sound("bn6:elec-man")
-- Where he looks (a panel region around him, dx along his side's
-- direction), and what for, by strike and side (`byte_80B94AC`).
local REGION = regions.big_square
local TARGETS = { [0] = { [0] = 0x0401_0000, 0x0801_0000 }, { [0] = 0x0081_0000, 0x0081_0000 } }

-- His thunderbolts, by strike: the second's also hit the panels around
-- them.
local BOLTS: { [number]: Thunderbolt } = { [0] = { while_dimmed = true }, { while_dimmed = true, spreads = true } }

-- His actions.
local APPEAR, STAND, RAISE, THUNDER, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80B92DC`: on his panel, on the ground.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
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
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 3, true, false) then
        me:set_action(STAND)
    end
end

-- `sub_80B9376`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E, true, false) then
        me:set_action(RAISE)
    end
end

-- `sub_80B939E`: 13 ticks raising his arms.
local function raise(me: Object)
    if me.phase == 0 then
        me.anim = 0x12
    end
    if countdown(me, 0xD, false, false) then
        me:set_action(THUNDER)
    end
end

-- What the region search leaves in r3, which the first thunderbolt's Z
-- takes: `_object_getPanelDataOffset`'s x - 1, or (x on the field) y - 1,
-- for the region's last panel.
local function search_register(me: Object, direction: number): number
    local r = REGION.panels :: { { number } }
    local last = r[#r]
    local x, y = me.panel_x + last[1] * direction, me.panel_y + last[2]
    local r3 = x - 1
    if bit32.band(r3, 0xFFFFFFFF) < 6 then
        r3 = y - 1
    end
    return r3
end

-- `sub_80B9458`: strike `n` (0 or 1): a thunderbolt (the strike's) on each
-- panel the region search finds, from the last found to the first.
local function strike(me: Object, n: number)
    local direction = if me.alliance == 0 then 1 else -1
    local found = panels.region(me.panel_x, me.panel_y, REGION, direction, TARGETS[n][me.alliance], 0)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    -- r3: the search's, then each spawn's (the spawned bolt's side and
    -- flip halfword, or the element if the pool was full).
    local r3 = search_register(me, direction)
    for i = #found, 1, -1 do
        local x, y = panels.x(found[i]), panels.y(found[i])
        local bolt = thunder.spawn(me, x, y, me.element, BOLTS[n], damage, bit32.band(r3, 0xFFFF))
        r3 = if bolt then bit32.bor(me.alliance, bit32.lshift(me.flip, 8)) else me.element
    end
end

-- `sub_80B93C6`: the thunder, again at 60 left; 120 ticks.
local function call_thunder(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x13
        battle.play_sound(SOUND_ELEC_MAN)
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
        me:set_action(LEAVE)
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
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = { [APPEAR] = appear, [STAND] = stand, [RAISE] = raise, [THUNDER] = call_thunder, [LEAVE] = leave }

elecman.kind = define.kind {
    id = "bn6:elecman/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80B92B8`.
    update = function(me: Object)
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
    end,
}

-- `sub_80B9432`: ElecMan for `user`, reporting to `controller` (the
-- ElecMan chips' navi hook, `off_802CD5C[3]`). (His position is the
-- spawner's registers, all of which his init overwrites.)
function elecman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(elecman.kind, user, spec)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return elecman
