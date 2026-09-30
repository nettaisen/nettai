--!strict
-- EraseMan's own chip, EDeletBm (action 0x0A for AI index 4,
-- `sub_80F1056`): he raises his scythe (10 ticks), slashes an erase beam
-- (attack object #0xC3, objects/erase-beam) onto each panel ahead to the
-- field's edge, up to five, that lasts while he stays in the action (60
-- ticks), then 30 ticks of recovery. [unverified: no lab scenario reaches
-- it; docs/engine/standard-chips.md]

local erase_beam = require("../../objects/erase-beam/erase_beam")
local link = require("../190-heatpres/state")
type State = LinkChipState

local erase_delete = {}

local RAISE, SLASH, RECOVER = 0x0, 0x4, 0x8
-- The beams' parameters: the middle aim, 60 ticks, the navi's (they end
-- once he leaves action 0xA).
local BEAM = { 1, 0x3C, 1 }

-- `sub_80F1074`: animation 0x11 for 10 ticks.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = 0x11
        s.timer = 9
    elseif link.count_down(s) then
        link.go(me, SLASH)
    end
end

-- `sub_80F1148`: a beam on each panel ahead, while on the field (five at
-- most). They are spawned with the panel Y, the element and 1.0 as their
-- position (the registers), whose Z fraction (0) is all that lasts.
local function beams(me: Object)
    for n = 1, 5 do
        local x, y = me.panel_x + me.facing * n, me.panel_y
        if not field.valid(x, y) then
            return
        end
        local element = me.attack_element
        erase_beam.spawn(me, x, y, element, BEAM, link.damage_word(me), Vec3.new(y, element, 0x10000))
    end
end

-- `sub_80F10A0`: animation 0x12, the beams, sound 0xBA; 60 ticks.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0x12
        beams(me)
        battle.play_sound(0xBA)
        s.timer = 0x3C
    elseif link.count_down(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F10CE`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = 0x1E
    elseif link.count_down(s) then
        me.anim = 0
        me:exit_attack()
    end
end

-- `sub_80F1056` (`off_80F1068`).
function erase_delete.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == SLASH then
        slash(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("EDeletBm phase %#x reads past off_80F1068", step))
    end
end

return erase_delete
